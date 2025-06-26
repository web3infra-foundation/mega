use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};

use async_trait::async_trait;
use tokio::process::Command;

use callisto::sea_orm_active_enums::ConvTypeEnum;
use callisto::{mega_blob, mega_mr, mega_tree, raw_blob};
use common::errors::MegaError;
use jupiter::storage::batch_save_model;
use jupiter::storage::Storage;
use jupiter::utils::converter::generate_git_keep_with_timestamp;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};

use crate::api_service::ApiHandler;
use crate::model::git::CreateFileInfo;

#[derive(Clone)]
pub struct MonoApiService {
    pub storage: Storage,
}

#[async_trait]
impl ApiHandler for MonoApiService {
    fn get_context(&self) -> Storage {
        self.storage.clone()
    }

    /// Creates a new file or directory in the monorepo based on the provided file information.
    ///
    /// # Arguments
    ///
    /// * `file_info` - Information about the file or directory to create.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `GitError` on failure.
    async fn create_monorepo_file(&self, file_info: CreateFileInfo) -> Result<(), GitError> {
        let storage = self.storage.services.mono_storage.clone();
        let path = PathBuf::from(file_info.path);
        let mut save_trees = vec![];

        // Search for the tree to update and get its tree items
        let (update_trees, search_tree) = self.search_tree_for_update(&path).await?;
        let mut t_items = search_tree.tree_items;

        // Create a new tree item based on whether it's a directory or file
        let new_item = if file_info.is_directory {
            if t_items
                .iter()
                .any(|x| x.mode == TreeItemMode::Tree && x.name == file_info.name)
            {
                return Err(GitError::CustomError("Duplicate name".to_string()));
            }
            let blob = generate_git_keep_with_timestamp();
            let tree_item = TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: String::from(".gitkeep"),
            };
            let child_tree = Tree::from_tree_items(vec![tree_item]).unwrap();
            save_trees.push(child_tree.clone());
            TreeItem {
                mode: TreeItemMode::Tree,
                id: child_tree.id,
                name: file_info.name.clone(),
            }
        } else {
            let content = file_info.content.unwrap();
            let blob = Blob::from_content(&content);
            let mega_blob: mega_blob::ActiveModel = Into::<mega_blob::Model>::into(&blob).into();
            let raw_blob: raw_blob::ActiveModel =
                Into::<raw_blob::Model>::into(blob.clone()).into();

            let conn = storage.get_connection();
            batch_save_model(conn, vec![mega_blob]).await.unwrap();
            batch_save_model(conn, vec![raw_blob]).await.unwrap();
            TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: file_info.name.clone(),
            }
        };
        // Add the new item to the tree items and create a new tree
        t_items.push(new_item);
        let p_tree = Tree::from_tree_items(t_items).unwrap();

        // Create a commit for the new tree
        let refs = storage.get_ref("/").await.unwrap().unwrap();
        let commit = Commit::from_tree_id(
            p_tree.id,
            vec![SHA1::from_str(&refs.ref_commit_hash).unwrap()],
            &format!("\ncreate file {} commit", file_info.name),
        );

        // Update the parent tree with the new commit
        let commit_id = self.update_parent_tree(path, update_trees, commit).await?;
        save_trees.push(p_tree);

        let save_trees: Vec<mega_tree::ActiveModel> = save_trees
            .into_iter()
            .map(|save_t| {
                let mut tree_model: mega_tree::Model = save_t.into();
                tree_model.commit_id.clone_from(&commit_id);
                tree_model.into()
            })
            .collect();
        batch_save_model(storage.get_connection(), save_trees)
            .await
            .unwrap();
        Ok(())
    }

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, GitError> {
        Ok(path.to_path_buf())
    }

    async fn get_root_commit(&self) -> Commit {
        unreachable!()
    }

    async fn get_root_tree(&self) -> Tree {
        let storage = self.storage.services.mono_storage.clone();
        let refs = storage.get_ref("/").await.unwrap().unwrap();

        storage
            .get_tree_by_hash(&refs.ref_tree_hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree {
        self.storage
            .services
            .mono_storage
            .get_tree_by_hash(hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Option<Commit> {
        let storage = self.storage.services.mono_storage.clone();
        match storage.get_commit_by_hash(hash).await {
            Ok(Some(commit)) => Some(commit.into()),
            _ => None,
        }
    }

    async fn get_tree_relate_commit(&self, t_hash: &str) -> Commit {
        let storage = self.storage.services.mono_storage.clone();
        let tree_info = storage.get_tree_by_hash(t_hash).await.unwrap().unwrap();
        storage
            .get_commit_by_hash(&tree_info.commit_id)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let storage = self.storage.services.mono_storage.clone();
        let commits = storage.get_commits_by_hashes(&c_hashes).await.unwrap();
        Ok(commits.into_iter().map(|x| x.into()).collect())
    }

    async fn item_to_commit_map(
        &self,
        path: PathBuf,
    ) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
        match self.search_tree_by_path(&path).await? {
            Some(tree) => {
                let mut item_to_commit = HashMap::new();

                let storage = self.storage.services.mono_storage.clone();
                let tree_hashes = tree
                    .tree_items
                    .iter()
                    .filter(|x| x.mode == TreeItemMode::Tree)
                    .map(|x| x.id.to_string())
                    .collect();
                let trees = storage.get_trees_by_hashes(tree_hashes).await.unwrap();
                for tree in trees {
                    item_to_commit.insert(tree.tree_id, tree.commit_id);
                }

                let blob_hashes = tree
                    .tree_items
                    .iter()
                    .filter(|x| x.mode == TreeItemMode::Blob)
                    .map(|x| x.id.to_string())
                    .collect();
                let blobs = storage.get_mega_blobs_by_hashes(blob_hashes).await.unwrap();
                for blob in blobs {
                    item_to_commit.insert(blob.blob_id, blob.commit_id);
                }

                let commit_ids: HashSet<String> = item_to_commit.values().cloned().collect();
                let commits = self
                    .get_commits_by_hashes(commit_ids.into_iter().collect())
                    .await
                    .unwrap();
                let commit_map: HashMap<String, Commit> =
                    commits.into_iter().map(|x| (x.id.to_string(), x)).collect();

                let mut result: HashMap<TreeItem, Option<Commit>> = HashMap::new();
                for item in tree.tree_items {
                    if let Some(commit_id) = item_to_commit.get(&item.id.to_string()) {
                        let commit = if let Some(commit) = commit_map.get(commit_id) {
                            Some(commit.to_owned())
                        } else {
                            tracing::warn!("failed fetch from commit map: {}", commit_id);
                            None
                        };
                        result.insert(item, commit);
                    }
                }
                Ok(result)
            }
            None => Ok(HashMap::new()),
        }
    }
}

impl MonoApiService {
    pub async fn merge_mr(&self, user_id: String, mr: mega_mr::Model) -> Result<(), MegaError> {
        let storage = self.storage.services.mono_storage.clone();
        let refs = storage.get_ref(&mr.path).await.unwrap().unwrap();

        if mr.from_hash == refs.ref_commit_hash {
            let commit: Commit = storage
                .get_commit_by_hash(&mr.to_hash)
                .await
                .unwrap()
                .unwrap()
                .into();

            if mr.path != "/" {
                let path = PathBuf::from(mr.path.clone());
                // beacuse only parent tree is needed so we skip current directory
                let (tree_vec, _) = self
                    .search_tree_for_update(path.parent().unwrap())
                    .await
                    .unwrap();
                self.update_parent_tree(path, tree_vec, commit)
                    .await
                    .unwrap();
                // remove refs start with path exceprt mr type
                storage.remove_none_mr_refs(&mr.path).await.unwrap();
                // TODO: self.clean_dangling_commits().await;
            }
            // add conversation
            self.storage
                .issue_storage()
                .add_conversation(&mr.link, &user_id, None, ConvTypeEnum::Merged)
                .await
                .unwrap();
            // update mr status last
            self.storage
                .mr_storage()
                .merge_mr(mr.clone())
                .await
                .unwrap();
        } else {
            return Err(MegaError::with_message("ref hash conflict"));
        }
        Ok(())
    }

    async fn update_parent_tree(
        &self,
        mut path: PathBuf,
        mut tree_vec: Vec<Tree>,
        commit: Commit,
    ) -> Result<String, GitError> {
        let storage = self.storage.services.mono_storage.clone();
        let mut save_trees = Vec::new();
        let mut p_commit_id = String::new();

        let mut target_hash = commit.tree_id;

        while let Some(mut tree) = tree_vec.pop() {
            let cloned_path = path.clone();
            let name = cloned_path.file_name().unwrap().to_str().unwrap();
            path.pop();

            let index = tree.tree_items.iter().position(|x| x.name == name).unwrap();
            tree.tree_items[index].id = target_hash;
            let new_tree = Tree::from_tree_items(tree.tree_items).unwrap();
            target_hash = new_tree.id;

            let model: mega_tree::Model = new_tree.into();
            save_trees.push(model);

            let p_ref = storage.get_ref(path.to_str().unwrap()).await.unwrap();
            if let Some(mut p_ref) = p_ref {
                if path == Path::new("/") {
                    let p_commit = Commit::new(
                        commit.author.clone(),
                        commit.committer.clone(),
                        target_hash,
                        vec![SHA1::from_str(&p_ref.ref_commit_hash).unwrap()],
                        &commit.message,
                    );
                    p_commit_id = p_commit.id.to_string();
                    // update p_ref
                    p_ref.ref_commit_hash = p_commit.id.to_string();
                    p_ref.ref_tree_hash = target_hash.to_string();
                    storage.update_ref(p_ref).await.unwrap();
                    storage.save_mega_commits(vec![p_commit]).await.unwrap();
                } else {
                    storage.remove_ref(p_ref).await.unwrap();
                }
            }
        }
        let save_trees: Vec<mega_tree::ActiveModel> = save_trees
            .into_iter()
            .map(|mut x| {
                p_commit_id.clone_into(&mut x.commit_id);
                x.into()
            })
            .collect();

        batch_save_model(storage.get_connection(), save_trees)
            .await
            .unwrap();
        Ok(p_commit_id)
    }

    pub async fn content_diff(&self, mr_link: &str, listen_addr: &str) -> Result<String, GitError> {
        let stg = self.storage.mr_storage();
        if let Some(mr) = stg.get_mr(mr_link).await.unwrap() {
            let base_path = self.storage.config().base_dir.clone();
            env::set_current_dir(&base_path).unwrap();
            let clone_path = base_path.join(mr_link);
            if !fs::exists(&clone_path).unwrap() {
                let result = self.run_libra_diff(&mr, listen_addr, &clone_path).await;
                if result.is_err() && fs::exists(&clone_path).unwrap() {
                    fs::remove_dir_all(&clone_path).unwrap();
                }
            } else {
                env::set_current_dir(&clone_path).unwrap();
            }
            tracing::debug!("Run libra Command: libra diff --old {}", mr.from_hash);
            let output = Command::new("libra")
                .arg("diff")
                .arg("--old")
                .arg(mr.from_hash)
                .output()
                .await?;

            let stderr_str = String::from_utf8_lossy(&output.stderr);

            if !stderr_str.trim().is_empty() || !output.status.success() {
                tracing::error!(
                    "Command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                fs::remove_dir_all(&clone_path).unwrap();
            } else {
                return Ok(String::from_utf8_lossy(&output.stdout).to_string());
            }
        }
        Ok(String::new())
    }

    async fn run_libra_diff(
        &self,
        mr: &callisto::mega_mr::Model,
        listen_addr: &str,
        clone_path: &PathBuf,
    ) -> Result<(), anyhow::Error> {
        Command::new("mkdir").arg(&mr.link).output().await?;
        env::set_current_dir(clone_path).unwrap();
        Command::new("libra").arg("init").output().await?;
        let git_remote = if mr.path.starts_with("/") {
            format!("{}{}", listen_addr, mr.path)
        } else {
            format!("{}/{}", listen_addr, mr.path)
        };
        tracing::debug!("Run libra Command: libra remote add origin {}", &git_remote);
        Command::new("libra")
            .arg("remote")
            .arg("add")
            .arg("origin")
            .arg(git_remote)
            .output()
            .await?;
        tracing::debug!("Run libra Command: libra fetch origin refs/mr/{}", &mr.link);
        Command::new("libra")
            .arg("fetch")
            .arg("origin")
            .arg(format!("refs/mr/{}", &mr.link))
            .output()
            .await?;
        tracing::debug!(
            "Run libra Command: libra branch {} origin/mr/{}",
            &mr.link,
            &mr.link
        );
        Command::new("libra")
            .arg("branch")
            .arg(&mr.link)
            .arg(format!("origin/mr/{}", &mr.link))
            .output()
            .await?;
        tracing::debug!("Run libra Command: libra switch {}", &mr.link);
        Command::new("libra")
            .arg("switch")
            .arg(&mr.link)
            .output()
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    pub fn test() {
        let mut full_path = PathBuf::from("/project/rust/mega");
        for _ in 0..3 {
            let cloned_path = full_path.clone(); // Clone full_path
            let name = cloned_path.file_name().unwrap().to_str().unwrap();
            full_path.pop();
            println!("name: {}, path: {:?}", name, full_path);
        }
    }
}
