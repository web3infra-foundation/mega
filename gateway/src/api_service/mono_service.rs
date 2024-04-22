use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use axum::async_trait;

use callisto::db_enums::ConvType;
use callisto::{mega_blob, mega_tree, raw_blob};
use common::errors::MegaError;
use ganymede::model::converter;
use ganymede::model::create_file::CreateFileInfo;
use jupiter::storage::batch_save_model;
use jupiter::storage::mega_storage::MegaStorage;
use venus::errors::GitError;
use venus::hash::SHA1;
use venus::internal::object::blob::Blob;
use venus::internal::object::commit::Commit;
use venus::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use venus::monorepo::mr::{MergeOperation, MergeResult};

use crate::model::objects::{LatestCommitInfo, TreeCommitInfo};

use super::ApiHandler;

#[derive(Clone)]
pub struct MonorepoService {
    pub storage: Arc<MegaStorage>,
}

#[async_trait]
impl ApiHandler for MonorepoService {
    async fn get_latest_commit(
        &self,
    ) -> Result<LatestCommitInfo, GitError> {
        todo!()
    }

    async fn get_tree_commit_info(&self) -> Result<TreeCommitInfo, GitError> {
        unimplemented!()
    }
}

impl MonorepoService {
    pub async fn init_monorepo(&self) {
        self.storage.init_monorepo().await
    }

    pub async fn create_mega_file(&self, file_info: CreateFileInfo) -> Result<(), GitError> {
        let path = PathBuf::from(file_info.path);

        let new_item = if file_info.is_directory {
            let blob = converter::generate_git_keep();
            let tree_item = TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: String::from(".gitkeep"),
            };
            let child_tree = Tree::from_tree_items(vec![tree_item]).unwrap();
            TreeItem {
                mode: TreeItemMode::Tree,
                id: child_tree.id,
                name: file_info.name.clone(),
            }
        } else {
            let blob = Blob::from_content(&file_info.content.unwrap());
            let mega_blob: mega_blob::Model = blob.clone().into();
            let mega_blob: mega_blob::ActiveModel = mega_blob.into();
            let raw_blob: raw_blob::Model = blob.clone().into();
            let raw_blob: raw_blob::ActiveModel = raw_blob.into();
            batch_save_model(self.storage.get_connection(), vec![mega_blob])
                .await
                .unwrap();
            batch_save_model(self.storage.get_connection(), vec![raw_blob])
                .await
                .unwrap();
            TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: file_info.name.clone(),
            }
        };

        let (tree_vec, search_tree) = self.search_tree_by_path(&path).await.unwrap();

        let mut t_items = search_tree.tree_items;
        t_items.push(new_item);
        let new_tree = Tree::from_tree_items(t_items).unwrap();

        let refs = self.storage.get_ref("/").await.unwrap().unwrap();
        let commit = Commit::from_tree_id(
            new_tree.id,
            vec![SHA1::from_str(&refs.ref_commit_hash).unwrap()],
            &format!("create file {} commit", file_info.name),
        );

        let tree_model: mega_tree::Model = new_tree.into();
        let tree_model: mega_tree::ActiveModel = tree_model.into();

        batch_save_model(self.storage.get_connection(), vec![tree_model])
            .await
            .unwrap();

        self.update_parent_tree(path, tree_vec, commit)
            .await
            .unwrap();

        Ok(())
    }

    pub async fn merge_mr(&self, op: MergeOperation) -> Result<MergeResult, MegaError> {
        let mut res = MergeResult {
            result: true,
            err_message: "".to_owned(),
        };
        if let Some(mut mr) = self.storage.get_open_mr_by_id(op.mr_id).await.unwrap() {
            let refs = self.storage.get_ref(&mr.path).await.unwrap().unwrap();

            if mr.from_hash == refs.ref_commit_hash {
                // update mr
                mr.merge(op.message);
                self.storage.update_mr(mr.clone()).await.unwrap();

                let commit: Commit = self
                    .storage
                    .get_commit_by_hash(&mr.to_hash)
                    .await
                    .unwrap()
                    .unwrap()
                    .into();

                // add conversation
                self.storage
                    .add_mr_conversation(mr.id, 0, ConvType::Merged)
                    .await
                    .unwrap();
                if mr.path != "/" {
                    let path = PathBuf::from(mr.path.clone());

                    let (tree_vec, _) = self.search_tree_by_path(&path).await.unwrap();
                    self.update_parent_tree(path, tree_vec, commit)
                        .await
                        .unwrap();
                    // remove refs start with path
                    self.storage.remove_refs(&mr.path).await.unwrap();
                    // todo: self.clean_dangling_commits().await;
                }
            } else {
                res.result = false;
                res.err_message = "ref hash conflict".to_owned();
            }
        } else {
            res.result = false;
            res.err_message = "Invalid mr id".to_owned();
        }
        Ok(res)
    }

    async fn search_tree_by_path(&self, path: &Path) -> Result<(Vec<Tree>, Tree), GitError> {
        let refs = self.storage.get_ref("/").await.unwrap().unwrap();

        let path_parent = path.parent().unwrap().to_owned();
        let root_tree: Tree = self
            .storage
            .get_tree_by_hash(&refs.ref_tree_hash)
            .await
            .unwrap()
            .unwrap()
            .into();
        let mut search_tree = root_tree.clone();
        let mut tree_vec = vec![root_tree];
        for component in path_parent.components() {
            if component != Component::RootDir {
                let target_name = component.as_os_str().to_str().unwrap();
                let search_res = search_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == target_name);

                if let Some(search_res) = search_res {
                    let hash = search_res.id.to_plain_str();
                    let res: Tree = self
                        .storage
                        .get_tree_by_hash(&hash)
                        .await
                        .unwrap()
                        .unwrap()
                        .into();
                    search_tree = res.clone();
                    tree_vec.push(res);
                } else {
                    return Err(GitError::ConversionError(
                        "can't find target parent tree under latest commit".to_string(),
                    ));
                }
            }
        }
        Ok((tree_vec, search_tree))
    }

    async fn update_parent_tree(
        &self,
        mut path: PathBuf,
        mut tree_vec: Vec<Tree>,
        commit: Commit,
    ) -> Result<(), GitError> {
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

            let p_ref = self.storage.get_ref(path.to_str().unwrap()).await.unwrap();
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
                    self.storage.update_ref(p_ref).await.unwrap();
                    self.storage
                        .save_mega_commits(vec![p_commit])
                        .await
                        .unwrap();
                } else {
                    self.storage.remove_ref(p_ref).await.unwrap();
                }
            }
        }
        let save_trees: Vec<mega_tree::ActiveModel> = save_trees
            .into_iter()
            .map(|mut x| {
                x.commit_id = p_commit_id.to_owned();
                x.into()
            })
            .collect();

        batch_save_model(self.storage.get_connection(), save_trees)
            .await
            .unwrap();
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
