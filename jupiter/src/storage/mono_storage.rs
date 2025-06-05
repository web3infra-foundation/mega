use std::sync::{Arc, Mutex};

use futures::{stream, StreamExt};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect,
};

use callisto::{mega_blob, mega_commit, mega_refs, mega_tag, mega_tree, raw_blob};
use common::config::MonoConfig;
use common::errors::MegaError;
use common::utils::{generate_id, MEGA_BRANCH_NAME};
use mercury::internal::object::MegaObjectModel;
use mercury::internal::{object::commit::Commit, pack::entry::Entry};

use crate::storage::batch_save_model;
use crate::utils::converter::MegaModelConverter;

#[derive(Clone)]
pub struct MonoStorage {
    pub connection: Arc<DatabaseConnection>,
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
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        MonoStorage { connection }
    }

    pub fn mock() -> Self {
        MonoStorage {
            connection: Arc::new(DatabaseConnection::default()),
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

    pub async fn get_mr_ref(&self, ref_name: &str) -> Result<Option<mega_refs::Model>, MegaError> {
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
    ) -> Result<(), MegaError> {
        let git_objects = Arc::new(Mutex::new(GitObjects {
            commits: Vec::new(),
            trees: Vec::new(),
            blobs: Vec::new(),
            raw_blobs: Vec::new(),
            tags: Vec::new(),
        }));

        stream::iter(entry_list)
            .for_each_concurrent(None, |entry| {
                let git_objects = git_objects.clone();
                async move {
                    let raw_obj = entry.process_entry();
                    let model = raw_obj.convert_to_mega_model();
                    let mut git_objects = git_objects.lock().unwrap();
                    match model {
                        MegaObjectModel::Commit(commit) => {
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

        batch_save_model(self.get_connection(), git_objects.commits)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), git_objects.trees)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), git_objects.blobs)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), git_objects.raw_blobs)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), git_objects.tags)
            .await
            .unwrap();

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
        batch_save_model(self.get_connection(), mega_trees)
            .await
            .unwrap();
        let mega_blobs = converter.mega_blobs.borrow().values().cloned().collect();
        batch_save_model(self.get_connection(), mega_blobs)
            .await
            .unwrap();
        let raw_blobs = converter.raw_blobs.borrow().values().cloned().collect();
        batch_save_model(self.get_connection(), raw_blobs)
            .await
            .unwrap();
    }

    pub async fn save_mega_commits(&self, commits: Vec<Commit>) -> Result<(), MegaError> {
        let mega_commits: Vec<mega_commit::Model> =
            commits.into_iter().map(mega_commit::Model::from).collect();
        let mut save_models = Vec::new();
        for mega_commit in mega_commits {
            save_models.push(mega_commit.into_active_model());
        }
        batch_save_model(self.get_connection(), save_models)
            .await
            .unwrap();
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
