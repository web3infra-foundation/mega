use std::{
    collections::{HashMap, HashSet},
    path::{Component, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::Receiver,
    },
    vec,
};

use async_trait::async_trait;
use bytes::Bytes;

use callisto::raw_blob;
use common::{errors::MegaError, utils::MEGA_BRANCH_NAME};
use jupiter::context::Context;
use mercury::internal::pack::encode::PackEncoder;
use mercury::{
    errors::GitError,
    hash::SHA1,
    internal::{
        object::{commit::Commit, tree::Tree, types::ObjectType},
        pack::entry::Entry,
    },
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use venus::{
    import_repo::import_refs::{RefCommand, Refs},
    monorepo::mr::MergeRequest,
};

use crate::pack::handler::PackHandler;

pub struct MonoRepo {
    pub context: Context,
    pub path: PathBuf,
    pub from_hash: Option<String>,
    pub to_hash: Option<String>,
}

#[async_trait]
impl PackHandler for MonoRepo {
    async fn head_hash(&self) -> (String, Vec<Refs>) {
        let storage = self.context.services.mega_storage.clone();

        let result = storage.get_ref(self.path.to_str().unwrap()).await.unwrap();
        let refs = if result.is_some() {
            vec![result.unwrap().into()]
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
                            .get_trees_by_hashes(vec![sha1.to_plain_str()])
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
                    &c.id.to_plain_str(),
                    &c.tree_id.to_plain_str(),
                )
                .await
                .unwrap();
            storage.save_mega_commits(vec![c.clone()]).await.unwrap();

            vec![Refs {
                ref_name: MEGA_BRANCH_NAME.to_string(),
                ref_hash: c.id.to_plain_str(),
                default_branch: true,
                ..Default::default()
            }]
        };
        self.find_head_hash(refs)
    }

    async fn unpack(&self, pack_file: Bytes) -> Result<(), GitError> {
        let receiver = self
            .pack_decoder(&self.context.config.pack, pack_file)
            .unwrap();

        let storage = self.context.services.mega_storage.clone();

        let (mut mr, mr_exist) = self.get_mr().await;

        let mut commit_size = 0;
        if mr_exist {
            if mr.from_hash == self.from_hash.clone().unwrap() {
                let to_hash = self.to_hash.clone().unwrap();
                if mr.to_hash != to_hash {
                    let comment = self.comment_for_force_update(&mr.to_hash, &to_hash);
                    mr.to_hash = to_hash;
                    storage
                        .add_mr_comment(mr.id, 0, Some(comment))
                        .await
                        .unwrap();
                    commit_size = self.save_entry(receiver).await;
                }
            } else {
                mr.close();
                storage
                    .add_mr_comment(mr.id, 0, Some("Mega closed MR due to conflict".to_string()))
                    .await
                    .unwrap();
            }
            storage.update_mr(mr.clone()).await.unwrap();
        } else {
            commit_size = self.save_entry(receiver).await;

            storage.save_mr(mr.clone()).await.unwrap();
        };

        if commit_size > 1 {
            mr.close();
            storage
                .add_mr_comment(
                    mr.id,
                    0,
                    Some("Mega closed MR due to multi commit detected".to_string()),
                )
                .await
                .unwrap();
        }
        Ok(())
    }

    // monorepo full pack should follow the shallow clone command 'git clone --depth=1'
    async fn full_pack(&self) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let pack_config = &self.context.config.pack;
        let storage = self.context.services.mega_storage.clone();
        let obj_num = AtomicUsize::new(0);

        let refs = storage
            .get_ref(self.path.to_str().unwrap())
            .await
            .unwrap()
            .unwrap();
        let commit: Commit = storage
            .get_commit_by_hash(&refs.ref_commit_hash)
            .await
            .unwrap()
            .unwrap()
            .into();
        let tree: Tree = storage
            .get_tree_by_hash(&refs.ref_tree_hash)
            .await
            .unwrap()
            .unwrap()
            .into();
        self.traverse_for_count(tree.clone(), &HashSet::new(), &obj_num)
            .await;

        obj_num.fetch_add(1, Ordering::SeqCst);

        let (entry_tx, entry_rx) = mpsc::channel(pack_config.channel_message_size);
        let (stream_tx, stream_rx) = mpsc::channel(pack_config.channel_message_size);

        let encoder = PackEncoder::new(obj_num.into_inner(), 0, stream_tx);
        encoder.encode_async(entry_rx).await.unwrap();
        self.traverse(tree, &mut HashSet::new(), Some(&entry_tx))
            .await;
        entry_tx.send(commit.into()).await.unwrap();
        drop(entry_tx);
        Ok(ReceiverStream::new(stream_rx))
    }

    async fn incremental_pack(
        &self,
        mut want: Vec<String>,
        have: Vec<String>,
    ) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let pack_config = &self.context.config.pack;
        let storage = self.context.services.mega_storage.clone();
        let obj_num = AtomicUsize::new(0);

        let mut exist_objs = HashSet::new();

        let mut want_commits: Vec<Commit> = storage
            .get_commits_by_hashes(&want)
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

                if !have.contains(&p_commit_id) && !want.contains(&p_commit_id) {
                    let parent: Commit = storage
                        .get_commit_by_hash(&p_commit_id)
                        .await
                        .unwrap()
                        .unwrap()
                        .into();
                    want_commits.push(parent.clone());
                    want.push(p_commit_id);
                    traversal_list.push(parent);
                }
            }
        }

        let want_tree_ids = want_commits
            .iter()
            .map(|c| c.tree_id.to_plain_str())
            .collect();
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

        // traverse for get obj nums
        for c in want_commits.clone() {
            self.traverse_for_count(
                want_trees.get(&c.tree_id).unwrap().clone(),
                &exist_objs,
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
            .mega_storage
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
        self.context
            .services
            .mega_storage
            .get_raw_blobs_by_hashes(hashes)
            .await
    }

    async fn update_refs(&self, _: &RefCommand) -> Result<(), GitError> {
        //do nothing in monorepo because we use mr to handle refs update
        Ok(())
    }

    async fn check_commit_exist(&self, hash: &str) -> bool {
        self.context
            .services
            .mega_storage
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
    async fn get_mr(&self) -> (MergeRequest, bool) {
        let storage = self.context.services.mega_storage.clone();

        let mr = storage
            .get_open_mr(self.path.to_str().unwrap())
            .await
            .unwrap();
        if let Some(mr) = mr {
            (mr, true)
        } else {
            let mr = MergeRequest {
                path: self.path.to_str().unwrap().to_owned(),
                from_hash: self.from_hash.clone().unwrap(),
                to_hash: self.to_hash.clone().unwrap(),
                ..Default::default()
            };
            (mr, false)
        }
    }

    fn comment_for_force_update(&self, from: &str, to: &str) -> String {
        format!(
            "Mega updated the mr automatic from {} to {}",
            &from[..6],
            &to[..6]
        )
    }

    async fn save_entry(&self, receiver: Receiver<Entry>) -> i32 {
        let storage = self.context.services.mega_storage.clone();
        let mut entry_list = Vec::new();

        let mut commit_size = 0;
        for entry in receiver {
            if entry.obj_type == ObjectType::Commit {
                commit_size += 1;
            }
            entry_list.push(entry);
            if entry_list.len() >= 1000 {
                storage.save_entry(entry_list).await.unwrap();
                entry_list = Vec::new();
            }
        }
        storage.save_entry(entry_list).await.unwrap();
        commit_size
    }
}
