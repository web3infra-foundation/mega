use std::collections::HashMap;
use std::path::Path;
use std::{collections::HashSet, sync::Arc};

use crate::errors::GitError;
use crate::hash::Hash;
use crate::internal::object::blob::Blob;
use crate::internal::object::commit::Commit;
use crate::internal::object::tree::Tree;
use crate::internal::object::ObjectT;
use crate::internal::pack::encode::pack_encode;
use crate::protocol::{CommandType, PackProtocol, RefCommand};
use anyhow::Result;
use async_recursion::async_recursion;
use common::utils::ZERO_ID;
use database::driver::ObjectStorage;
use entity::{commit, git_obj, refs};
use sea_orm::ActiveValue::NotSet;
use sea_orm::Set;

use super::nodes::Repo;

impl PackProtocol {
    /// Asynchronously retrieves the full pack data for the specified repository path.
    /// This function collects commits and nodes from the storage and packs them into
    /// a single binary vector. There is no need to build the entire tree; the function
    /// only sends all the data related to this repository.
    ///
    /// # Arguments
    /// * `repo_path` - The path to the repository.
    ///
    /// # Returns
    /// * `Result<Vec<u8>, GitError>` - The packed binary data as a vector of bytes.
    ///
    pub async fn get_full_pack_data(&self, repo_path: &Path) -> Result<Vec<u8>, GitError> {
        let mut hash_object: HashMap<Hash, Arc<dyn ObjectT>> = HashMap::new();
        let commit_models = self
            .storage
            .get_all_commits_by_path(repo_path.to_str().unwrap())
            .await
            .unwrap();
        commit_models.iter().for_each(|model| {
            let mut commit = Commit::new_from_data(model.meta.clone());
            let hash = Hash::new_from_str(&model.git_id);
            commit.set_hash(hash);
            hash_object.insert(hash, Arc::new(commit));
        });
        let blob_and_tree = self.storage.get_node_by_path(repo_path).await.unwrap();
        let git_ids = blob_and_tree
            .iter()
            .map(|model| model.git_id.clone())
            .collect();
        // may take lots of time
        let obj_datas = self.storage.get_obj_data_by_ids(git_ids).await.unwrap();
        obj_datas.iter().for_each(|model| {
            let hash = Hash::new_from_str(&model.git_id);
            let obj: Arc<dyn ObjectT> = match model.object_type.as_str() {
                "blob" => {
                    let mut blob = Blob::new_from_data(model.data.clone());
                    blob.set_hash(hash);
                    Arc::new(blob)
                }
                "tree" => {
                    let mut tree = Tree::new_from_data(model.data.clone());
                    tree.set_hash(hash);
                    Arc::new(tree)
                }
                _ => panic!("not supported node type: {}", model.object_type),
            };
            hash_object.insert(hash, obj);
        });
        let meta_vec: Vec<Arc<dyn ObjectT>> = hash_object.into_values().collect();
        let result: Vec<u8> = pack_encode(meta_vec).unwrap();
        Ok(result)
    }

    pub async fn get_incremental_pack_data(
        &self,
        repo_path: &Path,
        want: &HashSet<String>,
        _have: &HashSet<String>,
    ) -> Result<Vec<u8>, GitError> {
        let mut hash_meta: HashMap<String, Arc<dyn ObjectT>> = HashMap::new();
        let all_commits = self
            .storage
            .get_all_commits_by_path(repo_path.to_str().unwrap())
            .await
            .unwrap();

        for c_data in all_commits {
            if want.contains(&c_data.git_id) {
                let c = Commit::new_from_data(c_data.meta);
                if let Some(root) = self
                    .storage
                    .get_obj_data_by_id(&c.tree_id.to_plain_str())
                    .await
                    .unwrap()
                {
                    // todo: replace cache;
                    get_child_trees(&root, &mut hash_meta, &HashMap::new()).await
                } else {
                    return Err(GitError::InvalidTreeObject(c.tree_id.to_plain_str()));
                };
            }
        }
        // todo: add encode process
        let result: Vec<u8> = vec![];
        // Pack::default().encode(Some(hash_meta.into_values().collect()));
        Ok(result)
    }

    pub async fn get_head_object_id(&self, repo_path: &Path) -> String {
        let path_str = repo_path.to_str().unwrap();
        let refs_list = self.storage.search_refs(path_str).await.unwrap();

        if refs_list.is_empty() {
            ZERO_ID.to_string()
        } else {
            for refs in &refs_list {
                if repo_path.to_str().unwrap() == refs.repo_path {
                    return refs.ref_git_id.clone();
                }
            }
            for refs in &refs_list {
                // if repo_path is subdirectory of some commit, we should generae a fake commit
                if repo_path.starts_with(refs.repo_path.clone()) {
                    return generate_child_commit_and_refs(self.storage.clone(), refs, repo_path)
                        .await;
                }
            }
            //situation: repo_path: root/repotest2/src, commit: root/repotest
            ZERO_ID.to_string()
        }
    }
}

// retrieve all sub trees recursively
#[async_recursion]
async fn get_child_trees(
    root: &git_obj::Model,
    hash_object: &mut HashMap<String, Arc<dyn ObjectT>>,
    pack_cache: &HashMap<String, git_obj::Model>,
) {
    let t = Tree::new_from_data(root.data.clone());
    let mut child_ids = vec![];
    for item in &t.tree_items {
        if !hash_object.contains_key(&item.id.to_plain_str()) {
            child_ids.push(item.id.to_plain_str());
        }
    }
    for id in child_ids {
        let model = pack_cache.get(&id).unwrap();
        if model.object_type == "tree" {
            get_child_trees(model, hash_object, pack_cache).await;
        } else {
            let blob = Blob::new_from_data(model.data.clone());
            hash_object.insert(model.git_id.clone(), Arc::new(blob));
        }
    }
    let tree = Tree::new_from_data(t.get_raw());
    hash_object.insert(t.id.to_plain_str(), Arc::new(tree));
}

/// Generates a new commit for a subdirectory of the original project directory.
/// Steps:
/// 1. Retrieve the root commit based on the provided reference's Git ID.
/// 2. If a root tree is found by searching for the repository path:
///    a. Construct a child commit using the retrieved root commit and the root tree.
///    b. Save the child commit.
///    c. Obtain the commit ID of the child commit.
///    d. Construct a child reference with the repository path, reference name, commit ID, and other relevant information.
///    e. Save the child reference in the database.
/// 3. Return the commit ID of the child commit if successful; otherwise, return a default ID.
pub async fn generate_child_commit_and_refs(
    storage: Arc<dyn ObjectStorage>,
    refs: &refs::Model,
    repo_path: &Path,
) -> String {
    if let Some(root_tree) = storage.search_root_node_by_path(repo_path).await {
        let root_commit = storage
            .get_commit_by_hash(&refs.ref_git_id.clone())
            .await
            .unwrap()
            .unwrap();
        let child_commit = Commit::build_from_model_and_root(&root_commit, root_tree);
        let child_model = child_commit.convert_to_model(repo_path);
        storage
            .save_commits(vec![child_model.clone()])
            .await
            .unwrap();
        let commit_id = child_commit.id.to_plain_str();
        let child_refs = refs::ActiveModel {
            id: NotSet,
            repo_path: Set(repo_path.to_str().unwrap().to_string()),
            ref_name: Set(refs.ref_name.clone()),
            ref_git_id: Set(commit_id.clone()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        };
        storage.save_refs(vec![child_refs]).await.unwrap();
        commit_id
    } else {
        ZERO_ID.to_string()
    }
}

pub async fn save_packfile(
    storage: Arc<dyn ObjectStorage>,
    mr_id: i64,
    repo_path: &Path,
) -> Result<(), anyhow::Error> {
    let tree_map: HashMap<Hash, Tree> = get_objects_from_mr(storage.clone(), mr_id, "tree").await;
    let blob_map: HashMap<Hash, Blob> = get_objects_from_mr(storage.clone(), mr_id, "blob").await;
    let commit_map: HashMap<Hash, Commit> =
        get_objects_from_mr(storage.clone(), mr_id, "commit").await;
    let repo = Repo {
        storage: storage.clone(),
        mr_id,
        tree_map,
        blob_map,
        repo_path: repo_path.to_path_buf(),
    };
    let commits: Vec<&Commit> = commit_map.values().collect();
    let nodes = repo.build_node_tree(&commits).await.unwrap();
    storage.save_nodes(nodes).await.unwrap();

    let save_models: Vec<commit::ActiveModel> = commits
        .iter()
        .map(|commit| commit.convert_to_model(repo_path))
        .collect();

    storage.save_commits(save_models).await.unwrap();
    Ok(())
}

pub async fn handle_refs(storage: Arc<dyn ObjectStorage>, command: &RefCommand, path: &Path) {
    match command.command_type {
        CommandType::Create => {
            storage
                .save_refs(vec![command.convert_to_model(path.to_str().unwrap())])
                .await
                .unwrap();
        }
        CommandType::Delete => storage.delete_refs(command.old_id.clone(), path).await,
        CommandType::Update => {
            storage
                .update_refs(command.old_id.clone(), command.new_id.clone(), path)
                .await;
        }
    }
}

pub async fn get_objects_from_mr<T: ObjectT>(
    storage: Arc<dyn ObjectStorage>,
    mr_id: i64,
    object_type: &str,
) -> HashMap<Hash, T> {
    let git_ids = storage
        .get_mr_objects_by_type(mr_id, object_type)
        .await
        .unwrap()
        .iter()
        .map(|model| model.git_id.clone())
        .collect();
    let models = storage.get_obj_data_by_ids(git_ids).await.unwrap();

    models
        .iter()
        .map(|model| {
            let mut obj = T::new_from_data(model.data.clone());
            let hash = Hash::new_from_str(&model.git_id);
            obj.set_hash(hash);
            (hash, obj)
        })
        .collect()
}
