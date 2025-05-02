use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use futures::{future::join_all, StreamExt};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_stream::wrappers::ReceiverStream;

use callisto::{mega_tree, raw_blob, sea_orm_active_enums::RefTypeEnum};
use common::errors::MegaError;
use jupiter::{context::Context, storage::batch_save_model};
use mercury::{
    errors::GitError,
    internal::{
        object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree},
        pack::entry::Entry,
    },
};
use mercury::{hash::SHA1, internal::pack::encode::PackEncoder};

use crate::{
    api_service::{mono_api_service::MonoApiService, ApiHandler},
    pack::PackHandler,
    protocol::{
        import_refs::{CommandType, RefCommand, Refs},
        repo::Repo,
    },
};

pub struct ImportRepo {
    pub context: Context,
    pub repo: Repo,
    pub command_list: Vec<RefCommand>,
}

#[async_trait]
impl PackHandler for ImportRepo {
    async fn head_hash(&self) -> (String, Vec<Refs>) {
        let result = self
            .context
            .services
            .git_db_storage
            .get_ref(self.repo.repo_id)
            .await
            .unwrap();
        let refs: Vec<Refs> = result.into_iter().map(|x| x.into()).collect();

        self.find_head_hash(refs)
    }

    async fn handle_receiver(
        &self,
        mut receiver: UnboundedReceiver<Entry>,
    ) -> Result<Option<Commit>, GitError> {
        let storage = self.context.services.git_db_storage.clone();
        let mut entry_list = vec![];
        let mut join_tasks = vec![];
        let repo_id = self.repo.repo_id;
        while let Some(entry) = receiver.recv().await {
            entry_list.push(entry);
            if entry_list.len() >= 10000 {
                let stg_clone = storage.clone();
                let handle = tokio::spawn(async move {
                    stg_clone.save_entry(repo_id, entry_list).await.unwrap();
                });
                join_tasks.push(handle);
                entry_list = vec![];
            }
        }
        join_all(join_tasks).await;
        storage.save_entry(repo_id, entry_list).await.unwrap();
        self.attach_to_monorepo_parent().await.unwrap();
        Ok(None)
    }

    async fn full_pack(&self, _: Vec<String>) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let pack_config = &self.context.config.pack;
        let (entry_tx, entry_rx) = mpsc::channel(pack_config.channel_message_size);
        let (stream_tx, stream_rx) = mpsc::channel(pack_config.channel_message_size);

        let storage = self.context.services.git_db_storage.clone();
        let raw_storage = self.context.services.raw_db_storage.clone();
        let total = storage.get_obj_count_by_repo_id(self.repo.repo_id).await;
        let encoder = PackEncoder::new(total, 0, stream_tx);
        encoder.encode_async(entry_rx).await.unwrap();

        let repo_id = self.repo.repo_id;
        tokio::spawn(async move {
            let mut commit_stream = storage.get_commits_by_repo_id(repo_id).await.unwrap();

            while let Some(model) = commit_stream.next().await {
                match model {
                    Ok(m) => {
                        let c: Commit = m.into();
                        let entry = c.into();
                        entry_tx.send(entry).await.unwrap();
                    }
                    Err(err) => eprintln!("Error: {:?}", err),
                }
            }
            tracing::info!("send commits end");

            let mut tree_stream = storage.get_trees_by_repo_id(repo_id).await.unwrap();
            while let Some(model) = tree_stream.next().await {
                match model {
                    Ok(m) => {
                        let t: Tree = m.into();
                        let entry = t.into();
                        entry_tx.send(entry).await.unwrap();
                    }
                    Err(err) => eprintln!("Error: {:?}", err),
                }
            }
            tracing::info!("send trees end");

            let mut bid_stream = storage.get_blobs_by_repo_id(repo_id).await.unwrap();
            let mut bids = vec![];
            while let Some(model) = bid_stream.next().await {
                match model {
                    Ok(m) => bids.push(m.blob_id),
                    Err(err) => eprintln!("Error: {:?}", err),
                }
            }

            let mut blob_handler = vec![];
            for chunk in bids.chunks(10000) {
                let raw_storage = raw_storage.clone();
                let sender_clone = entry_tx.clone();
                let chunk_clone = chunk.to_vec();
                let handler = tokio::spawn(async move {
                    let mut blob_stream =
                        raw_storage.get_raw_blobs_stream(chunk_clone).await.unwrap();
                    while let Some(model) = blob_stream.next().await {
                        match model {
                            Ok(m) => {
                                // todo handle storage type
                                let b: Blob = m.into();
                                let entry: Entry = b.into();
                                sender_clone.send(entry).await.unwrap();
                            }
                            Err(err) => eprintln!("Error: {:?}", err),
                        }
                    }
                });
                blob_handler.push(handler);
            }
            join_all(blob_handler).await;
            tracing::info!("send blobs end");

            let tags = storage.get_tags_by_repo_id(repo_id).await.unwrap();
            for m in tags.into_iter() {
                let c: Tag = m.into();
                let entry: Entry = c.into();
                entry_tx.send(entry).await.unwrap();
            }
            drop(entry_tx);
            tracing::info!("sending all object end...");
        });

        Ok(ReceiverStream::new(stream_rx))
    }

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let mut want_clone = want.clone();
        let pack_config = &self.context.config.pack;
        let storage = self.context.services.git_db_storage.clone();
        let obj_num = AtomicUsize::new(0);

        let mut exist_objs = HashSet::new();

        let mut want_commits: Vec<Commit> = storage
            .get_commits_by_hashes(self.repo.repo_id, &want_clone)
            .await
            .unwrap()
            .into_iter()
            .map(|x| x.into())
            .collect();
        let mut traversal_list: Vec<Commit> = want_commits.clone();

        // traverse commit's all parents to find the commit that client does not have
        while let Some(temp) = traversal_list.pop() {
            for p_commit_id in temp.parent_commit_ids {
                let p_commit_id = p_commit_id.to_string();

                if !have.contains(&p_commit_id) && !want_clone.contains(&p_commit_id) {
                    let parent: Commit = storage
                        .get_commit_by_hash(self.repo.repo_id, &p_commit_id)
                        .await
                        .unwrap()
                        .unwrap()
                        .into();
                    want_commits.push(parent.clone());
                    want_clone.push(p_commit_id);
                    traversal_list.push(parent);
                }
            }
        }

        let want_tree_ids = want_commits.iter().map(|c| c.tree_id.to_string()).collect();
        let want_trees: HashMap<SHA1, Tree> = storage
            .get_trees_by_hashes(self.repo.repo_id, want_tree_ids)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (SHA1::from_str(&m.tree_id).unwrap(), m.into()))
            .collect();

        obj_num.fetch_add(want_commits.len(), Ordering::SeqCst);

        let have_commits = storage
            .get_commits_by_hashes(self.repo.repo_id, &have)
            .await
            .unwrap();
        let have_trees = storage
            .get_trees_by_hashes(
                self.repo.repo_id,
                have_commits.iter().map(|x| x.tree.clone()).collect(),
            )
            .await
            .unwrap();
        // traverse to get exist_objs
        for have_tree in have_trees {
            self.traverse(have_tree.into(), &mut exist_objs, None).await;
        }

        let mut counted_obj = HashSet::new();
        // traverse for get obj nums
        for c in want_commits.clone() {
            self.traverse_for_count(
                want_trees.get(&c.tree_id).unwrap().clone(),
                &exist_objs,
                &mut counted_obj,
                &obj_num,
            )
            .await;
        }
        let (entry_tx, entry_rx) = mpsc::channel(pack_config.channel_message_size);
        let (stream_tx, stream_rx) = mpsc::channel(pack_config.channel_message_size);
        let encoder = PackEncoder::new(obj_num.into_inner(), 0, stream_tx);
        encoder.encode_async(entry_rx).await.unwrap();

        for c in want_commits {
            self.traverse(
                want_trees.get(&c.tree_id).unwrap().clone(),
                &mut exist_objs,
                Some(&entry_tx),
            )
            .await;
            entry_tx.send(c.into()).await.unwrap();
        }
        drop(entry_tx);

        Ok(ReceiverStream::new(stream_rx))
    }

    async fn get_trees_by_hashes(&self, hashes: Vec<String>) -> Result<Vec<Tree>, MegaError> {
        Ok(self
            .context
            .services
            .git_db_storage
            .get_trees_by_hashes(self.repo.repo_id, hashes)
            .await
            .unwrap()
            .into_iter()
            .map(|x| x.into())
            .collect())
    }

    async fn get_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<raw_blob::Model>, MegaError> {
        self.context
            .services
            .raw_db_storage
            .get_raw_blobs_by_hashes(hashes)
            .await
    }

    async fn update_refs(&self, _: Option<Commit>, refs: &RefCommand) -> Result<(), GitError> {
        let storage = self.context.services.git_db_storage.clone();
        match refs.command_type {
            CommandType::Create => {
                storage
                    .save_ref(self.repo.repo_id, refs.clone().into())
                    .await
                    .unwrap();
            }
            CommandType::Delete => storage
                .remove_ref(self.repo.repo_id, &refs.ref_name)
                .await
                .unwrap(),
            CommandType::Update => {
                storage
                    .update_ref(self.repo.repo_id, &refs.ref_name, &refs.new_id)
                    .await
                    .unwrap();
            }
        }
        Ok(())
    }

    async fn check_commit_exist(&self, hash: &str) -> bool {
        self.context
            .services
            .git_db_storage
            .get_commit_by_hash(self.repo.repo_id, hash)
            .await
            .unwrap()
            .is_some()
    }

    async fn check_default_branch(&self) -> bool {
        let storage = self.context.services.git_db_storage.clone();
        storage
            .default_branch_exist(self.repo.repo_id)
            .await
            .unwrap()
    }
}

impl ImportRepo {
    // attach import repo to monorepo parent tree
    async fn attach_to_monorepo_parent(&self) -> Result<(), GitError> {
        let iter = self
            .command_list
            .clone()
            .into_iter()
            .find(|c| c.ref_type == RefTypeEnum::Branch);
        if iter.is_none() {
            return Ok(());
        }
        let commit_id = iter.unwrap().new_id;

        let path = PathBuf::from(self.repo.repo_path.clone());
        let mono_api_service = MonoApiService {
            context: self.context.clone(),
        };
        let storage = self.context.services.mono_storage.clone();
        let save_trees = mono_api_service.search_and_create_tree(&path).await?;

        let mut root_ref = storage.get_ref("/").await.unwrap().unwrap();
        let latest_commit: Commit = self
            .context
            .services
            .git_db_storage
            .get_commit_by_hash(self.repo.repo_id, &commit_id)
            .await
            .unwrap()
            .unwrap()
            .into();
        let commit_msg = latest_commit.format_message();
        let new_commit = Commit::from_tree_id(
            save_trees.back().unwrap().id,
            vec![SHA1::from_str(&root_ref.ref_commit_hash).unwrap()],
            &format!("\n{}", commit_msg),
        );

        let save_trees: Vec<mega_tree::ActiveModel> = save_trees
            .into_iter()
            .map(|tree| {
                let mut model: mega_tree::Model = tree.into();
                model.commit_id = new_commit.id.to_string();
                model.into()
            })
            .collect();

        batch_save_model(storage.get_connection(), save_trees)
            .await
            .unwrap();

        root_ref.ref_commit_hash = new_commit.id.to_string();
        root_ref.ref_tree_hash = new_commit.tree_id.to_string();
        storage.update_ref(root_ref).await.unwrap();
        storage.save_mega_commits(vec![new_commit]).await.unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    #[test]
    pub fn test_recurse_tree() {
        let path = PathBuf::from("/third-part/crates/tokio/tokio-console");
        let ancestors: Vec<_> = path.ancestors().collect();
        for path in ancestors.into_iter() {
            println!("{:?}", path);
        }
    }
}
