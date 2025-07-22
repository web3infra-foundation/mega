use std::{
    collections::{HashMap, HashSet},
    path::{Component, PathBuf},
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

use async_trait::async_trait;
use futures::future::join_all;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_stream::wrappers::ReceiverStream;

use callisto::{mega_mr, raw_blob, sea_orm_active_enums::ConvTypeEnum};
use common::{
    errors::MegaError,
    utils::{self, MEGA_BRANCH_NAME},
};
use jupiter::storage::Storage;
use mercury::internal::{object::ObjectTrait, pack::encode::PackEncoder};
use mercury::{
    errors::GitError,
    hash::SHA1,
    internal::{
        object::{commit::Commit, tree::Tree, types::ObjectType},
        pack::entry::Entry,
    },
};

use crate::{
    pack::PackHandler,
    protocol::import_refs::{RefCommand, Refs},
};

pub struct MonoRepo {
    pub storage: Storage,
    pub path: PathBuf,
    pub from_hash: String,
    pub to_hash: String,
}

#[async_trait]
impl PackHandler for MonoRepo {
    async fn head_hash(&self) -> (String, Vec<Refs>) {
        let storage = self.storage.mono_storage();

        let result = storage.get_refs(self.path.to_str().unwrap()).await.unwrap();

        let heads_exist = result
            .iter()
            .any(|x| x.ref_name == common::utils::MEGA_BRANCH_NAME);

        let refs = if heads_exist {
            let refs: Vec<Refs> = result.into_iter().map(|x| x.into()).collect();
            refs
        } else {
            let target_path = self.path.clone();
            let refs = storage.get_ref("/").await.unwrap().unwrap();
            let tree_hash = refs.ref_tree_hash.clone();

            let mut tree: Tree = storage
                .get_tree_by_hash(&tree_hash)
                .await
                .unwrap()
                .unwrap()
                .into();

            let commit: Commit = storage
                .get_commit_by_hash(&refs.ref_commit_hash)
                .await
                .unwrap()
                .unwrap()
                .into();

            for component in target_path.components() {
                if component != Component::RootDir {
                    let path_name = component.as_os_str().to_str().unwrap();
                    let sha1 = tree
                        .tree_items
                        .iter()
                        .find(|x| x.name == path_name)
                        .map(|x| x.id);
                    if let Some(sha1) = sha1 {
                        tree = storage
                            .get_trees_by_hashes(vec![sha1.to_string()])
                            .await
                            .unwrap()[0]
                            .clone()
                            .into();
                    } else {
                        return self.find_head_hash(vec![]);
                    }
                }
            }

            let c = Commit::new(
                commit.author,
                commit.committer,
                tree.id,
                vec![],
                &commit.message,
            );
            storage
                .save_ref(
                    self.path.to_str().unwrap(),
                    None,
                    &c.id.to_string(),
                    &c.tree_id.to_string(),
                    false,
                )
                .await
                .unwrap();
            storage.save_mega_commits(vec![c.clone()]).await.unwrap();

            vec![Refs {
                ref_name: MEGA_BRANCH_NAME.to_string(),
                ref_hash: c.id.to_string(),
                default_branch: true,
                ..Default::default()
            }]
        };
        self.find_head_hash(refs)
    }

    async fn handle_receiver(
        &self,
        mut receiver: UnboundedReceiver<Entry>,
    ) -> Result<Option<Commit>, GitError> {
        let storage = self.storage.mono_storage();
        let mut entry_list = Vec::new();
        let mut join_tasks = vec![];
        let mut current_commit_id = String::new();
        let mut current_commit = None;
        while let Some(entry) = receiver.recv().await {
            if current_commit.is_none() {
                if entry.obj_type == ObjectType::Commit {
                    current_commit_id = entry.hash.to_string();
                    let commit = Commit::from_bytes(&entry.data, entry.hash).unwrap();
                    current_commit = Some(commit);
                }
            } else {
                if entry.obj_type == ObjectType::Commit {
                    return Err(GitError::CustomError(
                        "only single commit support in each push".to_string(),
                    ));
                }
                if entry_list.len() >= 1000 {
                    let stg_clone = storage.clone();
                    let commit_id = current_commit_id.clone();
                    let handle = tokio::spawn(async move {
                        stg_clone.save_entry(&commit_id, entry_list).await.unwrap();
                    });
                    join_tasks.push(handle);
                    entry_list = vec![];
                }
            }
            entry_list.push(entry);
        }
        join_all(join_tasks).await;
        storage
            .save_entry(&current_commit_id, entry_list)
            .await
            .unwrap();
        Ok(current_commit)
    }

    // monorepo full pack should follow the shallow clone command 'git clone --depth=1'
    async fn full_pack(&self, want: Vec<String>) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        self.incremental_pack(want, Vec::new()).await
    }

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let mut want_clone = want.clone();
        let pack_config = &self.storage.config().pack;
        let storage = self.storage.mono_storage();
        let obj_num = AtomicUsize::new(0);

        let mut exist_objs = HashSet::new();

        let mut want_commits: Vec<Commit> = storage
            .get_commits_by_hashes(&want_clone)
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
                        .get_commit_by_hash(&p_commit_id)
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
            .get_trees_by_hashes(want_tree_ids)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (SHA1::from_str(&m.tree_id).unwrap(), m.into()))
            .collect();

        obj_num.fetch_add(want_commits.len(), Ordering::SeqCst);

        let have_commits = storage.get_commits_by_hashes(&have).await.unwrap();
        let have_trees = storage
            .get_trees_by_hashes(have_commits.iter().map(|x| x.tree.clone()).collect())
            .await
            .unwrap();
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
            .storage
            .mono_storage()
            .get_trees_by_hashes(hashes)
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
        self.storage
            .raw_db_storage()
            .get_raw_blobs_by_hashes(hashes)
            .await
    }

    async fn update_refs(&self, commit: Option<Commit>, refs: &RefCommand) -> Result<(), GitError> {
        let storage = self.storage.mono_storage();

        if let Some(c) = commit {
            let mr_link = self.handle_mr(&c.format_message()).await.unwrap();
            let ref_name = utils::mr_ref_name(&mr_link);
            if let Some(mut mr_ref) = storage.get_mr_ref(&ref_name).await.unwrap() {
                mr_ref.ref_commit_hash = refs.new_id.clone();
                mr_ref.ref_tree_hash = c.tree_id.to_string();
                storage.update_ref(mr_ref).await.unwrap();
            } else {
                storage
                    .save_ref(
                        self.path.to_str().unwrap(),
                        Some(ref_name),
                        &refs.new_id,
                        &c.tree_id.to_string(),
                        true,
                    )
                    .await
                    .unwrap();
            }
        }
        Ok(())
    }

    async fn check_commit_exist(&self, hash: &str) -> bool {
        self.storage
            .mono_storage()
            .get_commit_by_hash(hash)
            .await
            .unwrap()
            .is_some()
    }

    async fn check_default_branch(&self) -> bool {
        true
    }
}

impl MonoRepo {
    async fn handle_mr(&self, title: &str) -> Result<String, MegaError> {
        let storage = self.storage.mr_storage();
        let path_str = self.path.to_str().unwrap();

        match storage.get_open_mr_by_path(path_str).await.unwrap() {
            Some(mr) => {
                let link = mr.link.clone();
                self.handle_existing_mr(mr).await?;
                Ok(link)
            }
            None => {
                if self.from_hash == "0".repeat(40) {
                    return Err(MegaError::with_message(
                        "Can not init directory under monorepo directory!",
                    ));
                }
                let link = storage
                    .new_mr(path_str, title, &self.from_hash, &self.to_hash)
                    .await
                    .unwrap();
                Ok(link)
            }
        }
    }

    async fn handle_existing_mr(&self, mr: mega_mr::Model) -> Result<(), MegaError> {
        let mr_stg = self.storage.mr_storage();
        let comment_stg = self.storage.conversation_storage();
        if mr.from_hash == self.from_hash {
            if mr.to_hash != self.to_hash {
                comment_stg
                    .add_conversation(
                        &mr.link,
                        "",
                        Some(format!(
                            "Mega updated the mr automatic from {} to {}",
                            &mr.to_hash[..6],
                            &self.to_hash[..6]
                        )),
                        ConvTypeEnum::ForcePush,
                    )
                    .await?;
                mr_stg.update_mr_to_hash(mr, &self.to_hash).await?;
            } else {
                tracing::info!("repeat commit with mr: {}, do nothing", mr.id);
            }
        } else {
            comment_stg
                .add_conversation(
                    &mr.link,
                    "",
                    Some("Mega closed MR due to conflict".to_string()),
                    ConvTypeEnum::Closed,
                )
                .await
                .unwrap();
            mr_stg.close_mr(mr).await?;
        }
        Ok(())
    }
}
