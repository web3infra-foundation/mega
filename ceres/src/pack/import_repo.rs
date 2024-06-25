use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::Receiver,
    },
};

use async_trait::async_trait;
use futures::{future::join_all, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use callisto::raw_blob;
use common::errors::MegaError;
use jupiter::{context::Context, storage::GitStorageProvider};
use mercury::{
    errors::GitError,
    internal::{
        object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree},
        pack::entry::Entry,
    },
};
use mercury::{hash::SHA1, internal::pack::encode::PackEncoder};
use venus::import_repo::{
    import_refs::{CommandType, RefCommand, Refs},
    repo::Repo,
};

use crate::{
    api_service::mono_api_service::MonoApiService, model::create_file::CreateFileInfo,
    pack::handler::PackHandler,
};

pub struct ImportRepo {
    pub context: Context,
    pub repo: Repo,
}

#[async_trait]
impl PackHandler for ImportRepo {
    async fn head_hash(&self) -> (String, Vec<Refs>) {
        let refs: Vec<Refs> = self
            .context
            .services
            .git_db_storage
            .get_ref(&self.repo)
            .await
            .unwrap();

        self.find_head_hash(refs)
    }

    async fn handle_receiver(&self, receiver: Receiver<Entry>) -> Result<(), GitError> {
        self.create_tree_not_exist().await;
        let storage = self.context.services.git_db_storage.clone();
        let mut entry_list = vec![];
        let mut join_tasks = vec![];
        for entry in receiver {
            entry_list.push(entry);
            if entry_list.len() >= 1000 {
                let stg_clone = storage.clone();
                let repo_clone = self.repo.clone();
                let handle = tokio::spawn(async move {
                    stg_clone.save_entry(&repo_clone, entry_list).await.unwrap();
                });
                join_tasks.push(handle);
                entry_list = vec![];
            }
        }
        join_all(join_tasks).await;
        storage.save_entry(&self.repo, entry_list).await.unwrap();
        Ok(())
    }

    async fn full_pack(&self) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let pack_config = &self.context.config.pack;
        let (entry_tx, entry_rx) = mpsc::channel(pack_config.channel_message_size);
        let (stream_tx, stream_rx) = mpsc::channel(pack_config.channel_message_size);

        let storage = self.context.services.git_db_storage.clone();
        let total = storage.get_obj_count_by_repo_id(&self.repo).await;
        let encoder = PackEncoder::new(total, 0, stream_tx);
        encoder.encode_async(entry_rx).await.unwrap();

        let repo = self.repo.clone();
        tokio::spawn(async move {
            let mut commit_stream = storage.get_commits_by_repo_id(&repo).await.unwrap();

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

            let mut tree_stream = storage.get_trees_by_repo_id(&repo).await.unwrap();
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

            let mut bid_stream = storage.get_blobs_by_repo_id(&repo).await.unwrap();
            let mut bids = vec![];
            while let Some(model) = bid_stream.next().await {
                match model {
                    Ok(m) => bids.push(m.blob_id),
                    Err(err) => eprintln!("Error: {:?}", err),
                }
            }

            let mut blob_handler = vec![];
            for chunk in bids.chunks(10000) {
                let stg_clone = storage.clone();
                let sender_clone = entry_tx.clone();
                let chunk_clone = chunk.to_vec();
                let handler = tokio::spawn(async move {
                    let mut blob_stream = stg_clone.get_raw_blobs(chunk_clone).await.unwrap();
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

            let tags = storage.get_tags_by_repo_id(&repo).await.unwrap();
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
            .get_commits_by_hashes(&self.repo, &want_clone)
            .await
            .unwrap()
            .into_iter()
            .map(|x| x.into())
            .collect();
        let mut traversal_list: Vec<Commit> = want_commits.clone();

        // traverse commit's all parents to find the commit that client does not have
        while let Some(temp) = traversal_list.pop() {
            for p_commit_id in temp.parent_commit_ids {
                let p_commit_id = p_commit_id.to_plain_str();

                if !have.contains(&p_commit_id) && !want_clone.contains(&p_commit_id) {
                    let parent: Commit = storage
                        .get_commit_by_hash(&self.repo, &p_commit_id)
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

        let want_tree_ids = want_commits
            .iter()
            .map(|c| c.tree_id.to_plain_str())
            .collect();
        let want_trees: HashMap<SHA1, Tree> = storage
            .get_trees_by_hashes(&self.repo, want_tree_ids)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (SHA1::from_str(&m.tree_id).unwrap(), m.into()))
            .collect();

        obj_num.fetch_add(want_commits.len(), Ordering::SeqCst);

        let have_commits = storage
            .get_commits_by_hashes(&self.repo, &have)
            .await
            .unwrap();
        let have_trees = storage
            .get_trees_by_hashes(
                &self.repo,
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
            .get_trees_by_hashes(&self.repo, hashes)
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
            .mega_storage
            .get_raw_blobs_by_hashes(hashes)
            .await
    }

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError> {
        let storage = self.context.services.git_db_storage.clone();
        match refs.command_type {
            CommandType::Create => {
                storage.save_ref(&self.repo, refs).await.unwrap();
            }
            CommandType::Delete => storage.remove_ref(&self.repo, refs).await.unwrap(),
            CommandType::Update => {
                storage
                    .update_ref(&self.repo, &refs.ref_name, &refs.new_id)
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
            .get_commit_by_hash(&self.repo, hash)
            .await
            .unwrap()
            .is_some()
    }

    async fn check_default_branch(&self) -> bool {
        let storage = self.context.services.git_db_storage.clone();
        storage.default_branch_exist(&self.repo).await.unwrap()
    }
}

impl ImportRepo {
    async fn create_tree_not_exist(&self) {
        let path = PathBuf::from(self.repo.repo_path.clone());
        let f_name = path.file_name().unwrap().to_str().unwrap();
        let api_service = MonoApiService {
            storage: self.context.services.mega_storage.clone(),
        };
        let req = CreateFileInfo {
            is_directory: true,
            name: f_name.to_owned(),
            path: path.parent().unwrap().to_str().unwrap().to_owned(),
            content: None,
        };

        if (api_service.create_monorepo_file(req).await).is_ok() {}
    }
}
