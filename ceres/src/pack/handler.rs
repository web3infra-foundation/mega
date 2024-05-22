use std::{
    collections::HashSet,
    io::Cursor,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver},
    },
};

use async_trait::async_trait;
use bytes::Bytes;
use tokio_stream::wrappers::ReceiverStream;

use callisto::raw_blob;
use common::{config::PackConfig, errors::MegaError, utils::ZERO_ID};
use mercury::internal::pack::Pack;
use mercury::{
    errors::GitError,
    internal::{
        object::{
            blob::Blob,
            tree::{Tree, TreeItemMode},
        },
        pack::entry::Entry,
    },
};
use venus::import_repo::import_refs::{RefCommand, Refs};

#[async_trait]
pub trait PackHandler: Send + Sync {
    async fn head_hash(&self) -> (String, Vec<Refs>);

    fn find_head_hash(&self, refs: Vec<Refs>) -> (String, Vec<Refs>) {
        let mut head_hash = ZERO_ID.to_string();
        for git_ref in refs.iter() {
            if git_ref.default_branch {
                head_hash.clone_from(&git_ref.ref_hash);
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
    async fn full_pack(&self) -> Result<ReceiverStream<Vec<u8>>, GitError>;

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<ReceiverStream<Vec<u8>>, GitError>;

    async fn traverse_for_count(
        &self,
        tree: Tree,
        exist_objs: &HashSet<String>,
        obj_num: &AtomicUsize,
    ) {
        let mut search_tree_ids = vec![];
        let mut search_blob_ids = vec![];
        for item in &tree.tree_items {
            let hash = item.id.to_plain_str();
            if !exist_objs.contains(&hash) {
                if item.mode == TreeItemMode::Tree {
                    search_tree_ids.push(hash.clone())
                } else {
                    search_blob_ids.push(hash.clone());
                }
            }
        }
        obj_num.fetch_add(search_blob_ids.len(), Ordering::SeqCst);
        let trees = self.get_trees_by_hashes(search_tree_ids).await.unwrap();
        for t in trees {
            self.traverse_for_count(t, exist_objs, obj_num).await;
        }
        obj_num.fetch_add(1, Ordering::SeqCst);
    }

    async fn traverse(
        &self,
        tree: Tree,
        exist_objs: &mut HashSet<String>,
        sender: Option<&tokio::sync::mpsc::Sender<Entry>>,
    ) {
        exist_objs.insert(tree.id.to_plain_str());
        let mut search_tree_ids = vec![];
        let mut search_blob_ids = vec![];
        for item in &tree.tree_items {
            let hash = item.id.to_plain_str();
            if !exist_objs.contains(&hash) {
                if item.mode == TreeItemMode::Tree {
                    search_tree_ids.push(hash.clone())
                } else {
                    search_blob_ids.push(hash.clone());
                }
                exist_objs.insert(hash);
            }
        }

        if let Some(sender) = sender {
            let blobs = self.get_blobs_by_hashes(search_blob_ids).await.unwrap();
            for b in blobs {
                let blob: Blob = b.into();
                sender.send(blob.into()).await.unwrap();
            }
        }
        let trees = self.get_trees_by_hashes(search_tree_ids).await.unwrap();
        for t in trees {
            self.traverse(t, exist_objs, sender).await;
        }
        if let Some(sender) = sender {
            sender.send(tree.into()).await.unwrap();
        }
    }

    async fn get_trees_by_hashes(&self, hashes: Vec<String>) -> Result<Vec<Tree>, MegaError>;

    async fn get_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<raw_blob::Model>, MegaError>;

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError>;

    async fn check_commit_exist(&self, hash: &str) -> bool;

    async fn check_default_branch(&self) -> bool;

    fn pack_decoder(
        &self,
        pack_config: &PackConfig,
        pack_file: Bytes,
    ) -> Result<Receiver<Entry>, GitError> {
        // #[cfg(debug_assertions)]
        // {
        //     let datetime = chrono::Utc::now().naive_utc();
        //     let path = format!("{}.pack", datetime);
        //     let mut output = std::fs::File::create(path).unwrap();
        //     output.write_all(&pack_file).unwrap();
        // }
        let (sender, receiver) = mpsc::channel();
        let p = Pack::new(
            None,
            Some(1024 * 1024 * 1024 * pack_config.pack_decode_mem_size),
            Some(pack_config.pack_decode_cache_path.clone()),
            pack_config.clean_cache_after_decode,
        );
        p.decode_async(Cursor::new(pack_file), sender); //Pack moved here
        Ok(receiver)
    }
}
