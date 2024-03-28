use std::path::{Path, PathBuf};
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
                let ref_hash = &mr.to_hash.clone();
                let commit = self
                    .storage
                    .get_commit_by_hash(&Repo::empty(), ref_hash)
                    .await
                    .unwrap()
                    .unwrap();

                mr.merge(op.message);

                self.storage.update_mr(mr.clone()).await.unwrap();
                refs.ref_commit_hash = ref_hash.to_string();
                refs.ref_tree_hash = commit.tree;
                self.storage.update_ref(refs).await.unwrap();

                self.storage
                    .add_mr_conversation(mr.id, 0, ConvType::Merged)
                    .await
                    .unwrap();
                if mr.path != "/" {
                    self.handle_parent_directory(&PathBuf::from(mr.path))
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

    async fn handle_parent_directory(&self, mut path: &Path) -> Result<(), GitError> {
        let refs = self.storage.get_ref("/").await.unwrap().unwrap();

        let mut target_name = path.file_name().unwrap().to_str().unwrap();
        let mut target_hash = SHA1::from_str(&refs.ref_tree_hash.clone()).unwrap();

        let mut save_trees: Vec<mega_tree::ActiveModel> = Vec::new();
        let mut save_commits: Vec<mega_commit::ActiveModel> = Vec::new();

        while let Some(parent) = path.parent() {
            let parent_path = parent.to_str().unwrap().to_owned();
            let model = self
                .storage
                .get_tree_by_path(&parent_path, &refs.ref_commit_hash)
                .await
                .unwrap();
            if let Some(model) = model {
                let mut p_tree: Tree = model.into();
                let index = p_tree.tree_items.iter().position(|x| x.name == target_name);
                if let Some(index) = index {
                    p_tree.tree_items[index].id = target_hash;
                    let new_p_tree = Tree::from_tree_items(p_tree.tree_items).unwrap();
                    let new_tree_id = new_p_tree.id;
                    if parent.parent().is_some() {
                        target_name = parent.file_name().unwrap().to_str().unwrap();
                        target_hash = new_p_tree.id;
                    } else {
                        target_name = "root";
                    }

                    let mut model: mega_tree::Model = new_p_tree.into();
                    model.full_path = parent_path.clone();
                    model.name = target_name.to_owned();
                    let a_model = model.into();
                    save_trees.push(a_model);
                    let p_ref = self.storage.get_ref(&parent_path).await.unwrap();
                    if let Some(mut p_ref) = p_ref {
                        // generate commit
                        let p_commit = Commit::from_tree_id(
                            new_tree_id,
                            vec![SHA1::from_str(&p_ref.ref_commit_hash).unwrap()],
                            "This Commit was generate for handle parent directory",
                        );
                        // update p_ref
                        p_ref.ref_commit_hash = p_commit.id.to_plain_str();
                        p_ref.ref_tree_hash = new_tree_id.to_plain_str();
                        self.storage.update_ref(p_ref).await.unwrap();

                        let model: mega_commit::Model = p_commit.into();
                        save_commits.push(model.into());
                    }
                } else {
                    return Err(GitError::ConversionError("Can't find child.".to_string()));
                }
            } else {
                return Err(GitError::ConversionError(
                    "Can't find parent tree.".to_string(),
                ));
            }
            path = parent;
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
