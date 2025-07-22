use std::{
    collections::{HashMap, VecDeque},
    path::{Component, Path, PathBuf},
};

use async_trait::async_trait;

use callisto::raw_blob;
use common::errors::MegaError;
use jupiter::{storage::Storage, utils::converter::generate_git_keep_with_timestamp};
use mercury::{
    errors::GitError,
    hash::SHA1,
    internal::object::{
        commit::Commit,
        tree::{Tree, TreeItem, TreeItemMode},
        ObjectTrait,
    },
};

use crate::model::git::{
    CreateFileInfo, LatestCommitInfo, TreeBriefItem, TreeCommitItem, TreeHashItem,
};

pub mod import_api_service;
pub mod mono_api_service;

#[derive(Debug, Default, Clone)]
pub struct GitObjectCache {
    trees: HashMap<SHA1, Tree>,
    commits: HashMap<SHA1, Commit>,
}

#[async_trait]
pub trait ApiHandler: Send + Sync {
    fn get_context(&self) -> Storage;

    async fn create_monorepo_file(&self, file_info: CreateFileInfo) -> Result<(), GitError>;

    async fn get_raw_blob_by_hash(&self, hash: &str) -> Result<Option<raw_blob::Model>, MegaError> {
        let context = self.get_context();
        context.raw_db_storage().get_raw_blob_by_hash(hash).await
    }

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, GitError>;

    async fn get_root_commit(&self) -> Commit;

    async fn get_root_tree(&self) -> Tree;

    async fn get_binary_tree_by_path(
        &self,
        path: &Path,
        oid: Option<String>,
    ) -> Result<Vec<u8>, GitError> {
        let Some(tree) = self.search_tree_by_path(path).await.unwrap() else {
            return Ok(vec![]);
        };
        if let Some(oid) = oid {
            if oid != tree.id._to_string() {
                return Ok(vec![]);
            }
        }
        tree.to_data()
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree;

    async fn get_commit_by_hash(&self, hash: &str) -> Option<Commit>;

    async fn get_tree_relate_commit(&self, t_hash: &str) -> Commit;

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError>;

    async fn get_blob_as_string(&self, file_path: PathBuf) -> Result<Option<String>, GitError> {
        let filename = file_path.file_name().unwrap().to_str().unwrap();
        let parent = file_path.parent().unwrap();
        if let Some(tree) = self.search_tree_by_path(parent).await? {
            if let Some(item) = tree.tree_items.into_iter().find(|x| x.name == filename) {
                match self.get_raw_blob_by_hash(&item.id.to_string()).await {
                    Ok(Some(model)) => {
                        return Ok(Some(String::from_utf8(model.data.unwrap()).unwrap()))
                    }
                    _ => return Ok(None),
                };
            }
        }
        return Ok(None);
    }

    async fn get_latest_commit(&self, path: PathBuf) -> Result<LatestCommitInfo, GitError> {
        let tree = if let Some(tree) = self.search_tree_by_path(&path).await? {
            tree
        } else {
            return Err(GitError::CustomError(
                "can't find target parent tree under latest commit".to_string(),
            ));
        };
        let commit = self.get_tree_relate_commit(&tree.id.to_string()).await;
        Ok(commit.into())
    }

    async fn get_tree_info(&self, path: &Path) -> Result<Vec<TreeBriefItem>, GitError> {
        match self.search_tree_by_path(path).await? {
            Some(tree) => {
                let items = tree
                    .tree_items
                    .into_iter()
                    .map(|item| {
                        let full_path = path.join(&item.name);
                        let mut info: TreeBriefItem = item.into();
                        info.path = full_path.to_str().unwrap().to_owned();
                        info
                    })
                    .collect();
                Ok(items)
            }
            None => Ok(vec![]),
        }
    }

    async fn get_tree_commit_info(&self, path: PathBuf) -> Result<Vec<TreeCommitItem>, GitError> {
        let item_to_commit_map = self.item_to_commit_map(path).await?;

        let mut items: Vec<TreeCommitItem> = item_to_commit_map
            .into_iter()
            .map(TreeCommitItem::from)
            .collect();
        // sort with type and name
        items.sort_by(|a, b| {
            a.content_type
                .cmp(&b.content_type)
                .then(a.name.cmp(&b.name))
        });
        Ok(items)
    }

    async fn item_to_commit_map(
        &self,
        path: PathBuf,
    ) -> Result<HashMap<TreeItem, Option<Commit>>, GitError>;

    /// the dir's hash as same as old,file's hash is the content hash
    /// may think about change dir'hash as the content
    /// for now,only change the file's hash
    async fn get_tree_content_hash(&self, path: PathBuf) -> Result<Vec<TreeHashItem>, GitError> {
        match self.search_tree_by_path(&path).await? {
            Some(tree) => {
                let mut items: Vec<TreeHashItem> = tree
                    .tree_items
                    .into_iter()
                    .map(TreeHashItem::from)
                    .collect();

                // sort with type and name
                items.sort_by(|a, b| {
                    a.content_type
                        .cmp(&b.content_type)
                        .then(a.name.cmp(&b.name))
                });
                Ok(items)
            }
            None => Ok(Vec::new()),
        }
    }

    /// return the dir's hash only
    async fn get_tree_dir_hash(
        &self,
        path: PathBuf,
        dir_name: &str,
    ) -> Result<Vec<TreeHashItem>, GitError> {
        match self.search_tree_by_path(&path).await? {
            Some(tree) => {
                let items: Vec<TreeHashItem> = tree
                    .tree_items
                    .into_iter()
                    .filter(|x| x.mode == TreeItemMode::Tree && x.name == dir_name)
                    .map(TreeHashItem::from)
                    .collect();
                Ok(items)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Searches for a tree in the Git repository by its path and returns the trees involved in the update and the target tree.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path to search for.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - A vector of trees involved in the update process.
    /// - The target tree found at the end of the search.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the path does not exist.
    async fn search_tree_for_update(&self, path: &Path) -> Result<(Vec<Tree>, Tree), GitError> {
        let relative_path = self.strip_relative(path)?;
        let root_tree = self.get_root_tree().await;
        let mut search_tree = root_tree.clone();
        let mut update_tree = vec![root_tree];

        for component in relative_path.components() {
            // root tree already found
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);

                if let Some(search_res) = search_res {
                    let res = self.get_tree_by_hash(&search_res.id.to_string()).await;
                    search_tree = res.clone();
                    update_tree.push(res);
                } else {
                    return Err(GitError::CustomError(
                        "Path not exist, please create path first!".to_string(),
                    ));
                }
            }
        }
        Ok((update_tree, search_tree))
    }

    /// Searches for a tree by a given path.
    ///
    /// This function takes a `path` and searches for the corresponding tree
    /// in the repository. It returns a `Result` containing an `Option<Tree>`.
    /// If the tree is found, it returns `Some(Tree)`. If the path does not
    /// exist, it returns `None`. In case of an error, it returns a `GitError`.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the `Path` to search for the tree.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Tree>, GitError>` - A result containing an optional tree or a Git error.
    async fn search_tree_by_path(&self, path: &Path) -> Result<Option<Tree>, GitError> {
        let relative_path = self.strip_relative(path)?;
        let root_tree = self.get_root_tree().await;
        let mut search_tree = root_tree.clone();
        for component in relative_path.components() {
            // root tree already found
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);
                if let Some(search_res) = search_res {
                    if !search_res.is_tree() {
                        return Ok(None);
                    }
                    let res = self.get_tree_by_hash(&search_res.id.to_string()).await;
                    search_tree = res.clone();
                } else {
                    return Ok(None);
                }
            }
        }
        Ok(Some(search_tree))
    }

    /// Searches for a tree in the Git repository by its path, creating intermediate trees if necessary,
    /// and returns the trees involved in the update process.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path to search for.
    ///
    /// # Returns
    ///
    /// A vector of trees involved in the update process.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if an error occurs during the search or tree creation process.
    async fn search_and_create_tree(&self, path: &Path) -> Result<VecDeque<Tree>, GitError> {
        let relative_path = self.strip_relative(path)?;
        let root_tree = self.get_root_tree().await;
        let mut search_tree = root_tree.clone();
        let mut update_item_tree = VecDeque::new();
        update_item_tree.push_back((root_tree, Component::RootDir));
        let mut saving_trees = VecDeque::new();
        let mut stack: VecDeque<_> = VecDeque::new();

        for component in relative_path.components() {
            if component == Component::RootDir {
                continue;
            }

            let target_name = component.as_os_str().to_str().unwrap();
            if let Some(search_res) = search_tree
                .tree_items
                .iter()
                .find(|x| x.name == target_name)
            {
                search_tree = self.get_tree_by_hash(&search_res.id.to_string()).await;
                update_item_tree.push_back((search_tree.clone(), component));
            } else {
                stack.push_back(component);
            }
        }

        let blob = generate_git_keep_with_timestamp();
        let mut last_tree = Tree::from_tree_items(vec![TreeItem {
            mode: TreeItemMode::Blob,
            id: blob.id,
            name: String::from(".gitkeep"),
        }])
        .unwrap();
        let mut last_tree_name = "";
        let mut first_element = true;

        while let Some(component) = stack.pop_back() {
            if first_element {
                first_element = false;
            } else {
                last_tree = Tree::from_tree_items(vec![TreeItem {
                    mode: TreeItemMode::Tree,
                    id: last_tree.id,
                    name: last_tree_name.to_owned(),
                }])
                .unwrap();
            }
            saving_trees.push_back(last_tree.clone());
            last_tree_name = component.as_os_str().to_str().unwrap();
        }

        if let Some((mut new_item_tree, search_name_component)) = update_item_tree.pop_back() {
            new_item_tree.tree_items.push(TreeItem {
                mode: TreeItemMode::Tree,
                id: last_tree.id,
                name: last_tree_name.to_owned(),
            });
            last_tree = Tree::from_tree_items(new_item_tree.tree_items).unwrap();
            saving_trees.push_back(last_tree.clone());

            let mut replace_hash = last_tree.id;
            let mut search_name = search_name_component.as_os_str().to_str().unwrap();
            while let Some((mut tree, component)) = update_item_tree.pop_back() {
                if let Some(index) = tree.tree_items.iter().position(|x| x.name == search_name) {
                    tree.tree_items[index].id = replace_hash;
                    let new_tree = Tree::from_tree_items(tree.tree_items).unwrap();
                    replace_hash = new_tree.id;
                    search_name = component.as_os_str().to_str().unwrap();
                    saving_trees.push_back(new_tree);
                }
            }
        }

        Ok(saving_trees)
    }
}
