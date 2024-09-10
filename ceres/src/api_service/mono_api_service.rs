use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use axum::async_trait;

use callisto::db_enums::{ConvType, MergeStatus};
use callisto::{mega_blob, mega_tree, raw_blob};
use common::errors::MegaError;
use jupiter::context::Context;
use jupiter::storage::batch_save_model;
use jupiter::utils::converter::generate_git_keep_with_timestamp;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};

use crate::api_service::ApiHandler;
use crate::model::create_file::CreateFileInfo;
use crate::model::mr::{MRDetail, MrInfoItem};
use crate::protocol::mr::MergeRequest;

#[derive(Clone)]
pub struct MonoApiService {
    pub context: Context,
}

#[async_trait]
impl ApiHandler for MonoApiService {
    fn get_context(&self) -> Context {
        self.context.clone()
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
        let storage = self.context.services.mono_storage.clone();
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
            &format!("create file {} commit", file_info.name),
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
        let storage = self.context.services.mono_storage.clone();
        let refs = storage.get_ref("/").await.unwrap().unwrap();

        storage
            .get_tree_by_hash(&refs.ref_tree_hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_by_hash(&self, hash: &str) -> Tree {
        self.context
            .services
            .mono_storage
            .get_tree_by_hash(hash)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn get_tree_relate_commit(&self, t_hash: &str) -> Commit {
        let storage = self.context.services.mono_storage.clone();
        let tree_info = storage.get_tree_by_hash(t_hash).await.unwrap().unwrap();
        storage
            .get_commit_by_hash(&tree_info.commit_id)
            .await
            .unwrap()
            .unwrap()
            .into()
    }

    async fn add_trees_to_map(
        &self,
        item_to_commit: &mut HashMap<String, String>,
        hashes: Vec<String>,
    ) {
        let storage = self.context.services.mono_storage.clone();
        let trees = storage.get_trees_by_hashes(hashes).await.unwrap();
        for tree in trees {
            item_to_commit.insert(tree.tree_id, tree.commit_id);
        }
    }

    async fn add_blobs_to_map(
        &self,
        item_to_commit: &mut HashMap<String, String>,
        hashes: Vec<String>,
    ) {
        let storage = self.context.services.mono_storage.clone();
        let blobs = storage.get_mega_blobs_by_hashes(hashes).await.unwrap();
        for blob in blobs {
            item_to_commit.insert(blob.blob_id, blob.commit_id);
        }
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError> {
        let storage = self.context.services.mono_storage.clone();
        let commits = storage.get_commits_by_hashes(&c_hashes).await.unwrap();
        Ok(commits.into_iter().map(|x| x.into()).collect())
    }

    async fn traverse_commit_history(&self, _: &Path, _: Commit, _: TreeItem) -> Commit {
        unreachable!()
    }
}

impl MonoApiService {
    pub async fn init_monorepo(&self) {
        self.context.services.mono_storage.init_monorepo().await
    }

    pub async fn mr_list(&self, status: &str) -> Result<Vec<MrInfoItem>, MegaError> {
        let status = if status == "open" {
            vec![MergeStatus::Open]
        } else if status == "closed" {
            vec![MergeStatus::Closed, MergeStatus::Merged]
        } else {
            vec![MergeStatus::Open, MergeStatus::Closed, MergeStatus::Merged]
            // return Err(MegaError::with_message("Invalid status name"));
        };
        let storage = self.context.services.mono_storage.clone();
        let mr_list = storage.get_mr_by_status(status).await.unwrap();
        Ok(mr_list.into_iter().map(|m| m.into()).collect())
    }

    pub async fn mr_detail(&self, mr_link: &str) -> Result<Option<MRDetail>, MegaError> {
        let storage = self.context.services.mono_storage.clone();
        let model = storage.get_mr(mr_link).await.unwrap();
        if let Some(model) = model {
            let mut detail: MRDetail = model.into();
            let conversions = storage.get_mr_conversations(mr_link).await.unwrap();
            detail.conversions = conversions.into_iter().map(|x| x.into()).collect();
            return Ok(Some(detail));
        }
        Ok(None)
    }

    pub async fn mr_tree_files(&self, mr_link: &str) -> Result<Vec<PathBuf>, MegaError> {
        let storage = self.context.services.mono_storage.clone();
        let model = storage.get_mr(mr_link).await.unwrap();
        if let Some(model) = model {
            let to_tree_id = storage
                .get_commit_by_hash(&model.to_hash)
                .await
                .unwrap()
                .unwrap()
                .tree;

            let from_tree_id = storage
                .get_commit_by_hash(&model.from_hash)
                .await
                .unwrap()
                .unwrap()
                .tree;

            let mut tree_comparation = TreeComparation {
                left_tree: VecDeque::new(),
                result_file: vec![],
            };
            tree_comparation.left_tree.push_back((
                SHA1::from_str(&to_tree_id).unwrap(),
                Some(SHA1::from_str(&from_tree_id).unwrap()),
                PathBuf::from(model.path),
            ));
            while let Some((new_tree, base_tree, mut current_path)) =
                tree_comparation.left_tree.pop_front()
            {
                let new_tree: Tree = storage
                    .get_tree_by_hash(&new_tree.to_plain_str())
                    .await
                    .unwrap()
                    .unwrap()
                    .into();

                let base_tree = if let Some(base_tree) = base_tree {
                    let base_tree: Tree = storage
                        .get_tree_by_hash(&base_tree.to_plain_str())
                        .await
                        .unwrap()
                        .unwrap()
                        .into();
                    Some(base_tree)
                } else {
                    None
                };
                tree_comparation.compare(new_tree, base_tree, &mut current_path);
            }
            return Ok(tree_comparation.result_file);
        }
        Err(MegaError::with_message("Can not find related MR by id"))
    }

    pub async fn merge_mr(&self, mr_link: &str) -> Result<(), MegaError> {
        let storage = self.context.services.mono_storage.clone();
        if let Some(model) = storage.get_open_mr_by_link(mr_link).await.unwrap() {
            let mut mr: MergeRequest = model.into();
            let refs = storage.get_ref(&mr.path).await.unwrap().unwrap();

            if mr.from_hash == refs.ref_commit_hash {
                // update mr
                mr.merge();
                storage.update_mr(mr.clone().into()).await.unwrap();

                let commit: Commit = storage
                    .get_commit_by_hash(&mr.to_hash)
                    .await
                    .unwrap()
                    .unwrap()
                    .into();

                // add conversation
                storage
                    .add_mr_conversation(&mr.mr_link, 0, ConvType::Merged)
                    .await
                    .unwrap();
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
                    // remove refs start with path
                    storage.remove_refs(&mr.path).await.unwrap();
                    // TODO: self.clean_dangling_commits().await;
                }
            } else {
                return Err(MegaError::with_message("ref hash conflict"));
            }
        } else {
            return Err(MegaError::with_message("Invalid mr id"));
        }
        Ok(())
    }

    async fn update_parent_tree(
        &self,
        mut path: PathBuf,
        mut tree_vec: Vec<Tree>,
        commit: Commit,
    ) -> Result<String, GitError> {
        let storage = self.context.services.mono_storage.clone();
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
                    p_commit_id = p_commit.id.to_plain_str();
                    // update p_ref
                    p_ref.ref_commit_hash = p_commit.id.to_plain_str();
                    p_ref.ref_tree_hash = target_hash.to_plain_str();
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
}

pub struct TreeComparation {
    pub left_tree: VecDeque<(SHA1, Option<SHA1>, PathBuf)>,
    pub result_file: Vec<PathBuf>,
}

impl TreeComparation {
    // fn compare(&mut self, new: Tree, base: Option<Tree>) {
    //     let new_set: HashSet<_> = new.tree_items.into_iter().collect();
    //     let base_set = if let Some(tree) = base {
    //         tree.tree_items.into_iter().collect()
    //     } else {
    //         HashSet::new()
    //     };
    //     let diff: Vec<_> = new_set.symmetric_difference(&base_set).cloned().collect();
    //     for item in diff {
    //         if item.mode == TreeItemMode::Tree {
    //             let t_id = item.id.to_plain_str();
    //             if !self.left_tree.contains(&t_id) {
    //                 self.left_tree.push_back(t_id);
    //             }
    //         } else {
    //             self.result_file.push(item.name)
    //         }
    //     }
    // }

    pub fn compare(&mut self, new: Tree, base: Option<Tree>, current_path: &mut PathBuf) {
        let mut map_name_to_item = HashMap::new();
        let mut map_id_to_item = HashMap::new();

        if let Some(tree) = base {
            for item in tree.tree_items {
                map_name_to_item.insert(item.name.clone(), item.clone());
                map_id_to_item.insert(item.id, item.clone());
            }
        }

        for item in new.tree_items {
            match (
                map_name_to_item.get(&item.name),
                map_id_to_item.get(&item.id),
            ) {
                (Some(base_item), _) if base_item.id != item.id => {
                    self.process_diff(item, Some(base_item.id), current_path);
                }
                (_, Some(base_item)) if base_item.name != item.name => {
                    self.process_diff(item, None, current_path);
                }
                (None, None) => {
                    self.process_diff(item, None, current_path);
                }
                _ => {}
            }
        }
    }

    fn process_diff(&mut self, item: TreeItem, base_id: Option<SHA1>, current_path: &mut PathBuf) {
        if item.mode == TreeItemMode::Tree {
            if !self
                .left_tree
                .contains(&(item.id, base_id, current_path.to_path_buf()))
            {
                current_path.push(item.name);
                self.left_tree
                    .push_back((item.id, base_id, current_path.to_path_buf()));
                current_path.pop();
            }
        } else {
            current_path.push(item.name);
            self.result_file.push(current_path.to_path_buf());
            current_path.pop();
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::VecDeque, path::PathBuf};

    use mercury::{
        hash::SHA1,
        internal::object::tree::{Tree, TreeItem, TreeItemMode},
    };

    use crate::api_service::mono_api_service::TreeComparation;

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

    #[test]
    fn test_compare_tree() {
        let base = Tree::from_tree_items(vec![
            TreeItem {
                mode: TreeItemMode::Tree,
                id: SHA1::new(&"src".as_bytes().to_vec()),
                name: "src".to_string(),
            },
            TreeItem {
                mode: TreeItemMode::Tree,
                id: SHA1::new(&"mega".as_bytes().to_vec()),
                name: "mega".to_string(),
            },
            TreeItem {
                mode: TreeItemMode::Blob,
                id: SHA1::new(&"delete.txt".as_bytes().to_vec()),
                name: "delete.txt".to_string(),
            },
            TreeItem {
                mode: TreeItemMode::Blob,
                id: SHA1::new(&"README.md".as_bytes().to_vec()),
                name: "README.md".to_string(),
            },
        ])
        .unwrap();

        let new = Tree::from_tree_items(vec![
            TreeItem {
                mode: TreeItemMode::Tree,
                id: SHA1::new(&"src".as_bytes().to_vec()),
                name: "src_new".to_string(),
            },
            TreeItem {
                mode: TreeItemMode::Tree,
                id: SHA1::new(&"mega222".as_bytes().to_vec()),
                name: "mega".to_string(),
            },
            TreeItem {
                mode: TreeItemMode::Blob,
                id: SHA1::new(&"Cargo.toml".as_bytes().to_vec()),
                name: "Cargo.toml".to_string(),
            },
            TreeItem {
                mode: TreeItemMode::Blob,
                id: SHA1::new(&"README.md".as_bytes().to_vec()),
                name: "README.md".to_string(),
            },
        ])
        .unwrap();

        let mut tree_comparation = TreeComparation {
            left_tree: VecDeque::new(),
            result_file: vec![],
        };
        // let mut current_path = Path::new("/");
        tree_comparation.compare(new, Some(base), &mut PathBuf::from("/"));
        println!(
            "files: {:?} left Tree: {:?}",
            tree_comparation.result_file, tree_comparation.left_tree
        );
    }
}
