use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::StreamExt;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{
    Mutex,
    mpsc::{self},
};
use tokio_stream::wrappers::ReceiverStream;

use callisto::{mega_tree, raw_blob, sea_orm_active_enums::RefTypeEnum};
use common::errors::MegaError;
use git_internal::internal::metadata::{EntryMeta, MetaAttached};
use git_internal::{
    errors::GitError,
    internal::{
        object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree},
        pack::entry::Entry,
    },
};
use git_internal::{hash::SHA1, internal::pack::encode::PackEncoder};
use jupiter::storage::{Storage, base_storage::StorageConnector};
use jupiter::utils::converter::{FromGitModel, IntoMegaModel};

use crate::{
    api_service::{ApiHandler, mono_api_service::MonoApiService},
    pack::RepoHandler,
    protocol::{
        import_refs::{CommandType, RefCommand, Refs},
        repo::Repo,
    },
};

pub struct ImportRepo {
    pub storage: Storage,
    pub repo: Repo,
    pub command_list: Vec<RefCommand>,
    pub shared: Arc<Mutex<u32>>,
}

#[async_trait]
impl RepoHandler for ImportRepo {
    fn is_monorepo(&self) -> bool {
        false
    }

    async fn refs_with_head_hash(&self) -> (String, Vec<Refs>) {
        let result = self
            .storage
            .git_db_storage()
            .get_ref(self.repo.repo_id)
            .await
            .unwrap();
        let refs: Vec<Refs> = result.into_iter().map(|x| x.into()).collect();

        self.find_head_hash(refs)
    }

    async fn post_receive_pack(&self) -> Result<(), MegaError> {
        let _guard = self.shared.lock().await;
        self.traverses_tree_and_update_filepath().await?;
        self.attach_to_monorepo_parent().await
    }

    async fn save_entry(
        &self,
        entry_list: Vec<MetaAttached<Entry, EntryMeta>>,
    ) -> Result<(), MegaError> {
        let storage = self.storage.git_db_storage();
        storage.save_entry(self.repo.repo_id, entry_list).await
    }

    async fn update_pack_id(&self, temp_pack_id: &str, pack_id: &str) -> Result<(), MegaError> {
        let storage = self.storage.git_db_storage();
        storage.update_pack_id(temp_pack_id, pack_id).await
    }

    async fn collect_commits(&self, _: &Entry) -> Result<(), GitError> {
        Ok(())
    }

    async fn full_pack(&self, _: Vec<String>) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let pack_config = &self.storage.config().pack;
        let (entry_tx, entry_rx) = mpsc::channel(pack_config.channel_message_size);
        let (stream_tx, stream_rx) = mpsc::channel(pack_config.channel_message_size);

        let storage = self.storage.git_db_storage();
        let raw_storage = self.storage.raw_db_storage();
        let total = storage.get_obj_count_by_repo_id(self.repo.repo_id).await;
        let encoder = PackEncoder::new(total, 0, stream_tx);
        encoder.encode_async(entry_rx).await.unwrap();

        let repo_id = self.repo.repo_id;
        tokio::spawn(async move {
            let mut commit_stream = storage.get_commits_by_repo_id(repo_id).await.unwrap();

            while let Some(model) = commit_stream.next().await {
                match model {
                    Ok(m) => {
                        let c: Commit = Commit::from_git_model(m);
                        let entry = MetaAttached {
                            inner: c.into(),
                            meta: EntryMeta::new(),
                        };
                        entry_tx.send(entry).await.unwrap();
                    }
                    Err(err) => eprintln!("Error: {err:?}"),
                }
            }
            tracing::info!("send commits end");

            let mut tree_stream = storage.get_trees_by_repo_id(repo_id).await.unwrap();
            while let Some(model) = tree_stream.next().await {
                match model {
                    Ok(m) => {
                        let t: Tree = Tree::from_git_model(m);
                        let entry = MetaAttached {
                            inner: t.into(),
                            meta: EntryMeta::new(),
                        };
                        entry_tx.send(entry).await.unwrap();
                    }
                    Err(err) => eprintln!("Error: {err:?}"),
                }
            }
            tracing::info!("send trees end");

            let mut bid_stream = storage.get_blobs_by_repo_id(repo_id).await.unwrap();
            let mut bids = vec![];
            while let Some(model) = bid_stream.next().await {
                match model {
                    Ok(m) => bids.push(m.blob_id),
                    Err(err) => eprintln!("Error: {err:?}"),
                }
            }

            // let mut blob_handler = vec![];
            for chunk in bids.chunks(1000) {
                let raw_storage = raw_storage.clone();
                let sender_clone = entry_tx.clone();
                let chunk_clone = chunk.to_vec();
                // let handler = tokio::spawn(async move {
                let mut blob_stream = raw_storage.get_raw_blobs_stream(chunk_clone).await.unwrap();
                while let Some(model) = blob_stream.next().await {
                    match model {
                        Ok(m) => {
                            // TODO handle storage type
                            let data = m.data.unwrap_or_default();
                            let b: Blob = Blob::from_content_bytes(data);
                            // let blob_with_data = storage.get_blobs_by_hashes(repo_id,vec![b.id.to_string()]).await?.iter().next().unwrap();
                            let blob_with_data = storage
                                .get_blobs_by_hashes(repo_id, vec![b.id.to_string()])
                                .await
                                .expect("get_blobs_by_hashes failed")
                                .into_iter()
                                .next()
                                .expect("blob metadata not found");

                            let meta_data = EntryMeta {
                                pack_id: Some(blob_with_data.pack_id.clone()),
                                pack_offset: Some(blob_with_data.pack_offset as usize),
                                file_path: Some(blob_with_data.file_path.clone()),
                                is_delta: Some(blob_with_data.is_delta_in_pack),
                            };

                            let entry = MetaAttached {
                                inner: b.into(),
                                meta: meta_data,
                            };
                            sender_clone.send(entry).await.unwrap();
                        }
                        Err(err) => eprintln!("Error: {err:?}"),
                    }
                }
                // });
                // blob_handler.push(handler);
            }
            // join_all(blob_handler).await;
            tracing::info!("send blobs end");

            let tags = storage.get_tags_by_repo_id(repo_id).await.unwrap();
            for m in tags.into_iter() {
                let c: Tag = Tag::from_git_model(m);
                let entry = MetaAttached {
                    inner: c.into(),
                    meta: EntryMeta::new(),
                };
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
        let pack_config = &self.storage.config().pack;
        let storage = self.storage.git_db_storage();
        let obj_num = AtomicUsize::new(0);

        let mut exist_objs = HashSet::new();

        let mut want_commits: Vec<Commit> = storage
            .get_commits_by_hashes(self.repo.repo_id, &want_clone)
            .await
            .unwrap()
            .into_iter()
            .map(Commit::from_git_model)
            .collect();
        let mut traversal_list: Vec<Commit> = want_commits.clone();

        // traverse commit's all parents to find the commit that client does not have
        while let Some(temp) = traversal_list.pop() {
            for p_commit_id in temp.parent_commit_ids {
                let p_commit_id = p_commit_id.to_string();

                if !have.contains(&p_commit_id) && !want_clone.contains(&p_commit_id) {
                    let parent: Commit = Commit::from_git_model(
                        storage
                            .get_commit_by_hash(self.repo.repo_id, &p_commit_id)
                            .await
                            .unwrap()
                            .unwrap(),
                    );
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
            .map(|m| (SHA1::from_str(&m.tree_id).unwrap(), Tree::from_git_model(m)))
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
            self.traverse(Tree::from_git_model(have_tree), &mut exist_objs, None)
                .await;
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
            entry_tx
                .send(MetaAttached {
                    inner: c.into(),
                    meta: EntryMeta::new(),
                })
                .await
                .unwrap();
        }
        drop(entry_tx);

        Ok(ReceiverStream::new(stream_rx))
    }

    async fn get_trees_by_hashes(&self, hashes: Vec<String>) -> Result<Vec<Tree>, MegaError> {
        Ok(self
            .storage
            .git_db_storage()
            .get_trees_by_hashes(self.repo.repo_id, hashes)
            .await
            .unwrap()
            .into_iter()
            .map(Tree::from_git_model)
            .collect())
    }

    async fn get_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<raw_blob::Model>, MegaError> {
        self.storage
            .raw_db_storage()
            .get_raw_blobs_by_hashes(hashes)
            .await
    }

    async fn get_blob_metadata_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<HashMap<String, EntryMeta>, MegaError> {
        let models = self
            .storage
            .git_db_storage()
            .get_blobs_by_hashes(self.repo.repo_id, hashes)
            .await?;

        let map = models
            .into_iter()
            .map(|blob| {
                (
                    blob.blob_id.clone(),
                    EntryMeta {
                        pack_id: Some(blob.pack_id.clone()),
                        pack_offset: Some(blob.pack_offset as usize),
                        file_path: Some(blob.file_path.clone()),
                        is_delta: Some(blob.is_delta_in_pack),
                    },
                )
            })
            .collect::<HashMap<String, EntryMeta>>();

        Ok(map)
    }

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError> {
        let storage = self.storage.git_db_storage();
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
        self.storage
            .git_db_storage()
            .get_commit_by_hash(self.repo.repo_id, hash)
            .await
            .unwrap()
            .is_some()
    }

    async fn check_default_branch(&self) -> bool {
        let storage = self.storage.git_db_storage();
        storage
            .default_branch_exist(self.repo.repo_id)
            .await
            .unwrap()
    }

    async fn traverses_tree_and_update_filepath(&self) -> Result<(), MegaError> {
        //let (current_head, refs) = self.head_hash().await;
        let (current_head, _refs) = self.refs_with_head_hash().await;
        let commit = Commit::from_git_model(
            self.storage
                .git_db_storage()
                .get_commit_by_hash(self.repo.repo_id, &current_head)
                .await?
                .unwrap(),
        );

        let root_tree = Tree::from_git_model(
            self.storage
                .git_db_storage()
                .get_tree_by_hash(self.repo.repo_id, &commit.tree_id.to_string())
                .await?
                .unwrap()
                .clone(),
        );
        self.traverses_and_update_filepath(root_tree, PathBuf::new())
            .await?;
        Ok(())
    }
}

impl ImportRepo {
    #[async_recursion]
    async fn traverses_and_update_filepath(
        &self,
        tree: Tree,
        path: PathBuf,
    ) -> Result<(), MegaError> {
        for item in tree.tree_items {
            if item.is_tree() {
                let tree = Tree::from_git_model(
                    self.storage
                        .git_db_storage()
                        .get_tree_by_hash(self.repo.repo_id, &item.id.to_string())
                        .await?
                        .unwrap()
                        .clone(),
                );

                // 递归调用
                self.traverses_and_update_filepath(tree, path.join(item.name))
                    .await?;
            } else {
                let id = item.id.to_string();
                self.storage
                    .git_db_storage()
                    .update_git_blob_filepath(&id, path.join(item.name).to_str().unwrap())
                    .await?;
            }
        }

        Ok(())
    }

    // attach import repo to monorepo parent tree
    pub(crate) async fn attach_to_monorepo_parent(&self) -> Result<(), MegaError> {
        // 1. find branch command
        let commit_id = match self
            .command_list
            .iter()
            .find(|c| c.ref_type == RefTypeEnum::Branch)
        {
            Some(cmd) => cmd.new_id.clone(),
            None => return Ok(()),
        };

        // 2. search and create tree
        let path = PathBuf::from(self.repo.repo_path.clone());
        let mono_api_service = MonoApiService {
            storage: self.storage.clone(),
        };
        let storage = self.storage.mono_storage();
        let save_trees = mono_api_service.search_and_create_tree(&path).await?;

        // 3. get root ref
        let mut root_ref = storage
            .get_main_ref("/")
            .await?
            .ok_or_else(|| MegaError::with_message("root ref not found"))?;

        // 4. get latest commit
        let latest_commit: Commit = Commit::from_git_model(
            self.storage
                .git_db_storage()
                .get_commit_by_hash(self.repo.repo_id, &commit_id)
                .await?
                .ok_or_else(|| {
                    MegaError::with_message(format!("commit {} not found", commit_id))
                })?,
        );

        // 5. generate commit
        let commit_msg = latest_commit.format_message();
        let new_commit = Commit::from_tree_id(
            save_trees
                .back()
                .ok_or_else(|| MegaError::with_message("no tree generated"))?
                .id,
            vec![SHA1::from_str(&root_ref.ref_commit_hash).unwrap()],
            &format!("\n{commit_msg}"),
        );

        // 6. batch save tree
        let save_trees: Vec<mega_tree::ActiveModel> = save_trees
            .into_iter()
            .map(|tree| {
                let model: mega_tree::Model = tree.into_mega_model(EntryMeta::new());
                model.into()
            })
            .collect();
        storage.batch_save_model(save_trees).await?;

        // 7. update ref & save commit
        root_ref.ref_commit_hash = new_commit.id.to_string();
        root_ref.ref_tree_hash = new_commit.tree_id.to_string();
        storage.update_ref(root_ref).await?;
        storage.save_mega_commits(vec![new_commit]).await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    #[test]
    pub fn test_recurse_tree() {
        let path = PathBuf::from("/third-party/crates/tokio/tokio-console");
        let ancestors: Vec<_> = path.ancestors().collect();
        for path in ancestors.into_iter() {
            println!("{path:?}");
        }
    }
}
