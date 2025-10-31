use std::{
    collections::HashSet,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, future::join_all};
use sysinfo::System;
use tokio::sync::{Semaphore, mpsc::UnboundedReceiver};
use tokio_stream::wrappers::ReceiverStream;

use crate::protocol::import_refs::{RefCommand, Refs};
use callisto::raw_blob;
use common::{
    config::PackConfig,
    errors::{MegaError, ProtocolError},
    utils::ZERO_ID,
};
use git_internal::internal::pack::Pack;
use git_internal::{
    errors::GitError,
    internal::{
        object::{
            blob::Blob,
            tree::{Tree, TreeItemMode},
        },
        pack::entry::Entry,
    },
};
use git_internal::hash::SHA1;
use git_internal::internal::metadata::{EntryMeta, MetaAttached};
use uuid::Uuid;

pub mod import_repo;
pub mod monorepo;

#[async_trait]
pub trait RepoHandler: Send + Sync + 'static {
    fn is_monorepo(&self) -> bool;

    async fn refs_with_head_hash(&self) -> (String, Vec<Refs>);

    async fn receiver_handler(
        self: Arc<Self>,
        mut rx: UnboundedReceiver<MetaAttached<Entry,EntryMeta>>,
        mut rx_pack_id: UnboundedReceiver<SHA1>,
    ) -> Result<(), GitError> {
        let mut entry_list = vec![];
        let semaphore = Arc::new(Semaphore::new(4));
        let mut join_tasks = vec![];

        let temp_pack_id = Uuid::new_v4().to_string();
        let mut update_flag = false;
        while let Some(mut entry) = rx.recv().await {
            if self.check_entry(&entry.inner).await?{
                update_flag = true;
            };
            entry.meta.set_pack_id(temp_pack_id.clone());
            entry_list.push(entry);
            if entry_list.len() >= 1000 {
                let acquired = semaphore.clone().acquire_owned().await.unwrap();
                let entries = std::mem::take(&mut entry_list);
                let shared = self.clone();
                let handle = tokio::spawn(async move {
                    let _acquired = acquired;
                    shared.save_entry(entries).await
                });
                join_tasks.push(handle);
            }
        }
        // process left entries
        if !entry_list.is_empty() {
            let handler = self.clone();
            let entries = std::mem::take(&mut entry_list);
            let handle = tokio::spawn(async move { handler.save_entry(entries).await });
            join_tasks.push(handle);
        }

        let results = join_all(join_tasks).await;
        for (i, res) in results.into_iter().enumerate() {
            match res {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::error!("Task {} save_entry Err: {:?}", i, e);
                }
                Err(join_err) => {
                    tracing::error!("Task {} panic or cancle: {:?}", i, join_err);
                }
            }
        }

        // receive pack_id and update it
        if let Some(real_pack_id) = rx_pack_id.recv().await{
            let real_pack_id_str = real_pack_id.to_string();
            tracing::debug!("Received real pack_id: {}, updating database from temp_pack_id: {}", real_pack_id_str, temp_pack_id);

            // 通过数据库操作更新 pack_id
            if let Err(e) = self.update_pack_id(&temp_pack_id, &real_pack_id_str).await {
                tracing::error!("Failed to update pack_id in database: {:?}", e);
                return Err(GitError::CustomError(format!("Failed to update pack_id: {:?}", e)));
            }
        }
        
        // // if have new commit traverse trees and update filepath of blobs 
        // if update_flag {
        //     
        // }
        
        Ok(())
    }

    async fn post_receive_pack(&self) -> Result<(), MegaError>;

    async fn save_entry(&self, entry_list: Vec<MetaAttached<Entry,EntryMeta>>) -> Result<(), MegaError>;
    
    async fn update_pack_id(&self, temp_pack_id: &str, pack_id: &str) -> Result<(), MegaError>;

    async fn check_entry(&self, entry: &Entry) -> Result<bool, GitError>;

    /// Asynchronously retrieves the full pack data for the specified repository path.
    /// This function collects commits and nodes from the storage and packs them into
    /// a single binary vector. There is no need to build the entire tree; the function
    /// only sends all the data related to this repository.
    ///
    /// # Returns
    /// * `Result<Vec<u8>, GitError>` - The packed binary data as a vector of bytes.
    ///
    async fn full_pack(&self, want: Vec<String>) -> Result<ReceiverStream<Vec<u8>>, GitError>;

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<ReceiverStream<Vec<u8>>, GitError>;

    async fn get_trees_by_hashes(&self, hashes: Vec<String>) -> Result<Vec<Tree>, MegaError>;

    async fn get_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<raw_blob::Model>, MegaError>;
    
    async fn get_blob_metadata_by_hashes( 
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<EntryMeta>, MegaError>;

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError>;

    async fn check_commit_exist(&self, hash: &str) -> bool;

    async fn check_default_branch(&self) -> bool;

    fn find_head_hash(&self, refs: Vec<Refs>) -> (String, Vec<Refs>) {
        let mut head_hash = ZERO_ID.to_string();
        for git_ref in refs.iter() {
            if git_ref.default_branch {
                head_hash.clone_from(&git_ref.ref_hash);
            }
        }
        (head_hash, refs)
    }

    async fn unpack_stream(
        &self,
        pack_config: &PackConfig,
        stream: Pin<Box<dyn Stream<Item = Result<Bytes, axum::Error>> + Send>>,
    ) -> Result<(UnboundedReceiver<MetaAttached<Entry,EntryMeta>>,UnboundedReceiver<SHA1>), ProtocolError> {
        let total_mem = || {
            let sys = System::new_all();
            Ok(sys.total_memory() as usize)
        };

        let cache_mem =
            match PackConfig::get_size_from_str(&pack_config.pack_decode_mem_size, total_mem) {
                Ok(mem) => mem,
                Err(err) => return Err(ProtocolError::InvalidInput(err)),
            };

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let (pack_id_sender, pack_id_receiver) = tokio::sync::mpsc::unbounded_channel();

        let p = Pack::new(
            None,
            Some(cache_mem),
            Some(pack_config.pack_decode_cache_path.clone()),
            pack_config.clean_cache_after_decode,
        );
        p.decode_stream(stream, sender,Some(pack_id_sender)).await;
        Ok((receiver,pack_id_receiver))
    }

    async fn traverse_for_count(
        &self,
        tree: Tree,
        exist_objs: &HashSet<String>,
        counted_obj: &mut HashSet<String>,
        obj_num: &AtomicUsize,
    ) {
        let mut search_tree_ids = vec![];
        let mut search_blob_ids = vec![];
        for item in &tree.tree_items {
            let hash = item.id.to_string();
            if !exist_objs.contains(&hash) && counted_obj.insert(hash.clone()) {
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
            self.traverse_for_count(t, exist_objs, counted_obj, obj_num)
                .await;
        }
        obj_num.fetch_add(1, Ordering::SeqCst);
    }

    /// Traverse a tree structure asynchronously.
    ///
    /// This function traverses a given tree, keeps track of processed objects, and optionally sends
    /// traversal data to a provided sender. The function will:
    /// 1. Traverse the tree and calculate the quantities of tree and blob items.
    /// 2. If a sender is provided, send blob and tree data via the sender.
    ///
    /// # Parameters
    /// - `tree`: The tree structure to traverse.
    /// - `exist_objs`: A mutable reference to a set containing already processed object IDs.
    /// - `sender`: An optional sender for sending traversal data.
    ///
    /// # Details
    /// - The function processes tree items, distinguishing between tree and blob items.
    /// - It collects IDs of items that have not been processed yet.
    /// - It retrieves and sends blob data if a sender is provided.
    /// - It recursively traverses sub-trees.
    /// - It sends the entire tree data if a sender is provided.
    async fn traverse(
        &self,
        tree: Tree,
        exist_objs: &mut HashSet<String>,
        sender: Option<&tokio::sync::mpsc::Sender<Entry>>,
    ) {
        let mut search_tree_ids = vec![];
        let mut search_blob_ids = vec![];

        for item in &tree.tree_items {
            let hash = item.id.to_string();
            if exist_objs.insert(hash.clone()) {
                if item.mode == TreeItemMode::Tree {
                    search_tree_ids.push(hash);
                } else {
                    search_blob_ids.push(hash);
                }
            }
        }

        if let Some(sender) = sender {
            let blobs = self.get_blobs_by_hashes(search_blob_ids).await.unwrap();
            for b in blobs {
                let data = b.data.unwrap_or_default();
                let blob: Blob = Blob::from_content_bytes(data);
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

    async fn traverses_tree_and_update_filepath(&self) ->  Result<(), MegaError>;
}
