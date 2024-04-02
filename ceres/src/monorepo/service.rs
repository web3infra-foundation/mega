use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use callisto::db_enums::ConvType;
use callisto::{mega_commit, mega_tree};
use common::errors::MegaError;
use ganymede::model::create_file::CreateFileInfo;
use jupiter::storage::batch_save_model;
use jupiter::storage::mega_storage::MegaStorage;
use venus::errors::GitError;
use venus::hash::SHA1;
use venus::internal::object::commit::Commit;
use venus::internal::object::tree::Tree;
use venus::monorepo::mr::{MergeOperation, MergeResult};
use venus::repo::Repo;

#[derive(Clone)]
pub struct MonorepoService {
    pub storage: Arc<MegaStorage>,
}

impl MonorepoService {
    pub async fn init_monorepo(&self) {
        self.storage.init_monorepo().await
    }

    pub async fn create_mega_file(&self, file_info: CreateFileInfo) -> Result<(), MegaError> {
        self.storage.create_mega_file(file_info).await
    }

    pub async fn merge_mr(&self, op: MergeOperation) -> Result<MergeResult, MegaError> {
        let mut res = MergeResult {
            result: true,
            err_message: "".to_owned(),
        };
        if let Some(mut mr) = self.storage.get_open_mr_by_id(op.mr_id).await.unwrap() {
            let mut refs = self.storage.get_ref(&mr.path).await.unwrap().unwrap();

            if mr.from_hash == refs.ref_commit_hash {
                // update mr
                mr.merge(op.message);
                self.storage.update_mr(mr.clone()).await.unwrap();

                // update refs
                let ref_commit = mr.to_hash;
                let commit = self
                    .storage
                    .get_commit_by_hash(&Repo::empty(), &ref_commit)
                    .await
                    .unwrap()
                    .unwrap();
                refs.ref_commit_hash = ref_commit;
                refs.ref_tree_hash = commit.tree.clone();
                self.storage.update_ref(refs).await.unwrap();

                // add conversation
                self.storage
                    .add_mr_conversation(mr.id, 0, ConvType::Merged)
                    .await
                    .unwrap();
                if mr.path != "/" {
                    self.handle_parent_directory(&PathBuf::from(mr.path), &commit.tree)
                        .await
                        .unwrap();
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

    async fn handle_parent_directory(
        &self,
        path: &Path,
        path_tree_hash: &str,
    ) -> Result<(), GitError> {
        let refs = self.storage.get_ref("/").await.unwrap().unwrap();

        let mut save_trees: Vec<mega_tree::ActiveModel> = Vec::new();
        let mut save_commits: Vec<mega_commit::ActiveModel> = Vec::new();

        let handle_path = path.parent().unwrap().to_owned();
        let root_tree: Tree = self
            .storage
            .get_tree_by_hash(&Repo::empty(), &refs.ref_tree_hash)
            .await
            .unwrap()
            .unwrap()
            .into();
        let mut search_tree = root_tree.clone();
        let mut tree_vec = vec![root_tree];
        for component in handle_path.components() {
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
                        .get_tree_by_hash(&Repo::empty(), &hash)
                        .await
                        .unwrap()
                        .unwrap()
                        .into();
                    search_tree = res.clone();
                    tree_vec.push(res);
                } else {
                    return Err(GitError::ConversionError(
                        "can't find target parent tree under latest commit, you should update your local repository".to_string(),
                    ));
                }
            }
        }

        let mut target_hash = SHA1::from_str(path_tree_hash).unwrap();

        let mut full_path = PathBuf::from(path);
        while let Some(mut tree) = tree_vec.pop() {
            let cloned_path = full_path.clone();
            let name = cloned_path.file_name().unwrap().to_str().unwrap();
            full_path.pop();

            let index = tree.tree_items.iter().position(|x| x.name == name).unwrap();
            tree.tree_items[index].id = target_hash;
            let new_tree = Tree::from_tree_items(tree.tree_items).unwrap();
            target_hash = new_tree.id;

            let model: mega_tree::Model = new_tree.into();
            let a_model = model.into();
            save_trees.push(a_model);

            let p_ref = self.storage.get_ref(full_path.to_str().unwrap()).await.unwrap();
            if let Some(mut p_ref) = p_ref {
                // generate commit
                let p_commit = Commit::from_tree_id(
                    target_hash,
                    vec![SHA1::from_str(&p_ref.ref_commit_hash).unwrap()],
                    "This Commit was generate for handle parent directory",
                );
                // update p_ref
                p_ref.ref_commit_hash = p_commit.id.to_plain_str();
                p_ref.ref_tree_hash = target_hash.to_plain_str();
                self.storage.update_ref(p_ref).await.unwrap();

                let model: mega_commit::Model = p_commit.into();
                save_commits.push(model.into());
            }
        }
        batch_save_model(self.storage.get_connection(), save_trees)
            .await
            .unwrap();
        batch_save_model(self.storage.get_connection(), save_commits)
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