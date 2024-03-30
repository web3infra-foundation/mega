use std::{
    collections::HashSet,
    env,
    io::{Cursor, Write},
    path::PathBuf,
    sync::{atomic::{AtomicUsize, Ordering}, mpsc::{self, Receiver, Sender}},
};

use async_trait::async_trait;
use bytes::Bytes;

use callisto::{mega_tree, raw_blob};
use common::{errors::MegaError, utils::ZERO_ID};
use mercury::internal::pack::Pack;
use venus::{
    errors::GitError,
    internal::{
        object::{
            blob::Blob,
            tree::{Tree, TreeItemMode},
        },
        pack::{
            entry::Entry,
            reference::{RefCommand, Refs},
        },
    },
};

#[async_trait]
pub trait PackHandler: Send + Sync {
    async fn head_hash(&self) -> (String, Vec<Refs>);

    fn check_head_hash(&self, refs: Vec<Refs>) -> (String, Vec<Refs>) {
        let mut head_hash = ZERO_ID.to_string();
        for git_ref in refs.iter() {
            if git_ref.default_branch {
                head_hash = git_ref.ref_hash.clone();
            }
        }
        (head_hash, refs)
    }

    async fn unpack(&self, pack_file: Bytes) -> Result<(), GitError>;

    /// Asynchronously retrieves the full pack data for the specified repository path.
    /// This function collects commits and nodes from the storage and packs them into
    /// a single binary vector. There is no need to build the entire tree; the function
    /// only sends all the data related to this repository.
    ///
    /// # Returns
    /// * `Result<Vec<u8>, GitError>` - The packed binary data as a vector of bytes.
    ///
    async fn full_pack(&self) -> Result<Vec<u8>, GitError>;

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<Vec<u8>, GitError>;

    // retrieve all sub trees recursively
    async fn traverse_want_trees(
        &self,
        tree: Tree,
        exist_objs: &HashSet<String>,
        sender: Sender<Entry>,
        obj_num: &AtomicUsize,
    ) {
        let mut search_tree_ids = vec![];
        let mut seacrh_blob_ids = vec![];
        for item in &tree.tree_items {
            if !exist_objs.contains(&item.id.to_plain_str()) {
                if item.mode == TreeItemMode::Tree {
                    search_tree_ids.push(item.id.to_plain_str())
                } else {
                    seacrh_blob_ids.push(item.id.to_plain_str());
                }
            }
        }

        let blobs = self.get_blobs_by_hashes(seacrh_blob_ids).await.unwrap();
        for b in blobs {
            let blob: Blob = b.into();
            sender.send(blob.into()).unwrap();
            obj_num.fetch_add(1, Ordering::SeqCst);
        }

        let trees = self.get_trees_by_hashes(search_tree_ids).await.unwrap();
        for t in trees {
            self.traverse_want_trees(t.into(), exist_objs, sender.clone(), obj_num)
                .await;
        }
        sender.send(tree.into()).unwrap();
        obj_num.fetch_add(1, Ordering::SeqCst);
    }

    async fn add_to_exist_objs(&self, tree: Tree, exist_objs: &mut HashSet<String>) {
        let mut search_tree_ids = vec![];
        for item in &tree.tree_items {
            if !exist_objs.contains(&item.id.to_plain_str()) {
                if item.mode == TreeItemMode::Tree {
                    search_tree_ids.push(item.id.to_plain_str())
                } else {
                    exist_objs.insert(item.id.to_plain_str());
                }
            }
        }
        let trees = self.get_trees_by_hashes(search_tree_ids).await.unwrap();
        for tree in trees {
            self.add_to_exist_objs(tree.into(), exist_objs).await;
        }
        exist_objs.insert(tree.id.to_plain_str());
    }

    async fn get_trees_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<mega_tree::Model>, MegaError>;

    async fn get_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<raw_blob::Model>, MegaError>;

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError>;

    async fn check_commit_exist(&self, hash: &str) -> bool;

    async fn check_default_branch(&self) -> bool;

    fn pack_decoder(&self, pack_file: Bytes) -> Result<Receiver<Entry>, GitError> {
        #[cfg(debug_assertions)]
        {
            let datetime = chrono::Utc::now().naive_utc();
            let path = format!("{}.pack", datetime);
            let mut output = std::fs::File::create(path).unwrap();
            output.write_all(&pack_file).unwrap();
        }

        let cache_size: usize = env::var("MEGA_PACK_DECODE_MEM_SIZE")
            .unwrap()
            .parse::<usize>()
            .unwrap();

        let (sender, receiver) = mpsc::channel();
        let tmp = PathBuf::from(env::var("MEGA_PACK_DECODE_CACHE_PATH").unwrap());
        let clean_tmp: bool = env::var("CLEAN_CACHE_AFTER_DECODE")
            .unwrap()
            .parse::<bool>()
            .unwrap();
        let p = Pack::new(
            None,
            Some(1024 * 1024 * 1024 * cache_size),
            Some(tmp.clone()),
            clean_tmp,
        );
        p.decode_async(Cursor::new(pack_file), sender); //Pack moved here
        Ok(receiver)
    }
}
