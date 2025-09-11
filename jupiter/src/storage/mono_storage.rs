use std::ops::Deref;
use std::sync::{Arc, Mutex};

use futures::{stream, StreamExt};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect,
};

use callisto::{mega_blob, mega_commit, mega_refs, mega_tag, mega_tree, raw_blob};
use common::config::MonoConfig;
use common::errors::MegaError;
use common::utils::{generate_id, MEGA_BRANCH_NAME};
use mercury::internal::object::{MegaObjectModel, ObjectTrait};
use mercury::internal::{object::commit::Commit, pack::entry::Entry};

use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::commit_binding_storage::CommitBindingStorage;
use crate::storage::user_storage::UserStorage;
use crate::utils::converter::MegaModelConverter;

#[derive(Clone)]
pub struct MonoStorage {
    pub base: BaseStorage,
}

impl Deref for MonoStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug)]
struct GitObjects {
    pub commits: Vec<mega_commit::ActiveModel>,
    trees: Vec<mega_tree::ActiveModel>,
    blobs: Vec<mega_blob::ActiveModel>,
    raw_blobs: Vec<raw_blob::ActiveModel>,
    tags: Vec<mega_tag::ActiveModel>,
}

impl MonoStorage {
    pub fn user_storage(&self) -> UserStorage {
        UserStorage {
            base: self.base.clone(),
        }
    }

    pub fn commit_binding_storage(&self) -> CommitBindingStorage {
        CommitBindingStorage {
            base: self.base.clone(),
        }
    }

    pub async fn save_ref(
        &self,
        path: &str,
        ref_name: Option<String>,
        ref_commit_hash: &str,
        ref_tree_hash: &str,
        is_mr: bool,
    ) -> Result<(), MegaError> {
        let model = mega_refs::Model {
            id: generate_id(),
            path: path.to_owned(),
            ref_name: ref_name.unwrap_or(MEGA_BRANCH_NAME.to_owned()),
            ref_commit_hash: ref_commit_hash.to_owned(),
            ref_tree_hash: ref_tree_hash.to_owned(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            is_mr,
        };
        model
            .into_active_model()
            .insert(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn remove_none_mr_refs(&self, path: &str) -> Result<(), MegaError> {
        mega_refs::Entity::delete_many()
            .filter(mega_refs::Column::Path.starts_with(path))
            .filter(mega_refs::Column::IsMr.eq(false))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn remove_ref(&self, refs: mega_refs::Model) -> Result<(), MegaError> {
        mega_refs::Entity::delete_by_id(refs.id)
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn get_refs(&self, path: &str) -> Result<Vec<mega_refs::Model>, MegaError> {
        let result = mega_refs::Entity::find()
            .filter(mega_refs::Column::Path.eq(path))
            // .filter(mega_refs::Column::IsMr.eq(false))
            .order_by_asc(mega_refs::Column::RefName)
            .all(self.get_connection())
            .await?;
        Ok(result)
    }

    pub async fn get_ref(&self, path: &str) -> Result<Option<mega_refs::Model>, MegaError> {
        let result = mega_refs::Entity::find()
            .filter(mega_refs::Column::Path.eq(path))
            .filter(mega_refs::Column::RefName.eq(MEGA_BRANCH_NAME.to_owned()))
            .one(self.get_connection())
            .await?;
        Ok(result)
    }

    pub async fn get_ref_by_commit(
        &self,
        path: &str,
        commit: &str,
    ) -> Result<Option<mega_refs::Model>, MegaError> {
        let result = mega_refs::Entity::find()
            .filter(mega_refs::Column::Path.eq(path))
            .filter(mega_refs::Column::RefCommitHash.eq(commit))
            .one(self.get_connection())
            .await?;
        Ok(result)
    }

    pub async fn get_ref_by_name(
        &self,
        ref_name: &str,
    ) -> Result<Option<mega_refs::Model>, MegaError> {
        let res = mega_refs::Entity::find()
            .filter(mega_refs::Column::RefName.eq(ref_name))
            .one(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn update_ref(&self, refs: mega_refs::Model) -> Result<(), MegaError> {
        let mut ref_data: mega_refs::ActiveModel = refs.into();
        ref_data.reset(mega_refs::Column::RefCommitHash);
        ref_data.reset(mega_refs::Column::RefTreeHash);
        ref_data.reset(mega_refs::Column::UpdatedAt);
        ref_data.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn save_entry(
        &self,
        commit_id: &str,
        entry_list: Vec<Entry>,
        authenticated_username: Option<String>,
    ) -> Result<(), MegaError> {
        let git_objects = Arc::new(Mutex::new(GitObjects {
            commits: Vec::new(),
            trees: Vec::new(),
            blobs: Vec::new(),
            raw_blobs: Vec::new(),
            tags: Vec::new(),
        }));

        // Collect commits for binding processing
        let commits_to_process = Arc::new(Mutex::new(Vec::<(String, String)>::new()));

        stream::iter(entry_list)
            .for_each_concurrent(None, |entry| {
                let git_objects = git_objects.clone();
                let commits_to_process = commits_to_process.clone();
                async move {
                    let raw_obj = entry.process_entry();
                    let model = raw_obj.convert_to_mega_model();
                    let mut git_objects = git_objects.lock().unwrap();
                    match model {
                        MegaObjectModel::Commit(commit) => {
                            // Store for binding processing
                            if let Ok(commit_obj) =
                                mercury::internal::object::commit::Commit::from_bytes(
                                    &entry.data,
                                    entry.hash,
                                )
                            {
                                let mut commits = commits_to_process.lock().unwrap();
                                commits.push((
                                    commit_obj.id.to_string(),
                                    commit_obj.author.email.clone(),
                                ));
                            }
                            git_objects.commits.push(commit.into_active_model())
                        }
                        MegaObjectModel::Tree(mut tree) => {
                            commit_id.clone_into(&mut tree.commit_id);
                            git_objects.trees.push(tree.into_active_model());
                        }
                        MegaObjectModel::Blob(mut blob, raw) => {
                            commit_id.clone_into(&mut blob.commit_id);
                            git_objects.blobs.push(blob.clone().into_active_model());
                            git_objects.raw_blobs.push(raw.into_active_model());
                        }
                        MegaObjectModel::Tag(tag) => git_objects.tags.push(tag.into_active_model()),
                    }
                }
            })
            .await;

        let git_objects = Arc::try_unwrap(git_objects)
            .expect("Failed to unwrap Arc")
            .into_inner()
            .unwrap();

        self.batch_save_model(git_objects.commits).await.unwrap();
        self.batch_save_model(git_objects.trees).await.unwrap();
        self.batch_save_model(git_objects.blobs).await.unwrap();
        self.batch_save_model(git_objects.raw_blobs).await.unwrap();
        self.batch_save_model(git_objects.tags).await.unwrap();

        // Process commit author bindings after saving objects
        let commits_to_process = Arc::try_unwrap(commits_to_process)
            .expect("Failed to unwrap Arc")
            .into_inner()
            .unwrap();

        if !commits_to_process.is_empty() {
            self.process_commit_bindings(&commits_to_process, authenticated_username.as_deref())
                .await?;
        }

        Ok(())
    }

    /// Process commit author bindings
    async fn process_commit_bindings(
        &self,
        commits: &[(String, String)],
        authenticated_username: Option<&str>,
    ) -> Result<(), MegaError> {
        let user_storage = self.user_storage();
        let commit_binding_storage = self.commit_binding_storage();

        for (commit_sha, author_email) in commits {
            // Try to find user by authenticated username first
            let matched_username = if let Some(username) = authenticated_username {
                // If authenticated username is available, use it to find user
                match user_storage.find_user_by_name(username).await {
                    Ok(Some(_user)) => {
                        tracing::info!("Found user for username: {}", username);
                        Some(username.to_string())
                    }
                    Ok(None) => {
                        tracing::warn!(
                            "Authenticated username {} not found in user table",
                            username
                        );
                        None
                    }
                    Err(e) => {
                        tracing::error!("Error finding user by username {}: {}", username, e);
                        None
                    }
                }
            } else {
                // No authenticated username, commit will be anonymous
                tracing::info!(
                    "No authenticated username available for commit {}",
                    commit_sha
                );
                None
            };

            let is_anonymous = matched_username.is_none();

            // Save or update binding
            if let Err(e) = commit_binding_storage
                .upsert_binding(commit_sha, author_email, matched_username, is_anonymous)
                .await
            {
                tracing::error!("Failed to save commit binding for {}: {}", commit_sha, e);
                // Continue processing other commits even if one fails
            } else {
                tracing::info!(
                    "Processed binding for commit {} with email {} (anonymous: {})",
                    commit_sha,
                    author_email,
                    is_anonymous
                );
            }
        }
        Ok(())
    }

    pub async fn init_monorepo(&self, mono_config: &MonoConfig) {
        if self.get_ref("/").await.unwrap().is_some() {
            tracing::info!("Monorepo Directory Already Inited, skip init process!");
            return;
        }
        let converter = MegaModelConverter::init(mono_config);
        let commit: mega_commit::Model = converter.commit.into();
        mega_commit::Entity::insert(commit.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap();
        mega_refs::Entity::insert(converter.refs)
            .exec(self.get_connection())
            .await
            .unwrap();

        let mega_trees = converter.mega_trees.borrow().values().cloned().collect();
        self.batch_save_model(mega_trees).await.unwrap();
        let mega_blobs = converter.mega_blobs.borrow().values().cloned().collect();
        self.batch_save_model(mega_blobs).await.unwrap();
        let raw_blobs = converter.raw_blobs.borrow().values().cloned().collect();
        self.batch_save_model(raw_blobs).await.unwrap();
    }

    pub async fn save_mega_commits(&self, commits: Vec<Commit>) -> Result<(), MegaError> {
        let save_models: Vec<mega_commit::ActiveModel> = commits
            .into_iter()
            .map(mega_commit::Model::from)
            .map(|m| m.into_active_model())
            .collect();
        self.batch_save_model(save_models).await.unwrap();
        Ok(())
    }

    pub async fn save_mega_blobs(
        &self,
        blobs: Vec<&Blob>,
        commit_id: &str,
    ) -> Result<(), MegaError> {
        let mega_blobs: Vec<mega_blob::ActiveModel> = blobs
            .clone()
            .into_iter()
            .map(mega_blob::Model::from)
            .map(|mut m| {
                m.commit_id = commit_id.to_owned();
                m.into_active_model()
            })
            .collect();
        self.batch_save_model(mega_blobs).await.unwrap();

        let raw_blobs: Vec<raw_blob::ActiveModel> = blobs
            .into_iter()
            .map(raw_blob::Model::from)
            .map(|m| m.into_active_model())
            .collect();
        self.batch_save_model(raw_blobs).await.unwrap();

        Ok(())
    }

    pub async fn get_commit_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<mega_commit::Model>, MegaError> {
        Ok(mega_commit::Entity::find()
            .filter(mega_commit::Column::CommitId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_commits_by_hashes(
        &self,
        hashes: &Vec<String>,
    ) -> Result<Vec<mega_commit::Model>, MegaError> {
        Ok(mega_commit::Entity::find()
            .filter(mega_commit::Column::CommitId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_tree_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<mega_tree::Model>, MegaError> {
        Ok(mega_tree::Entity::find()
            .filter(mega_tree::Column::TreeId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_trees_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<mega_tree::Model>, MegaError> {
        Ok(mega_tree::Entity::find()
            .filter(mega_tree::Column::TreeId.is_in(hashes))
            .distinct()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_mega_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<mega_blob::Model>, MegaError> {
        Ok(mega_blob::Entity::find()
            .filter(mega_blob::Column::BlobId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }
}

#[cfg(test)]
mod test {}
