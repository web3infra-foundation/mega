use std::collections::HashMap;
use std::path::{Component, Components, Path, PathBuf};
use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use async_recursion::async_recursion;
use itertools::Itertools;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{DbErr, Set, TransactionTrait};

use common::utils::ZERO_ID;
use entity::{objects, refs, repo_directory};
use storage::driver::database::storage::ObjectStorage;

use crate::errors::GitError;
use crate::hash::Hash;
use crate::internal::object::blob::Blob;
use crate::internal::object::commit::Commit;
use crate::internal::object::tag::Tag;
use crate::internal::object::tree::Tree;
use crate::internal::object::ObjectT;
use crate::internal::pack::encode::pack_encode;
use crate::protocol::PackProtocol;
use crate::structure::nodes::NodeBuilder;

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
        // container for reserve all commit,blob and tree objs
        let mut hash_meta: HashMap<Hash, Arc<dyn ObjectT>> = HashMap::new();
        let all_commits: Vec<Commit> = self
            .storage
            .get_all_commits_by_path(repo_path.to_str().unwrap())
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.into())
            .collect();
        let all_tree_ids = all_commits
            .iter()
            .map(|c| c.tree_id.to_plain_str())
            .collect();
        let all_trees: HashMap<String, objects::Model> = self
            .storage
            .get_obj_data_by_ids(all_tree_ids)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (m.git_id.clone(), m))
            .collect();
        for c in all_commits {
            self.traverse_want_trees(
                all_trees.get(&c.tree_id.to_plain_str()).unwrap(),
                &mut hash_meta,
                &HashSet::new(),
            )
            .await;
            hash_meta.insert(c.id, Arc::new(c));
        }

        let tag_ids = self
            .storage
            .get_all_refs_by_path(repo_path.to_str().unwrap())
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.ref_git_id)
            .collect_vec();
        self.get_all_tags(tag_ids, &mut hash_meta).await;

        let meta_vec: Vec<Arc<dyn ObjectT>> = hash_meta.into_values().collect();
        let result: Vec<u8> = pack_encode(meta_vec).unwrap();
        Ok(result)
    }

    pub async fn get_incremental_pack_data(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<Vec<u8>, GitError> {
        let mut hash_meta: HashMap<Hash, Arc<dyn ObjectT>> = HashMap::new();
        let mut commit_id = String::new();
        let exist_want_objs = self.storage.get_obj_data_by_ids(want).await.unwrap();
        for obj in exist_want_objs {
            match obj.object_type.as_str() {
                "commit" => {
                    if commit_id.is_empty() {
                        commit_id = obj.git_id;
                    } else {
                        panic!("only single commit id in want supported!")
                    }
                }
                "tag" => {
                    let tag: Tag = obj.into();
                    hash_meta.insert(tag.id, Arc::new(tag));
                }
                other_type => panic!("want objetcs type invalid: {}!", other_type),
            }
        }

        let mut exist_objs = HashSet::new();
        let repo_path = self.path.to_str().unwrap();

        let commit: Commit = self
            .storage
            .get_commit_by_hash(&commit_id, repo_path)
            .await
            .unwrap()
            .unwrap()
            .into();
        let mut traversal_list: Vec<Commit> = vec![commit.clone()];
        let mut want_commits: Vec<Commit> = vec![commit];

        // tarverse commit's all parents to find the commit that client does not have
        while let Some(temp) = traversal_list.pop() {
            for p_commit_id in temp.parent_commit_ids {
                let p_commit_id = &p_commit_id.to_plain_str();

                let want_commit_ids: Vec<String> =
                    want_commits.iter().map(|x| x.id.to_plain_str()).collect();

                if !have.contains(p_commit_id) && !want_commit_ids.contains(p_commit_id) {
                    let parent: Commit = self
                        .storage
                        .get_commit_by_hash(p_commit_id, repo_path)
                        .await
                        .unwrap()
                        .unwrap()
                        .into();
                    want_commits.push(parent.clone());
                    traversal_list.push(parent);
                }
            }
        }

        let want_tree_ids = want_commits
            .iter()
            .map(|c| c.tree_id.to_plain_str())
            .collect();
        let want_trees: HashMap<String, objects::Model> = self
            .storage
            .get_obj_data_by_ids(want_tree_ids)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (m.git_id.clone(), m))
            .collect();

        for c in want_commits {
            let have_commit_hashes: Vec<String> = c
                .parent_commit_ids
                .clone()
                .into_iter()
                .filter(|p_id| have.contains(&p_id.to_plain_str()))
                .map(|hash| hash.to_plain_str())
                .collect();
            let have_commits = self
                .storage
                .get_commit_by_hashes(have_commit_hashes, self.path.to_str().unwrap())
                .await
                .unwrap();

            for have_c in have_commits {
                let have_tree = self
                    .storage
                    .get_obj_data_by_id(&have_c.tree)
                    .await
                    .unwrap()
                    .unwrap();
                self.add_to_exist_objs(&have_tree, &mut exist_objs).await;
            }

            self.traverse_want_trees(
                want_trees.get(&c.tree_id.to_plain_str()).unwrap(),
                &mut hash_meta,
                &exist_objs,
            )
            .await;
            hash_meta.insert(c.id, Arc::new(c));
        }

        let meta_vec: Vec<Arc<dyn ObjectT>> = hash_meta.into_values().collect();
        let result: Vec<u8> = pack_encode(meta_vec).unwrap();
        Ok(result)
    }

    pub async fn get_all_tags(
        &self,
        tag_ids: Vec<String>,
        hash_meta: &mut HashMap<Hash, Arc<dyn ObjectT>>,
    ) {
        let tag_objs: Vec<Tag> = self
            .storage
            .get_obj_data_by_ids(tag_ids)
            .await
            .unwrap()
            .into_iter()
            .filter(|o| o.object_type == "tag")
            .map(|o| o.into())
            .collect();
        for tag in tag_objs {
            hash_meta.insert(tag.id, Arc::new(tag));
        }
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
                    return self.generate_subdir_commit(refs, repo_path).await;
                }
            }
            //situation: repo_path: root/repotest2/src, commit: root/repotest
            ZERO_ID.to_string()
        }
    }

    // get all objects id from have tree
    #[async_recursion]
    async fn add_to_exist_objs(&self, have_tree: &objects::Model, exist_objs: &mut HashSet<Hash>) {
        let mut t = Tree::new_from_data(have_tree.data.clone());
        t.set_hash(Hash::new_from_str(&have_tree.git_id));

        let mut search_child_ids = vec![];
        for item in &t.tree_items {
            if !exist_objs.contains(&item.id) {
                search_child_ids.push(item.id.to_plain_str());
            }
        }
        let objs = self
            .storage
            .get_obj_data_by_ids(search_child_ids)
            .await
            .unwrap();
        for obj in objs {
            if obj.object_type == "tree" {
                self.add_to_exist_objs(&obj, exist_objs).await;
            } else {
                let blob_id = Hash::new_from_str(&obj.git_id.clone());
                exist_objs.insert(blob_id);
            }
        }
        exist_objs.insert(t.id);
    }

    // retrieve all sub trees recursively
    #[async_recursion]
    async fn traverse_want_trees(
        &self,
        want_t: &objects::Model,
        all_objects: &mut HashMap<Hash, Arc<dyn ObjectT>>,
        exist_objs: &HashSet<Hash>,
    ) {
        let mut t = Tree::new_from_data(want_t.data.clone());
        t.set_hash(Hash::new_from_str(&want_t.git_id));

        let mut search_child_ids = vec![];
        for item in &t.tree_items {
            if !all_objects.contains_key(&item.id) && !exist_objs.contains(&item.id) {
                search_child_ids.push(item.id.to_plain_str());
            }
        }
        let objs = self
            .storage
            .get_obj_data_by_ids(search_child_ids)
            .await
            .unwrap();
        for obj in objs {
            if obj.object_type == "tree" {
                self.traverse_want_trees(&obj, all_objects, exist_objs)
                    .await;
            } else {
                let mut blob = Blob::new_from_data(obj.data.clone());
                let blob_id = Hash::new_from_str(&obj.git_id.clone());
                blob.set_hash(blob_id);
                all_objects.insert(blob_id, Arc::new(blob));
            }
        }
        all_objects.insert(t.id, Arc::new(t));
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
    pub async fn generate_subdir_commit(&self, refs: &refs::Model, repo_path: &Path) -> String {
        let root_commit: Commit = self
            .storage
            .get_commit_by_hash(&refs.ref_git_id.clone(), &refs.repo_path)
            .await
            .unwrap()
            .unwrap()
            .into();

        let relative_path = PathBuf::from(&repo_path.to_str().unwrap()[refs.repo_path.len()..]);
        let mut comp = relative_path.components();
        // skip the first root dir
        comp.next();
        let t_id = self
            .search_dir_from_tree(&root_commit.tree_id.to_plain_str(), comp)
            .await;

        let child_commit =
            Commit::subdir_commit(root_commit.to_data().unwrap(), Hash::new_from_str(&t_id));
        let child_c_model = child_commit.convert_to_model(repo_path);
        self.storage
            .save_commits(None, vec![child_c_model.clone()])
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
        self.storage.save_refs(vec![child_refs]).await.unwrap();
        commit_id
    }

    // find search_dir's tree id from a provided tree
    #[async_recursion]
    pub async fn search_dir_from_tree<'a>(
        &self,
        tree_id: &str,
        mut relative_path: Components<'async_recursion>,
    ) -> String {
        let root_tree: Tree = self
            .storage
            .get_obj_data_by_id(tree_id)
            .await
            .unwrap()
            .unwrap()
            .into();
        if let Some(Component::Normal(search_dir)) = relative_path.next() {
            let t_id = root_tree
                .tree_items
                .iter()
                .find(|item| item.name == search_dir.to_str().unwrap())
                .unwrap()
                .id
                .to_plain_str();
            self.search_dir_from_tree(&t_id, relative_path).await
        } else {
            root_tree.id.to_plain_str()
        }
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
    storage
        .get_connection()
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                builder.save_nodes(Some(txn), nodes).await.unwrap();
                builder.save_commits(Some(txn)).await.unwrap();
                Ok(())
            })
        })
        .await?;
    Ok(())
}

pub async fn save_node_from_git_obj(
    storage: Arc<dyn ObjectStorage>,
    repo_path: &Path,
    git_objs: Vec<objects::Model>,
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
    let mut tree_vec: Vec<objects::Model> = Vec::new();
    let mut blob_vec: Vec<objects::Model> = Vec::new();
    let mut commit_vec: Vec<objects::Model> = Vec::new();
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
        .map(|m| objects::ActiveModel {
            id: Set(m.id),
            git_id: Set(m.git_id.clone()),
            object_type: Set(m.object_type.clone()),
            data: Set(m.data.clone()),
            link: Set(m.link.clone()),
        })
        .collect();
    storage
        .save_obj_data(None, git_obj_active_model)
        .await
        .unwrap();

    let repo = NodeBuilder {
        storage: storage.clone(),
        tree_map,
        blob_map,
        repo_path: repo_path.to_path_buf(),
        commits: commits.clone(),
    };
    let nodes = repo.build_node_tree().await.unwrap();
    storage
        .get_connection()
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                repo.save_nodes(Some(txn), nodes).await.unwrap();
                repo.save_commits(Some(txn)).await.unwrap();
                Ok(())
            })
        })
        .await?;

    // save refs
    let mut commit_id = String::new();
    let mut parent_id_list: Vec<String> = Vec::new();
    for commit in commits.clone() {
        let mut p_list: Vec<String> = commit
            .parent_commit_ids
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
        .get_all_refs_by_path(repo_path.to_str().unwrap())
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

pub fn convert_model_to_map<T: ObjectT>(models: Vec<objects::Model>) -> HashMap<Hash, T> {
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
