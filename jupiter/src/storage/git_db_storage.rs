use std::ops::Deref;
use std::sync::Arc;

use futures::{Stream, StreamExt, stream};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbBackend, DbErr, EntityTrait, IntoActiveModel, QueryFilter,
    QueryTrait, Set,
};
use sea_orm::{PaginatorTrait, QueryOrder};
use tokio::sync::Mutex;

use crate::utils::converter::{GitObjectModel, process_entry};
use callisto::{git_blob, git_commit, git_repo, git_tag, git_tree, import_refs, raw_blob};
use common::errors::MegaError;
use common::model::Pagination;
use mercury::internal::pack::entry::Entry;

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct GitDbStorage {
    pub base: BaseStorage,
}

impl Deref for GitDbStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug)]
struct GitObjects {
    commits: Vec<git_commit::ActiveModel>,
    trees: Vec<git_tree::ActiveModel>,
    blobs: Vec<git_blob::ActiveModel>,
    raw_blobs: Vec<raw_blob::ActiveModel>,
    tags: Vec<git_tag::ActiveModel>,
}

impl GitDbStorage {
    pub async fn save_ref(
        &self,
        repo_id: i64,
        mut refs: import_refs::Model,
    ) -> Result<(), MegaError> {
        refs.repo_id = repo_id;
        let a_model = refs.into_active_model();
        import_refs::Entity::insert(a_model)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn remove_ref(&self, repo_id: i64, ref_name: &str) -> Result<(), MegaError> {
        import_refs::Entity::delete_many()
            .filter(import_refs::Column::RepoId.eq(repo_id))
            .filter(import_refs::Column::RefName.eq(ref_name))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn get_ref(&self, repo_id: i64) -> Result<Vec<import_refs::Model>, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo_id))
            .order_by_asc(import_refs::Column::RefName)
            .all(self.get_connection())
            .await?;
        Ok(result)
    }

    pub async fn update_ref(
        &self,
        repo_id: i64,
        ref_name: &str,
        new_id: &str,
    ) -> Result<(), MegaError> {
        let ref_data: import_refs::Model = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo_id))
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

    pub async fn get_default_ref(
        &self,
        repo_id: i64,
    ) -> Result<Option<import_refs::Model>, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo_id))
            .filter(import_refs::Column::DefaultBranch.eq(true))
            .one(self.get_connection())
            .await?;
        Ok(result)
    }

    pub async fn default_branch_exist(&self, repo_id: i64) -> Result<bool, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo_id))
            .filter(import_refs::Column::DefaultBranch.eq(true))
            .count(self.get_connection())
            .await?;
        Ok(result > 0)
    }

    pub async fn save_entry(&self, repo_id: i64, entry_list: Vec<Entry>) -> Result<(), MegaError> {
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
                    let raw_obj = process_entry(entry);
                    let model = raw_obj.convert_to_git_model();
                    let mut git_objects = git_objects.lock().await;

                    match model {
                        GitObjectModel::Commit(mut commit) => {
                            commit.repo_id = repo_id;
                            git_objects.commits.push(commit.into_active_model())
                        }
                        GitObjectModel::Tree(mut tree) => {
                            tree.repo_id = repo_id;
                            git_objects.trees.push(tree.clone().into_active_model());
                        }
                        GitObjectModel::Blob(mut blob, raw) => {
                            blob.repo_id = repo_id;
                            git_objects.blobs.push(blob.clone().into_active_model());
                            git_objects.raw_blobs.push(raw.into_active_model());
                        }
                        GitObjectModel::Tag(mut tag) => {
                            tag.repo_id = repo_id;
                            git_objects.tags.push(tag.into_active_model())
                        }
                    }
                }
            })
            .await;

        let git_objects = Arc::try_unwrap(git_objects)
            .expect("Failed to unwrap Arc")
            .into_inner();
        self.batch_save_model(git_objects.commits).await?;
        self.batch_save_model(git_objects.trees).await?;
        self.batch_save_model(git_objects.blobs).await?;
        self.batch_save_model(git_objects.raw_blobs).await?;
        self.batch_save_model(git_objects.tags).await?;
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
            .filter(Expr::cust(format!("'{repo_path}' LIKE repo_path || '%'")))
            .order_by_desc(Expr::cust("LENGTH(repo_path)"));
        tracing::debug!("{}", query.build(DbBackend::Postgres).to_string());
        let result = query.one(self.get_connection()).await?;
        Ok(result)
    }

    pub async fn save_git_repo(&self, repo: git_repo::Model) -> Result<(), MegaError> {
        let a_model = repo.into_active_model();
        git_repo::Entity::insert(a_model)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn get_commit_by_hash(
        &self,
        repo_id: i64,
        hash: &str,
    ) -> Result<Option<git_commit::Model>, MegaError> {
        Ok(git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo_id))
            .filter(git_commit::Column::CommitId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_commits_by_hashes(
        &self,
        repo_id: i64,
        hashes: &Vec<String>,
    ) -> Result<Vec<git_commit::Model>, MegaError> {
        Ok(git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo_id))
            .filter(git_commit::Column::CommitId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_commits_by_repo_id(
        &self,
        repo_id: i64,
    ) -> Result<impl Stream<Item = Result<git_commit::Model, DbErr>> + Send + '_, MegaError> {
        let stream = git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo_id))
            .stream(self.get_connection())
            .await
            .unwrap();
        Ok(stream)
    }

    pub async fn get_last_commit_by_repo_id(
        &self,
        repo_id: i64,
    ) -> Result<Option<git_commit::Model>, MegaError> {
        let one = git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo_id))
            .order_by_desc(git_commit::Column::CreatedAt)
            .one(self.get_connection())
            .await?;
        Ok(one)
    }

    pub async fn get_trees_by_repo_id(
        &self,
        repo_id: i64,
    ) -> Result<impl Stream<Item = Result<git_tree::Model, DbErr>> + '_ + Send, MegaError> {
        Ok(git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo_id))
            .stream(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_trees_by_hashes(
        &self,
        repo_id: i64,
        hashes: Vec<String>,
    ) -> Result<Vec<git_tree::Model>, MegaError> {
        Ok(git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo_id))
            .filter(git_tree::Column::TreeId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_tree_by_hash(
        &self,
        repo_id: i64,
        hash: &str,
    ) -> Result<Option<git_tree::Model>, MegaError> {
        Ok(git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo_id))
            .filter(git_tree::Column::TreeId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_blobs_by_repo_id(
        &self,
        repo_id: i64,
    ) -> Result<impl Stream<Item = Result<git_blob::Model, DbErr>> + '_ + Send, MegaError> {
        Ok(git_blob::Entity::find()
            .filter(git_blob::Column::RepoId.eq(repo_id))
            .stream(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_blobs_by_hashes(
        &self,
        repo_id: i64,
        hashes: Vec<String>,
    ) -> Result<Vec<git_blob::Model>, MegaError> {
        Ok(git_blob::Entity::find()
            .filter(git_blob::Column::RepoId.eq(repo_id))
            .filter(git_blob::Column::BlobId.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_tags_by_repo_id(
        &self,
        repo_id: i64,
    ) -> Result<Vec<git_tag::Model>, MegaError> {
        Ok(git_tag::Entity::find()
            .filter(git_tag::Column::RepoId.eq(repo_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    /// Paginated annotated tags for a given import repo id.
    pub async fn list_tags_by_repo_with_page(
        &self,
        repo_id: i64,
        page: Pagination,
    ) -> Result<(Vec<git_tag::Model>, u64), MegaError> {
        let paginator = git_tag::Entity::find()
            .filter(git_tag::Column::RepoId.eq(repo_id))
            .order_by_asc(git_tag::Column::TagName)
            .paginate(self.get_connection(), page.per_page);
        let num_items = paginator.num_items().await?;
        Ok(paginator
            .fetch_page(page.page.saturating_sub(1))
            .await
            .map(|m| (m, num_items))?)
    }

    /// Find single tag by repo id and tag name
    pub async fn get_tag_by_repo_and_name(
        &self,
        repo_id: i64,
        name: &str,
    ) -> Result<Option<git_tag::Model>, MegaError> {
        let res = git_tag::Entity::find()
            .filter(git_tag::Column::RepoId.eq(repo_id))
            .filter(git_tag::Column::TagName.eq(name.to_string()))
            .one(self.get_connection())
            .await?;
        Ok(res)
    }

    /// Insert a single tag model
    pub async fn insert_tag(&self, tag: git_tag::Model) -> Result<git_tag::Model, MegaError> {
        let am: git_tag::ActiveModel = tag.clone().into();
        git_tag::Entity::insert(am)
            .exec(self.get_connection())
            .await?;
        // load saved model back by tag_id
        let model = git_tag::Entity::find()
            .filter(git_tag::Column::TagId.eq(tag.tag_id.clone()))
            .one(self.get_connection())
            .await?;
        match model {
            Some(m) => Ok(m),
            None => Err(MegaError::with_message("Failed to load inserted tag")),
        }
    }

    /// Delete a tag by repo id and name
    pub async fn delete_tag(&self, repo_id: i64, name: &str) -> Result<(), MegaError> {
        git_tag::Entity::delete_many()
            .filter(git_tag::Column::RepoId.eq(repo_id))
            .filter(git_tag::Column::TagName.eq(name.to_string()))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn get_obj_count_by_repo_id(&self, repo_id: i64) -> usize {
        let c_count = git_commit::Entity::find()
            .filter(git_commit::Column::RepoId.eq(repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let t_count = git_tree::Entity::find()
            .filter(git_tree::Column::RepoId.eq(repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let b_count = git_blob::Entity::find()
            .filter(git_blob::Column::RepoId.eq(repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let tag_count = git_tag::Entity::find()
            .filter(git_tag::Column::RepoId.eq(repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        (c_count + t_count + b_count + tag_count)
            .try_into()
            .unwrap()
    }
}
