use std::sync::{Arc, Mutex};

use futures::{stream, StreamExt};
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect,
};

use callisto::db_enums::{ConvType, MergeStatus};
use callisto::{
    mega_blob, mega_commit, mega_mr, mega_mr_comment, mega_mr_conv, mega_refs, mega_tag, mega_tree,
    raw_blob,
};
use common::config::StorageConfig;
use common::errors::MegaError;
use common::utils::generate_id;
use mercury::internal::object::MegaObjectModel;
use mercury::internal::{object::commit::Commit, pack::entry::Entry};
use venus::monorepo::converter::MegaModelConverter;
use venus::monorepo::mega_refs::MegaRefs;
use venus::monorepo::mr::MergeRequest;

use crate::raw_storage::{self, RawStorage};
use crate::storage::batch_save_model;

#[derive(Clone)]
pub struct MegaStorage {
    pub raw_storage: Arc<dyn RawStorage>,
    pub connection: Arc<DatabaseConnection>,
    pub raw_obj_threshold: usize,
}

#[derive(Debug)]
struct GitObjects {
    pub commits: Vec<mega_commit::ActiveModel>,
    trees: Vec<mega_tree::ActiveModel>,
    blobs: Vec<mega_blob::ActiveModel>,
    raw_blobs: Vec<raw_blob::ActiveModel>,
    tags: Vec<mega_tag::ActiveModel>,
}

impl MegaStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>, config: StorageConfig) -> Self {
        MegaStorage {
            connection,
            raw_storage: raw_storage::init(config.raw_obj_storage_type, config.raw_obj_local_path)
                .await,
            raw_obj_threshold: config.big_obj_threshold,
        }
    }

    pub fn mock() -> Self {
        MegaStorage {
            connection: Arc::new(DatabaseConnection::default()),
            raw_storage: raw_storage::mock(),
            raw_obj_threshold: 1024,
        }
    }

    pub async fn save_ref(
        &self,
        path: &str,
        ref_commit_hash: &str,
        ref_tree_hash: &str,
    ) -> Result<(), MegaError> {
        let model = mega_refs::Model {
            id: generate_id(),
            path: path.to_owned(),
            ref_commit_hash: ref_commit_hash.to_owned(),
            ref_tree_hash: ref_tree_hash.to_owned(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        model
            .into_active_model()
            .insert(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn remove_refs(&self, path: &str) -> Result<(), MegaError> {
        mega_refs::Entity::delete_many()
            .filter(mega_refs::Column::Path.starts_with(path))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn remove_ref(&self, refs: MegaRefs) -> Result<(), MegaError> {
        mega_refs::Entity::delete_by_id(refs.id)
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn get_ref(&self, path: &str) -> Result<Option<MegaRefs>, MegaError> {
        let result = mega_refs::Entity::find()
            .filter(mega_refs::Column::Path.eq(path))
            .one(self.get_connection())
            .await?;
        Ok(result.map(|model| model.into()))
    }

    pub async fn update_ref(&self, refs: MegaRefs) -> Result<(), MegaError> {
        let ref_data: mega_refs::Model = refs.into();
        let mut ref_data: mega_refs::ActiveModel = ref_data.into();
        ref_data.reset(mega_refs::Column::RefCommitHash);
        ref_data.reset(mega_refs::Column::RefTreeHash);
        ref_data.reset(mega_refs::Column::UpdatedAt);
        ref_data.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn get_open_mr(&self, path: &str) -> Result<Option<MergeRequest>, MegaError> {
        let model = mega_mr::Entity::find()
            .filter(mega_mr::Column::Path.eq(path))
            .filter(mega_mr::Column::Status.eq(MergeStatus::Open))
            .one(self.get_connection())
            .await
            .unwrap();
        if let Some(model) = model {
            let mr: MergeRequest = model.into();
            return Ok(Some(mr));
        }
        Ok(None)
    }

    pub async fn get_mr_by_status(
        &self,
        status: Vec<MergeStatus>,
    ) -> Result<Vec<mega_mr::Model>, MegaError> {
        let model = mega_mr::Entity::find()
            .filter(mega_mr::Column::Status.is_in(status))
            .order_by_desc(mega_mr::Column::CreatedAt)
            .all(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn get_mr(&self, mr_id: i64) -> Result<Option<mega_mr::Model>, MegaError> {
        let model = mega_mr::Entity::find_by_id(mr_id)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(model)
    }

    pub async fn get_open_mr_by_id(&self, mr_id: i64) -> Result<Option<MergeRequest>, MegaError> {
        let model = mega_mr::Entity::find_by_id(mr_id)
            .filter(mega_mr::Column::Status.eq(MergeStatus::Open))
            .one(self.get_connection())
            .await
            .unwrap();
        if let Some(model) = model {
            let mr: MergeRequest = model.into();
            return Ok(Some(mr));
        }
        Ok(None)
    }

    pub async fn save_mr(&self, mr: MergeRequest) -> Result<(), MegaError> {
        let model: mega_mr::Model = mr.into();
        let a_model = model.into_active_model();
        a_model.insert(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn update_mr(&self, mr: MergeRequest) -> Result<(), MegaError> {
        let model: mega_mr::Model = mr.into();
        let mut a_model = model.into_active_model();
        a_model = a_model.reset_all();
        a_model.created_at = NotSet;
        a_model.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    pub async fn get_mr_conversations(
        &self,
        mr_id: i64,
    ) -> Result<Vec<mega_mr_conv::Model>, MegaError> {
        let model = mega_mr_conv::Entity::find()
            .filter(mega_mr_conv::Column::MrId.eq(mr_id))
            .all(self.get_connection())
            .await;
        Ok(model?)
    }

    pub async fn add_mr_conversation(
        &self,
        mr_id: i64,
        user_id: i64,
        conv_type: ConvType,
    ) -> Result<i64, MegaError> {
        let conversation = mega_mr_conv::Model {
            id: generate_id(),
            mr_id,
            user_id,
            conv_type,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        let conversation = conversation.into_active_model();
        let res = conversation.insert(self.get_connection()).await.unwrap();
        Ok(res.id)
    }

    pub async fn add_mr_comment(
        &self,
        mr_id: i64,
        user_id: i64,
        comment: Option<String>,
    ) -> Result<(), MegaError> {
        let conv_id = self
            .add_mr_conversation(mr_id, user_id, ConvType::Comment)
            .await
            .unwrap();
        let comment = mega_mr_comment::Model {
            id: generate_id(),
            conv_id,
            comment,
            edited: false,
        };
        let comment = comment.into_active_model();
        comment.insert(self.get_connection()).await.unwrap();
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

    pub async fn init_monorepo(&self) {
        if self.get_ref("/").await.unwrap().is_some() {
            tracing::info!("Monorepo Directory Already Inited, skip init process!");
            return;
        }
        let converter = MegaModelConverter::init();
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

    pub async fn get_raw_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<raw_blob::Model>, MegaError> {
        Ok(raw_blob::Entity::find()
            .filter(raw_blob::Column::Sha1.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_raw_blob_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<raw_blob::Model>, MegaError> {
        Ok(raw_blob::Entity::find()
            .filter(raw_blob::Column::Sha1.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }
}

#[cfg(test)]
mod test {}
