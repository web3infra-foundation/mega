use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
};

use axum::async_trait;

use callisto::raw_blob;
use common::errors::MegaError;
use mercury::{
    errors::GitError,
    internal::object::{
        commit::Commit,
        tree::{Tree, TreeItem, TreeItemMode},
    },
};

use crate::model::{
    create_file::CreateFileInfo,
    objects::{
        BlobObjects, LatestCommitInfo, TreeBriefInfo, TreeBriefItem, TreeCommitInfo,
        TreeCommitItem, UserInfo,
    },
};

pub mod import_api_service;
pub mod mono_api_service;

#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn create_monorepo_file(&self, file_info: CreateFileInfo) -> Result<(), GitError>;

    async fn get_raw_blob_by_hash(&self, hash: &str) -> Result<Option<raw_blob::Model>, MegaError>;

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, GitError>;

    async fn get_root_commit(&self) -> Commit;

    async fn get_root_tree(&self) -> Tree;

    async fn get_tree_by_hash(&self, hash: &str) -> Tree;

    async fn get_tree_relate_commit(&self, t_hash: &str) -> Commit;

    async fn add_trees_to_map(
        &self,
        item_to_commit: &mut HashMap<String, String>,
        hashes: Vec<String>,
    );

    async fn add_blobs_to_map(
        &self,
        item_to_commit: &mut HashMap<String, String>,
        hashes: Vec<String>,
    );

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError>;

    async fn traverse_commit_history(
        &self,
        path: &Path,
        commit: Commit,
        target: TreeItem,
    ) -> Commit;

    async fn get_blob_as_string(
        &self,
        path: PathBuf,
        filename: &str,
    ) -> Result<BlobObjects, GitError> {
        let (_, tree) = self.search_tree_by_path(&path).await.unwrap();
        let mut plain_text = String::new();
        if let Some(item) = tree.tree_items.into_iter().find(|x| x.name == filename) {
            plain_text = match self.get_raw_blob_by_hash(&item.id.to_plain_str()).await {
                Ok(Some(model)) => String::from_utf8(model.data.unwrap()).unwrap(),
                _ => String::new(),
            };
        }
        Ok(BlobObjects { plain_text })
    }

    async fn get_latest_commit(&self, path: PathBuf) -> Result<LatestCommitInfo, GitError> {
        let (_, tree) = self.search_tree_by_path(&path).await.unwrap();
        let commit = self.get_tree_relate_commit(&tree.id.to_plain_str()).await;
        self.convert_commit_to_info(commit)
    }

    async fn get_tree_info(&self, path: PathBuf) -> Result<TreeBriefInfo, GitError> {
        match self.search_tree_by_path(&path).await {
            Ok((_, tree)) => {
                let mut items = Vec::new();
                for item in tree.tree_items {
                    let mut info: TreeBriefItem = item.clone().into();
                    path.join(item.name)
                        .to_str()
                        .unwrap()
                        .clone_into(&mut info.path);
                    items.push(info);
                }
                Ok(TreeBriefInfo {
                    total_count: items.len(),
                    items,
                })
            }
            Err(_) => Ok(TreeBriefInfo {
                total_count: 0,
                items: Vec::new(),
            }),
        }
    }

    async fn get_tree_commit_info(&self, path: PathBuf) -> Result<TreeCommitInfo, GitError> {
        match self.search_tree_by_path(&path).await {
            Ok((_, tree)) => {
                let mut item_to_commit = HashMap::new();

                self.add_trees_to_map(
                    &mut item_to_commit,
                    tree.tree_items
                        .iter()
                        .filter(|x| x.mode == TreeItemMode::Tree)
                        .map(|x| x.id.to_plain_str())
                        .collect(),
                )
                .await;

                self.add_blobs_to_map(
                    &mut item_to_commit,
                    tree.tree_items
                        .iter()
                        .filter(|x| x.mode == TreeItemMode::Blob)
                        .map(|x| x.id.to_plain_str())
                        .collect(),
                )
                .await;

                let mut items = Vec::new();
                let commit_ids: HashSet<String> = item_to_commit.values().cloned().collect();
                let commits = self
                    .get_commits_by_hashes(commit_ids.into_iter().collect())
                    .await
                    .unwrap();
                let commit_map: HashMap<String, Commit> = commits
                    .into_iter()
                    .map(|x| (x.id.to_plain_str(), x))
                    .collect();

                for item in tree.tree_items {
                    let mut info: TreeCommitItem = item.clone().into();
                    if let Some(commit_id) = item_to_commit.get(&item.id.to_plain_str()) {
                        let commit = if let Some(commit) = commit_map.get(commit_id) {
                            commit
                        } else {
                            tracing::error!("failed fecth commit: {}", commit_id);
                            &self
                                .traverse_commit_history(&path, self.get_root_commit().await, item)
                                .await
                        };
                        info.oid = commit.id.to_plain_str();
                        info.message = commit.format_message();
                        info.date = commit.committer.timestamp.to_string();
                    }
                    items.push(info);
                }
                Ok(TreeCommitInfo {
                    total_count: items.len(),
                    items,
                })
            }
            Err(_) => Ok(TreeCommitInfo {
                total_count: 0,
                items: Vec::new(),
            }),
        }
    }

    fn convert_commit_to_info(&self, commit: Commit) -> Result<LatestCommitInfo, GitError> {
        let message = commit.format_message();
        let committer = UserInfo {
            display_name: commit.committer.name,
            ..Default::default()
        };
        let author = UserInfo {
            display_name: commit.author.name,
            ..Default::default()
        };

        let res = LatestCommitInfo {
            oid: commit.id.to_plain_str(),
            date: commit.committer.timestamp.to_string(),
            short_message: message,
            author,
            committer,
            status: "success".to_string(),
        };
        Ok(res)
    }

    /// Searches for a tree and affected parent by path.
    ///
    /// This function asynchronously searches for a tree by the provided path.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path to search.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing a vector of parent trees to be updated and
    /// the target tree if found, or an error of type `GitError`.
    async fn search_tree_by_path(&self, path: &Path) -> Result<(Vec<Tree>, Tree), GitError> {
        let relative_path = self.strip_relative(path).unwrap();
        let root_tree = self.get_root_tree().await;
        let mut search_tree = root_tree.clone();
        let mut update_tree = vec![root_tree];
        let component_num = relative_path.components().count();

        for (index, component) in relative_path.components().enumerate() {
            // root tree already found
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);

                if let Some(search_res) = search_res {
                    let res = self.get_tree_by_hash(&search_res.id.to_plain_str()).await;
                    search_tree = res.clone();
                    // skip last component
                    if index != component_num - 1 {
                        update_tree.push(res);
                    }
                } else {
                    return Err(GitError::ConversionError(
                        "can't find target parent tree under latest commit".to_string(),
                    ));
                }
            }
        }
        Ok((update_tree, search_tree))
    }

    async fn reachable_in_tree(
        &self,
        root_tree: &Tree,
        path: &Path,
        target: TreeItem,
    ) -> Result<bool, GitError> {
        let relative_path = self.strip_relative(path).unwrap();
        let mut search_tree = root_tree.clone();
        // first find search tree by path
        for component in relative_path.components() {
            // root tree already found
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);
                if let Some(search_res) = search_res {
                    search_tree = self.get_tree_by_hash(&search_res.id.to_plain_str()).await;
                } else {
                    return Ok(false);
                }
            }
        }
        // check item exist under search tree
        if search_tree.tree_items.into_iter().any(|x| x == target) {
            return Ok(true);
        }
        Ok(false)
    }
}
