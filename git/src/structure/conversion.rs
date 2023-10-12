use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::{collections::HashSet, sync::Arc};

use super::nodes::NodeBuilder;
use crate::errors::GitError;
use crate::hash::Hash;
use crate::internal::object::blob::Blob;
use crate::internal::object::commit::Commit;
use crate::internal::object::tree::Tree;
use crate::internal::object::ObjectT;
use crate::internal::pack::encode::pack_encode;
use crate::protocol::PackProtocol;
use anyhow::Result;
use async_recursion::async_recursion;
use common::utils::ZERO_ID;
use database::driver::ObjectStorage;
use entity::{git_obj, refs, repo_directory};
use sea_orm::ActiveValue::NotSet;
use sea_orm::Set;

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
        commit_models.into_iter().for_each(|model| {
            let commit: Commit = model.into();
            hash_object.insert(commit.id, Arc::new(commit));
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

        for model in all_commits {
            let commit_id = model.git_id.clone();
            if want.contains(&commit_id) {
                let c: Commit = model.into();
                if let Some(root) = self
                    .storage
                    .get_obj_data_by_id(&c.tree_id.to_plain_str())
                    .await
                    .unwrap()
                {
                    get_child_trees(&root, &mut hash_meta, self.storage.clone()).await
                } else {
                    return Err(GitError::InvalidTreeObject(c.tree_id.to_plain_str()));
                };
                hash_meta.insert(commit_id, Arc::new(c));
            }
        }
        let meta_vec: Vec<Arc<dyn ObjectT>> = hash_meta.into_values().collect();
        let result: Vec<u8> = pack_encode(meta_vec).unwrap();
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

    // TODO: Consider the scenario of deleting a repo
    pub async fn handle_directory(&self) -> Result<(), GitError> {
        let path = self.path.clone();
        let repo_name = path.file_name().unwrap();
        let mut current_path = PathBuf::new();
        let mut pid = Option::default();

        for component in path.components() {
            current_path.push(component);
            if let Component::Normal(dir) = component {
                if let Some(dir_str) = dir.to_str() {
                    let repo_dir = self
                        .storage
                        .get_directory_by_full_path(current_path.to_str().unwrap())
                        .await
                        .unwrap();
                    match repo_dir {
                        Some(dir) => {
                            pid = Some(dir.id);
                        }
                        None => {
                            let inserted_pid = self
                                .storage
                                .save_directory(repo_directory::ActiveModel {
                                    id: NotSet,
                                    pid: match pid {
                                        Some(id) => Set(id),
                                        None => NotSet,
                                    },
                                    name: Set(dir_str.to_owned()),
                                    is_repo: Set(repo_name == dir_str),
                                    full_path: Set(current_path.to_str().unwrap().to_owned()),
                                    created_at: Set(chrono::Utc::now().naive_utc()),
                                    updated_at: Set(chrono::Utc::now().naive_utc()),
                                })
                                .await
                                .unwrap();
                            pid = Some(inserted_pid);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// retrieve all sub trees recursively
#[async_recursion]
async fn get_child_trees(
    root: &git_obj::Model,
    hash_object: &mut HashMap<String, Arc<dyn ObjectT>>,
    storage: Arc<dyn ObjectStorage>,
) {
    let t = Tree::new_from_data(root.data.clone());
    let mut search_child_ids = vec![];
    for item in &t.tree_items {
        if !hash_object.contains_key(&item.id.to_plain_str()) {
            search_child_ids.push(item.id.to_plain_str());
        }
    }
    let objs = storage.get_obj_data_by_ids(search_child_ids).await.unwrap();
    for obj in objs {
        if obj.object_type == "tree" {
            get_child_trees(&obj, hash_object, storage.clone()).await;
        } else {
            let blob = Blob::new_from_data(obj.data.clone());
            hash_object.insert(obj.git_id.clone(), Arc::new(blob));
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
        let root_commit_obj = storage
            .get_obj_data_by_id(&refs.ref_git_id.clone())
            .await
            .unwrap()
            .unwrap();

        let child_commit = Commit::build_from_model_and_root(root_commit_obj.data, root_tree);
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

pub async fn save_node_from_mr(
    storage: Arc<dyn ObjectStorage>,
    mr_id: i64,
    repo_path: &Path,
) -> Result<(), anyhow::Error> {
    let tree_map: HashMap<Hash, Tree> = get_objects_from_mr(storage.clone(), mr_id, "tree").await;
    let blob_map: HashMap<Hash, Blob> = get_objects_from_mr(storage.clone(), mr_id, "blob").await;
    let commits: Vec<Commit> = get_objects_vec_from_mr(storage.clone(), mr_id, "commit").await;
    let builder = NodeBuilder {
        storage: storage.clone(),
        tree_map,
        blob_map,
        repo_path: repo_path.to_path_buf(),
        commits,
    };
    let nodes = builder.build_node_tree().await.unwrap();
    builder.save_nodes(nodes).await.unwrap();
    builder.save_commits().await.unwrap();
    Ok(())
}

pub async fn save_node_from_git_obj(
    storage: Arc<dyn ObjectStorage>,
    repo_path: &Path,
    git_objs: Vec<git_obj::Model>,
) -> Result<(), anyhow::Error> {
    // let mut model_vec_map: HashMap<String, Vec<git_obj::Model>> = HashMap::new();
    // for (key, group) in &git_objs
    //     .into_iter()
    //     .group_by(|model| model.object_type.clone())
    // {
    //     let model_vec: Vec<git_obj::Model> = group.collect();
    //     tracing::info!("key {:?}, model_vec {:?}", key, model_vec);
    //     model_vec_map.insert(key.to_owned(), model_vec);
    // }
    let mut tree_vec: Vec<git_obj::Model> = Vec::new();
    let mut blob_vec: Vec<git_obj::Model> = Vec::new();
    let mut commit_vec: Vec<git_obj::Model> = Vec::new();
    for obj in git_objs.clone() {
        match obj.object_type.as_str() {
            "tree" => tree_vec.push(obj.clone()),
            "blob" => blob_vec.push(obj.clone()),
            "commit" => commit_vec.push(obj.clone()),
            _ => {}
        }
    }
    let tree_map: HashMap<Hash, Tree> = convert_model_to_map(tree_vec);
    let blob_map: HashMap<Hash, Blob> = convert_model_to_map(blob_vec);
    let commit_map: HashMap<Hash, Commit> = convert_model_to_map(commit_vec);
    let commits: Vec<Commit> = commit_map.values().map(|x| x.to_owned()).collect();

    //save git_obj
    let git_obj_active_model = git_objs
        .iter()
        .map(|m| git_obj::ActiveModel {
            id: Set(m.id),
            git_id: Set(m.git_id.clone()),
            object_type: Set(m.object_type.clone()),
            data: Set(m.data.clone()),
        })
        .collect();
    storage.save_obj_data(git_obj_active_model).await.unwrap();

    let repo = NodeBuilder {
        storage: storage.clone(),
        tree_map,
        blob_map,
        repo_path: repo_path.to_path_buf(),
        commits: commits.clone(),
    };
    let nodes = repo.build_node_tree().await.unwrap();
    repo.save_nodes(nodes).await.unwrap();
    repo.save_commits().await.unwrap();

    // save refs
    // to do, if it is an incremental update, this code will not apply
    let mut commit_id = String::new();
    let mut parent_id_list: Vec<String> = Vec::new();
    for commit in commits.clone() {
        let mut p_list: Vec<String> = commit
            .parent_tree_ids
            .iter()
            .map(|x| x.to_plain_str())
            .collect();
        parent_id_list.append(&mut p_list);
    }
    for commit in commits {
        if !parent_id_list.contains(&commit.id.to_plain_str()) {
            commit_id = commit.id.to_plain_str();
        }
    }

    let mut refs = storage
        .get_ref_object_id(repo_path.to_str().unwrap())
        .await
        .unwrap();
    if refs.is_empty() {
        let child_refs = refs::ActiveModel {
            id: NotSet,
            repo_path: Set(repo_path.to_str().unwrap().to_string()),
            ref_name: Set(String::from("refs/heads/master")),
            ref_git_id: Set(commit_id.clone()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        };
        storage.save_refs(vec![child_refs]).await.unwrap();
    } else if let Some(r) = refs.pop() {
        storage
            .update_refs(r.ref_git_id, commit_id.clone(), repo_path)
            .await;
    }

    Ok(())
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
    convert_model_to_map(models)
}

pub fn convert_model_to_map<T: ObjectT>(models: Vec<git_obj::Model>) -> HashMap<Hash, T> {
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

pub async fn get_objects_vec_from_mr<T: ObjectT>(
    storage: Arc<dyn ObjectStorage>,
    mr_id: i64,
    object_type: &str,
) -> Vec<T> {
    let git_ids = storage
        .get_mr_objects_by_type(mr_id, object_type)
        .await
        .unwrap()
        .iter()
        .map(|model| model.git_id.clone())
        .collect();
    let models = storage.get_obj_data_by_ids(git_ids).await.unwrap();
    let result = models
        .iter()
        .map(|model| {
            let mut obj = T::new_from_data(model.data.clone());
            let hash = Hash::new_from_str(&model.git_id);
            obj.set_hash(hash);
            obj
        })
        .collect();
    result
}
