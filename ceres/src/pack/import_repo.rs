use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use callisto::raw_blob;
use common::errors::MegaError;
use jupiter::{
    context::Context,
    storage::{batch_query_by_columns, GitStorageProvider},
};
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

use crate::pack::handler::PackHandler;

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

    async fn unpack(&self, pack_file: Bytes) -> Result<(), GitError> {
        let receiver = self
            .pack_decoder(&self.context.config.pack, pack_file)
            .unwrap();

        let storage = self.context.services.git_db_storage.clone();
        let mut entry_list = Vec::new();
        for entry in receiver {
            entry_list.push(entry);
            if entry_list.len() >= 1000 {
                storage.save_entry(&self.repo, entry_list).await.unwrap();
                entry_list = Vec::new();
            }
        }
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
            let commits = storage.get_commits_by_repo_id(&repo).await.unwrap();
            for m in commits.into_iter() {
                let c: Commit = m.into();
                let entry: Entry = c.into();
                entry_tx.send(entry).await.unwrap();
            }

            let trees: Vec<callisto::git_tree::Model> =
                storage.get_trees_by_repo_id(&repo).await.unwrap();
            for m in trees.into_iter() {
                let c: Tree = m.into();
                let entry: Entry = c.into();
                entry_tx.send(entry).await.unwrap();
            }

            let bids: Vec<String> = storage
                .get_blobs_by_repo_id(&repo)
                .await
                .unwrap()
                .into_iter()
                .map(|b| b.blob_id)
                .collect();
            let raw_blobs = batch_query_by_columns::<raw_blob::Entity, raw_blob::Column>(
                storage.get_connection(),
                raw_blob::Column::Sha1,
                bids,
                None,
                None,
            )
            .await
            .unwrap();
            for m in raw_blobs {
                // todo handle storage type
                let c: Blob = m.into();
                let entry: Entry = c.into();
                entry_tx.send(entry).await.unwrap();
            }

            let tags = storage.get_tags_by_repo_id(&repo).await.unwrap();
            for m in tags.into_iter() {
                let c: Tag = m.into();
                let entry: Entry = c.into();
                entry_tx.send(entry).await.unwrap();
            }
            drop(entry_tx);
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
