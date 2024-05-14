use std::{env, sync::Arc};

use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect,
};

use callisto::db_enums::{ConvType, MergeStatus};
use callisto::{
    mega_commit, mega_mr, mega_mr_comment, mega_mr_conv, mega_refs, mega_tree, raw_blob,
};
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

impl MegaStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        let raw_obj_threshold = env::var("MEGA_BIG_OBJ_THRESHOLD_SIZE")
            .expect("MEGA_BIG_OBJ_THRESHOLD_SIZE not configured")
            .parse::<usize>()
            .unwrap();
        let storage_type = env::var("MEGA_RAW_STORAGE").unwrap();
        let path = env::var("MEGA_OBJ_LOCAL_PATH").unwrap();
        MegaStorage {
            connection,
            raw_storage: raw_storage::init(storage_type, path).await,
            raw_obj_threshold,
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

    pub async fn save_entry(&self, entry_list: Vec<Entry>) -> Result<(), MegaError> {
        let mut commits = Vec::new();
        let mut trees = Vec::new();
        let mut blobs = Vec::new();
        let mut raw_blobs = Vec::new();
        let mut tags = Vec::new();

        for entry in entry_list {
            let raw_obj = entry.process_entry();
            let model = raw_obj.convert_to_mega_model();
            match model {
                MegaObjectModel::Commit(commit) => commits.push(commit.into_active_model()),
                MegaObjectModel::Tree(tree) => {
                    trees.push(tree.clone().into_active_model());
                }
                MegaObjectModel::Blob(blob, raw) => {
                    blobs.push(blob.clone().into_active_model());
                    raw_blobs.push(raw.into_active_model());
                }
                MegaObjectModel::Tag(tag) => tags.push(tag.into_active_model()),
            }
        }

        batch_save_model(self.get_connection(), commits)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), trees)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), blobs)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), raw_blobs)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), tags).await.unwrap();
        Ok(())
    }

    pub async fn init_monorepo(&self) {
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
            .distinct()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_commits(&self) -> Result<Vec<mega_commit::Model>, MegaError> {
        Ok(mega_commit::Entity::find()
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
mod test {
    use std::rc::Rc;

    use venus::monorepo::mega_node::MegaNode;

    #[allow(unused)]
    pub fn print_tree(root: Rc<MegaNode>, depth: i32) {
        println!(
            "{:indent$}└── {}",
            "",
            root.name,
            indent = (depth as usize) * 4
        );
        for child in root.children.borrow().iter() {
            print_tree(child.clone(), depth + 1)
        }
    }
}
