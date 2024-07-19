use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::Stream;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, QueryTrait, Set,
};
use sea_orm::{PaginatorTrait, QueryOrder};

use callisto::{git_blob, git_commit, git_repo, git_tag, git_tree, import_refs, raw_blob};
use common::config::StorageConfig;
use common::errors::MegaError;
use mercury::internal::object::GitObjectModel;
use mercury::internal::pack::entry::Entry;
use venus::import_repo::import_refs::RefCommand;
use venus::import_repo::import_refs::Refs;
use venus::import_repo::repo::Repo;

use crate::{
    raw_storage::{self, RawStorage},
    storage::GitStorageProvider,
};

use crate::storage::batch_save_model;

#[derive(Clone)]
pub struct GitDbStorage {
    pub raw_storage: Arc<dyn RawStorage>,
    pub connection: Arc<DatabaseConnection>,
    pub raw_obj_threshold: usize,
}

#[async_trait]
impl GitStorageProvider for GitDbStorage {
    async fn save_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        let mut model: import_refs::Model = refs.clone().into();
        model.repo_id = repo.repo_id;
        let a_model = model.into_active_model();
        import_refs::Entity::insert(a_model)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    async fn remove_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        import_refs::Entity::delete_many()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .filter(import_refs::Column::RefName.eq(refs.ref_name.clone()))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    async fn get_ref(&self, repo: &Repo) -> Result<Vec<Refs>, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .order_by_asc(import_refs::Column::RefName)
            .all(self.get_connection())
            .await?;
        let res: Vec<Refs> = result.into_iter().map(|x| x.into()).collect();
        Ok(res)
    }

    async fn update_ref(&self, repo: &Repo, ref_name: &str, new_id: &str) -> Result<(), MegaError> {
        let ref_data: import_refs::Model = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .filter(import_refs::Column::RefName.eq(ref_name))
            .one(self.get_connection())
            .await
            .unwrap()
            .unwrap();
        let mut ref_data: import_refs::ActiveModel = ref_data.into();
        ref_data.ref_git_id = Set(new_id.to_string());
        ref_data.updated_at = Set(chrono::Utc::now().naive_utc());
        ref_data.update(self.get_connection()).await.unwrap();
        Ok(())
    }
}

impl GitDbStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>, config: StorageConfig) -> Self {
        GitDbStorage {
            connection,
            raw_storage: raw_storage::init(config.raw_obj_storage_type, config.raw_obj_local_path)
                .await,
            raw_obj_threshold: config.big_obj_threshold,
        }
    }

    pub fn mock() -> Self {
        GitDbStorage {
            connection: Arc::new(DatabaseConnection::default()),
            raw_storage: raw_storage::mock(),
            raw_obj_threshold: 1024,
        }
    }

    pub async fn get_default_ref(&self, repo: &Repo) -> Result<Option<Refs>, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .filter(import_refs::Column::DefaultBranch.eq(true))
            .one(self.get_connection())
            .await?;
        if let Some(model) = result {
            let refs: Refs = model.into();
            Ok(Some(refs))
        } else {
            Ok(None)
        }
    }

    pub async fn default_branch_exist(&self, repo: &Repo) -> Result<bool, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .filter(import_refs::Column::DefaultBranch.eq(true))
            .count(self.get_connection())
            .await?;
        Ok(result > 0)
    }

    pub async fn save_entry(&self, repo: &Repo, entry_list: Vec<Entry>) -> Result<(), MegaError> {
        let (commits, trees, blobs, raw_blobs, tags) = (
            Mutex::new(Vec::new()),
            Mutex::new(Vec::new()),
            Mutex::new(Vec::new()),
            Mutex::new(Vec::new()),
            Mutex::new(Vec::new()),
        );
        entry_list.par_iter().for_each(|entry| {
            let raw_obj = entry.process_entry();
            let model = raw_obj.convert_to_git_model();
            match model {
                GitObjectModel::Commit(mut commit) => {
                    commit.repo_id = repo.repo_id;
                    commits.lock().unwrap().push(commit.into_active_model())
                }
                GitObjectModel::Tree(mut tree) => {
                    tree.repo_id = repo.repo_id;
                    trees.lock().unwrap().push(tree.clone().into_active_model());
                }
                GitObjectModel::Blob(mut blob, raw) => {
                    blob.repo_id = repo.repo_id;
                    blobs.lock().unwrap().push(blob.clone().into_active_model());
                    raw_blobs.lock().unwrap().push(raw.into_active_model());
                }
                GitObjectModel::Tag(mut tag) => {
                    tag.repo_id = repo.repo_id;
                    tags.lock().unwrap().push(tag.into_active_model())
                }
            }
        });

        batch_save_model(self.get_connection(), commits.into_inner().unwrap())
            .await
            .unwrap();
        batch_save_model(self.get_connection(), trees.into_inner().unwrap())
            .await
            .unwrap();
        batch_save_model(self.get_connection(), blobs.into_inner().unwrap())
            .await
            .unwrap();
        batch_save_model(self.get_connection(), raw_blobs.into_inner().unwrap())
            .await
            .unwrap();
        batch_save_model(self.get_connection(), tags.into_inner().unwrap())
            .await
            .unwrap();
        Ok(())
    }

    /// Finds a Git repository with an exact match on the repository path.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - A string slice that holds the path of the repository to search for.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Option` with the Git repository model if found, or `None` if not found.
    /// Returns a `MegaError` if an error occurs during the search.
    pub async fn find_git_repo_exact_match(
        &self,
        repo_path: &str,
    ) -> Result<Option<git_repo::Model>, MegaError> {
        let result = git_repo::Entity::find()
            .filter(git_repo::Column::RepoPath.eq(repo_path))
            .one(self.get_connection())
            .await?;
        Ok(result)
    }

    /// Finds a Git repository with a path that matches the beginning of the provided repository path using a LIKE query.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - A string slice that holds the beginning of the path of the repository to search for.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Option` with the Git repository model if found, or `None` if not found.
    /// Returns a `MegaError` if an error occurs during the search.
    pub async fn find_git_repo_like_path(
        &self,
        repo_path: &str,
    ) -> Result<Option<git_repo::Model>, MegaError> {
        let query = git_repo::Entity::find()
            .filter(Expr::cust(format!("'{}' LIKE repo_path || '%'", repo_path)))
            .order_by_desc(Expr::cust("LENGTH(repo_path)"));
        tracing::debug!("{}", query.build(DbBackend::Postgres).to_string());
        let result = query.one(self.get_connection()).await?;
        Ok(result)
    }

    pub async fn find_git_repo_by_path(
        &self,
        repo_path: &str,
    ) -> Result<Option<git_repo::Model>, MegaError> {
        let query = git_repo::Entity::find().filter(git_repo::Column::RepoPath.eq(repo_path));
        tracing::debug!("{}", query.build(DbBackend::Postgres).to_string());
        let result = query.one(self.get_connection()).await?;
        Ok(result)
    }

    pub async fn save_git_repo(&self, repo: Repo) -> Result<(), MegaError> {
        let model: git_repo::Model = repo.into();
        let a_model = model.into_active_model();
        git_repo::Entity::insert(a_model)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn get_commit_by_hash(
        &self,
        repo: &Repo,
        hash: &str,
    ) -> Result<Option<git_commit::Model>, MegaError> {
        Ok(git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo.repo_id))
            .filter(git_commit::Column::CommitId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_commits_by_hashes(
        &self,
        repo: &Repo,
        hashes: &Vec<String>,
    ) -> Result<Vec<git_commit::Model>, MegaError> {
        Ok(git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo.repo_id))
            .filter(git_commit::Column::CommitId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_commits_by_repo_id<'a>(
        &'a self,
        repo: &'a Repo,
    ) -> Result<impl Stream<Item = Result<git_commit::Model, DbErr>> + 'a + Send, MegaError> {
        let stream = git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo.repo_id))
            .stream(self.get_connection())
            .await
            .unwrap();
        Ok(stream)
    }

    pub async fn get_trees_by_repo_id<'a>(
        &'a self,
        repo: &'a Repo,
    ) -> Result<impl Stream<Item = Result<git_tree::Model, DbErr>> + 'a + Send, MegaError> {
        Ok(git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo.repo_id))
            .stream(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_trees_by_hashes(
        &self,
        repo: &Repo,
        hashes: Vec<String>,
    ) -> Result<Vec<git_tree::Model>, MegaError> {
        Ok(git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo.repo_id))
            .filter(git_tree::Column::TreeId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_tree_by_hash(
        &self,
        repo: &Repo,
        hash: &str,
    ) -> Result<Option<git_tree::Model>, MegaError> {
        Ok(git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo.repo_id))
            .filter(git_tree::Column::TreeId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_blobs_by_repo_id<'a>(
        &'a self,
        repo: &'a Repo,
    ) -> Result<impl Stream<Item = Result<git_blob::Model, DbErr>> + 'a + Send, MegaError> {
        Ok(git_blob::Entity::find()
            .filter(git_blob::Column::RepoId.eq(repo.repo_id))
            .stream(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_blobs_by_hashes(
        &self,
        repo: &Repo,
        hashes: Vec<String>,
    ) -> Result<Vec<git_blob::Model>, MegaError> {
        Ok(git_blob::Entity::find()
            .filter(git_blob::Column::RepoId.eq(repo.repo_id))
            .filter(git_blob::Column::BlobId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_raw_blobs(
        &self,
        hashes: Vec<String>,
    ) -> Result<impl Stream<Item = Result<raw_blob::Model, DbErr>> + '_ + Send, MegaError> {
        Ok(raw_blob::Entity::find()
            .filter(raw_blob::Column::Sha1.is_in(hashes))
            .stream(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_tags_by_repo_id(&self, repo: &Repo) -> Result<Vec<git_tag::Model>, MegaError> {
        Ok(git_tag::Entity::find()
            .filter(git_tag::Column::RepoId.eq(repo.repo_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_obj_count_by_repo_id(&self, repo: &Repo) -> usize {
        let c_count = git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo.repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let t_count = git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo.repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let b_count = git_blob::Entity::find()
            .filter(git_blob::Column::RepoId.eq(repo.repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let tag_count = git_tag::Entity::find()
            .filter(git_tag::Column::RepoId.eq(repo.repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        (c_count + t_count + b_count + tag_count)
            .try_into()
            .unwrap()
    }
}
