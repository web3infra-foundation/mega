use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use futures::stream::FuturesUnordered;
use futures::{StreamExt, stream};

use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::commit_binding_storage::CommitBindingStorage;
use crate::storage::user_storage::UserStorage;
use crate::utils::converter::MegaModelConverter;
use crate::utils::converter::{IntoMegaModel, MegaObjectModel, ToRawBlob, process_entry};
use callisto::{mega_blob, mega_commit, mega_refs, mega_tag, mega_tree, raw_blob};
use common::config::MonoConfig;
use common::errors::MegaError;
use common::model::Pagination;
use common::utils::MEGA_BRANCH_NAME;
use git_internal::internal::metadata::{EntryMeta, MetaAttached};
use git_internal::internal::object::ObjectTrait;
use git_internal::internal::object::blob::Blob;
use git_internal::internal::{object::commit::Commit, pack::entry::Entry};
use sea_orm::sea_query::Expr;

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

#[derive(Debug)]
pub struct RefUpdateData {
    pub path: String,
    pub ref_name: String,
    pub commit_id: String,
    pub tree_hash: String,
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

    pub async fn save_refs(&self, model: mega_refs::Model) -> Result<(), MegaError> {
        model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(())
    }

    /// Removes non-CL refs under the given path, but keeps the ref matching the path itself.
    pub async fn remove_none_cl_refs(&self, path: &str) -> Result<(), MegaError> {
        mega_refs::Entity::delete_many()
            .filter(mega_refs::Column::Path.starts_with(path))
            .filter(mega_refs::Column::Path.ne(path))
            .filter(mega_refs::Column::IsCl.eq(false))
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

    pub async fn get_refs_for_paths_and_cls(
        &self,
        paths: &[&str],
        cls: Option<&[&str]>,
    ) -> Result<Vec<mega_refs::Model>, MegaError> {
        let mut query = mega_refs::Entity::find()
            .filter(mega_refs::Column::Path.is_in(paths.iter().copied()))
            .order_by_asc(mega_refs::Column::RefName);

        if let Some(cls_values) = cls {
            query = query.filter(mega_refs::Column::RefName.is_in(cls_values.iter().copied()));
        } else {
            query = query.filter(mega_refs::Column::RefName.eq(MEGA_BRANCH_NAME));
        }

        let result = query.all(self.get_connection()).await?;
        Ok(result)
    }

    pub async fn get_all_refs(
        &self,
        path: &str,
        filter_cl: bool,
    ) -> Result<Vec<mega_refs::Model>, MegaError> {
        let mut query = mega_refs::Entity::find()
            .filter(mega_refs::Column::Path.eq(path))
            .order_by_asc(mega_refs::Column::RefName);

        if filter_cl {
            query = query.filter(mega_refs::Column::IsCl.eq(false));
        }
        let result = query.all(self.get_connection()).await?;

        Ok(result)
    }

    pub async fn get_main_ref(&self, path: &str) -> Result<Option<mega_refs::Model>, MegaError> {
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
    pub async fn batch_update_by_path_concurrent(
        &self,
        updates: Vec<RefUpdateData>,
    ) -> Result<(), MegaError> {
        let conn = self.get_connection();
        let mut condition = Condition::any();
        for update in &updates {
            condition = condition.add(
                Condition::all()
                    .add(mega_refs::Column::Path.eq(update.path.clone()))
                    .add(mega_refs::Column::RefName.eq(update.ref_name.clone())),
            );
        }

        let existing_refs: Vec<mega_refs::Model> = mega_refs::Entity::find()
            .filter(condition)
            .all(conn)
            .await?;

        let ref_map: HashMap<(String, String), mega_refs::Model> = existing_refs
            .into_iter()
            .map(|r| ((r.path.clone(), r.ref_name.clone()), r))
            .collect();

        let mut futures = FuturesUnordered::new();

        for update in updates {
            if let Some(ref_data) = ref_map.get(&(update.path.clone(), update.ref_name.clone())) {
                let conn = conn.clone();
                let mut active: mega_refs::ActiveModel = ref_data.clone().into();

                futures.push(async move {
                    active.ref_commit_hash = Set(update.commit_id);
                    active.ref_tree_hash = Set(update.tree_hash);
                    active.updated_at = Set(chrono::Utc::now().naive_utc());
                    active.update(&conn).await?;
                    Ok::<(), MegaError>(())
                });
            }
        }

        while let Some(res) = futures.next().await {
            res?;
        }

        Ok(())
    }

    pub async fn save_entry(
        &self,
        commit_id: &str,
        entry_list: Vec<MetaAttached<Entry, EntryMeta>>,
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
                    let entry_data = entry.inner.data.clone();
                    let entry_hash = entry.inner.hash;
                    let raw_obj = process_entry(entry.inner);
                    let model = raw_obj.convert_to_mega_model(entry.meta);
                    let mut git_objects = git_objects.lock().unwrap();
                    match model {
                        MegaObjectModel::Commit(commit) => {
                            // Store for binding processing
                            if let Ok(commit_obj) =
                                git_internal::internal::object::commit::Commit::from_bytes(
                                    &entry_data,
                                    entry_hash,
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

    pub async fn update_blob_filepath(
        &self,
        blob_id: &str,
        file_path: &str,
    ) -> Result<(), MegaError> {
        if let Some(model) = mega_blob::Entity::find()
            .filter(mega_blob::Column::BlobId.eq(blob_id))
            .one(self.get_connection())
            .await?
        {
            let mut active: mega_blob::ActiveModel = model.into();

            active.file_path = Set(file_path.to_string());

            active.update(self.get_connection()).await?;
        }

        Ok(())
    }

    pub async fn update_pack_id(&self, temp_pack_id: &str, pack_id: &str) -> Result<(), MegaError> {
        let conn = self.get_connection();

        //
        let txn: DatabaseTransaction = conn.begin().await?;

        //
        let tables = [
            (
                "mega_blob",
                mega_blob::Entity::update_many()
                    .col_expr(mega_blob::Column::PackId, Expr::value(pack_id))
                    .filter(mega_blob::Column::PackId.eq(temp_pack_id))
                    .exec(&txn)
                    .await?,
            ),
            (
                "mega_tree",
                mega_tree::Entity::update_many()
                    .col_expr(mega_tree::Column::PackId, Expr::value(pack_id))
                    .filter(mega_tree::Column::PackId.eq(temp_pack_id))
                    .exec(&txn)
                    .await?,
            ),
            (
                "mega_tag",
                mega_tag::Entity::update_many()
                    .col_expr(mega_tag::Column::PackId, Expr::value(pack_id))
                    .filter(mega_tag::Column::PackId.eq(temp_pack_id))
                    .exec(&txn)
                    .await?,
            ),
            (
                "mega_commit",
                mega_commit::Entity::update_many()
                    .col_expr(mega_commit::Column::PackId, Expr::value(pack_id))
                    .filter(mega_commit::Column::PackId.eq(temp_pack_id))
                    .exec(&txn)
                    .await?,
            ),
        ];

        //
        for (name, res) in tables {
            if res.rows_affected > 0 {
                tracing::info!("mega object Updated {} rows in {}", res.rows_affected, name);
            }
        }

        //
        txn.commit().await?;
        Ok(())
    }

    /// Process commit author bindings
    async fn process_commit_bindings(
        &self,
        commits: &[(String, String)],
        authenticated_username: Option<&str>,
    ) -> Result<(), MegaError> {
        let commit_binding_storage = self.commit_binding_storage();

        for (commit_sha, _author_email) in commits {
            // Try to find user by authenticated username first
            let matched_username = if let Some(username) = authenticated_username {
                // Local users table removed: accept authenticated username directly
                Some(username.to_string())
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
                .upsert_binding(commit_sha, matched_username.clone(), is_anonymous)
                .await
            {
                tracing::error!("Failed to save commit binding for {}: {}", commit_sha, e);
                // Continue processing other commits even if one fails
            } else {
                tracing::info!(
                    "Processed binding for commit {} (anonymous: {}, username: {})",
                    commit_sha,
                    is_anonymous,
                    matched_username.unwrap_or_else(|| "anonymous".to_string())
                );
            }
        }
        Ok(())
    }

    pub async fn init_monorepo(&self, mono_config: &MonoConfig) {
        if self.get_main_ref("/").await.unwrap().is_some() {
            tracing::info!("Monorepo Directory Already Inited, skip init process!");
            return;
        }
        let converter = MegaModelConverter::init(mono_config);
        let commit: mega_commit::Model = converter.commit.into_mega_model(EntryMeta::default());
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
            .map(|c| c.into_mega_model(EntryMeta::default()))
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
            .iter()
            .map(|b| (*b).clone().into_mega_model(EntryMeta::default()))
            .map(|mut m: mega_blob::Model| {
                m.commit_id = commit_id.to_owned();
                m.into_active_model()
            })
            .collect();
        self.batch_save_model(mega_blobs).await.unwrap();

        let raw_blobs: Vec<raw_blob::ActiveModel> = blobs
            .into_iter()
            .map(|b| b.to_raw_blob())
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

    pub async fn get_tag_by_name(&self, name: &str) -> Result<Option<mega_tag::Model>, MegaError> {
        let res = mega_tag::Entity::find()
            .filter(mega_tag::Column::TagName.eq(name.to_string()))
            .one(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn insert_tag(&self, tag: mega_tag::Model) -> Result<mega_tag::Model, MegaError> {
        let am: mega_tag::ActiveModel = tag.clone().into();
        mega_tag::Entity::insert(am)
            .exec(self.get_connection())
            .await?;
        let model = mega_tag::Entity::find()
            .filter(mega_tag::Column::TagId.eq(tag.tag_id.clone()))
            .one(self.get_connection())
            .await?;
        match model {
            Some(m) => Ok(m),
            None => Err(MegaError::with_message("Failed to load inserted tag")),
        }
    }

    pub async fn delete_tag_by_name(&self, name: &str) -> Result<(), MegaError> {
        mega_tag::Entity::delete_many()
            .filter(mega_tag::Column::TagName.eq(name.to_string()))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    /// Paginated annotated tags stored in mega_tag table
    pub async fn get_tags_by_page(
        &self,
        page: Pagination,
    ) -> Result<(Vec<mega_tag::Model>, u64), MegaError> {
        let paginator = mega_tag::Entity::find()
            .order_by_asc(mega_tag::Column::TagName)
            .paginate(self.get_connection(), page.per_page);
        let num_items = paginator.num_items().await?;
        Ok(paginator
            .fetch_page(page.page.saturating_sub(1))
            .await
            .map(|m| (m, num_items))?)
    }
}

#[cfg(test)]
mod test {}
