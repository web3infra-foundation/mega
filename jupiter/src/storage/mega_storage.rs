use std::collections::VecDeque;
use std::rc::Rc;
use std::str::FromStr;
use std::{env, sync::Arc};

use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, Set,
};

use callisto::db_enums::MergeStatus;
use callisto::{
    git_repo, mega_blob, mega_commit, mega_mr, mega_snapshot, mega_tag, mega_tree, raw_blob, refs,
};
use common::errors::MegaError;
use ganymede::mega_node::MegaNode;
use ganymede::model::converter::{self, MegaModelConverter};
use ganymede::model::create_file::CreateFileInfo;
use venus::hash::SHA1;
use venus::internal::object::blob::Blob;
use venus::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use venus::internal::object::GitObjectModel;
use venus::internal::pack::reference::CommandType;
use venus::internal::{
    object::commit::Commit,
    pack::{entry::Entry, reference::RefCommand},
};
use venus::mr::MergeRequest;
use venus::repo::Repo;

use crate::storage::batch_save_model;
use crate::{
    raw_storage::{self, RawStorage},
    storage::GitStorageProvider,
};

use super::batch_save_model_with_conflict;

#[derive(Clone)]
pub struct MegaStorage {
    pub raw_storage: Arc<dyn RawStorage>,
    pub connection: Arc<DatabaseConnection>,
    pub raw_obj_threshold: usize,
}

#[async_trait]
impl GitStorageProvider for MegaStorage {
    async fn save_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        let mut model: refs::Model = refs.clone().into();
        model.ref_git_id = refs.new_id.clone();
        model.repo_id = repo.repo_id;
        let a_model = model.into_active_model();
        refs::Entity::insert(a_model)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    async fn remove_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        refs::Entity::delete_many()
            .filter(refs::Column::RepoId.eq(repo.repo_id))
            .filter(refs::Column::RefName.eq(refs.ref_name.clone()))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    async fn get_ref(&self, repo: &Repo, ref_name: &str) -> Result<String, MegaError> {
        let result = refs::Entity::find()
            .filter(refs::Column::RepoId.eq(repo.repo_id))
            .filter(refs::Column::RefName.eq(ref_name))
            .one(self.get_connection())
            .await?;
        if let Some(model) = result {
            return Ok(model.ref_git_id);
        }
        Ok(String::new())
    }

    async fn update_ref(&self, repo: &Repo, ref_name: &str, new_id: &str) -> Result<(), MegaError> {
        let ref_data: refs::Model = refs::Entity::find()
            .filter(refs::Column::RepoId.eq(repo.repo_id))
            .filter(refs::Column::RefName.eq(ref_name))
            .one(self.get_connection())
            .await
            .unwrap()
            .unwrap();
        let mut ref_data: refs::ActiveModel = ref_data.into();
        ref_data.ref_git_id = Set(new_id.to_string());
        ref_data.updated_at = Set(chrono::Utc::now().naive_utc());
        ref_data.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    // async fn save_entry(&self, repo: &Repo, entry_list: Vec<Entry>) -> Result<(), MegaError> {
    // let mut save_models: Vec<raw_objects::ActiveModel> = Vec::new();
    // for entry in result_entity.iter() {
    //     let mut model: raw_objects::Model = entry.clone().into();
    //     let data = model.data.clone().unwrap();
    //     // save data through raw_storage instead of database if exceed threshold
    //     if self.raw_obj_threshold != 0 && data.len() / 1024 > self.raw_obj_threshold {
    //         let b_link = self
    //             .raw_storage
    //             .put_entry(&repo.repo_name, entry)
    //             .await
    //             .unwrap();
    //         model.storage_type = self.raw_storage.get_storage_type();
    //         model.data = Some(b_link);
    //     }
    //     save_models.push(model.into_active_model())
    // }
    // batch_save_model(self.get_connection(), save_models)
    //     .await
    //     .unwrap();
    // }

    async fn get_entry_by_sha1(
        &self,
        _repo: Repo,
        _sha1_vec: Vec<&str>,
    ) -> Result<Vec<Entry>, MegaError> {
        // let models = raw_objects::Entity::find()
        //     .filter(raw_objects::Column::Sha1.is_in(sha1_vec))
        //     .all(self.get_connection())
        //     .await
        //     .unwrap();
        // let mut result: Vec<Entry> = Vec::new();
        // for mut model in models {
        //     if model.storage_type == StorageType::Database {
        //         result.push(model.into());
        //     } else {
        //         let data = self
        //             .raw_storage
        //             .get_object(&repo.repo_name, &model.sha1)
        //             .await
        //             .unwrap();
        //         model.data = Some(data.to_vec());
        //         result.push(model.into());
        //     }
        // }
        // Ok(result)
        todo!()
    }
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

    pub async fn handler_refs(&self, repo: &Repo, refs: &RefCommand) {
        match refs.command_type {
            CommandType::Create => {
                self.save_ref(repo, refs).await.unwrap();
            }
            CommandType::Delete => self.remove_ref(repo, refs).await.unwrap(),
            CommandType::Update => {
                self.update_ref(repo, &refs.ref_name, &refs.new_id)
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn get_repo_refs(&self, repo: &Repo) -> Result<Vec<refs::Model>, MegaError> {
        Ok(refs::Entity::find()
            .filter(refs::Column::RepoId.eq(repo.repo_id))
            .all(self.get_connection())
            .await?)
    }

    pub async fn save_mr(&self, mr: MergeRequest) -> Result<(), MegaError> {
        let model: mega_mr::Model = mr.into();
        mega_mr::Entity::insert(model.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn save_entry(
        &self,
        mr: &MergeRequest,
        repo: &Repo,
        entry_list: Vec<Entry>,
    ) -> Result<(), MegaError> {
        let mut commits = Vec::new();
        let mut trees = Vec::new();
        let mut blobs = Vec::new();
        let mut raw_blobs = Vec::new();
        let mut tags = Vec::new();
        let mut snapshots: Vec<mega_snapshot::ActiveModel> = Vec::new();

        for entry in entry_list {
            let raw_obj = entry.process_entry();
            let model = raw_obj.convert_to_mega_model(repo.repo_id, mr.id);
            match model {
                GitObjectModel::Commit(commit) => commits.push(commit.into_active_model()),
                GitObjectModel::Tree(tree) => {
                    trees.push(tree.clone().into_active_model());
                    if repo.monorepo() {
                        let snapshot: mega_snapshot::Model = tree.into();
                        snapshots.push(snapshot.into_active_model());
                    }
                }
                GitObjectModel::Blob(blob, raw) => {
                    blobs.push(blob.clone().into_active_model());
                    raw_blobs.push(raw.into_active_model());
                    if repo.monorepo() {
                        let snapshot: mega_snapshot::Model = blob.into();
                        snapshots.push(snapshot.into_active_model());
                    }
                }
                GitObjectModel::Tag(tag) => tags.push(tag.into_active_model()),
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
        if repo.monorepo() {
            batch_save_model_with_conflict(
                self.get_connection(),
                snapshots,
                OnConflict::new()
                    .update_columns(vec![
                        mega_snapshot::Column::CommitId,
                        mega_snapshot::Column::Sha1,
                    ])
                    .to_owned(),
            )
            .await
            .unwrap();
        }
        Ok(())
    }

    pub async fn init_mega_directory(&self) {
        let converter = MegaModelConverter::init();
        let mut commit: mega_commit::Model = converter.commit.into();
        commit.status = MergeStatus::Merged;
        mega_commit::Entity::insert(commit.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap();
        refs::Entity::insert(converter.refs)
            .exec(self.get_connection())
            .await
            .unwrap();

        let mega_trees = converter.mega_trees.borrow().clone();
        batch_save_model(self.get_connection(), mega_trees)
            .await
            .unwrap();
        let mega_blobs = converter.mega_blobs.borrow().clone();
        batch_save_model(self.get_connection(), mega_blobs)
            .await
            .unwrap();
        let raw_blobs = converter.raw_blobs.borrow().values().cloned().collect();
        batch_save_model(self.get_connection(), raw_blobs)
            .await
            .unwrap();
        let mega_snapshot = converter.mega_snapshots.borrow().clone();
        batch_save_model(self.get_connection(), mega_snapshot)
            .await
            .unwrap();
    }

    #[allow(unused)]
    fn mega_node_tree(&self, file_infos: Vec<CreateFileInfo>) -> Result<Rc<MegaNode>, MegaError> {
        let mut stack: VecDeque<CreateFileInfo> = VecDeque::new();
        let mut root: Option<Rc<MegaNode>> = None;

        for f_info in file_infos {
            if f_info.path == "/" && f_info.is_directory {
                root = Some(Rc::new(f_info.into()));
            } else {
                stack.push_back(f_info);
            }
        }

        let root = if let Some(info) = root {
            info
        } else {
            unreachable!("can not create files without root directory!")
        };
        let mut parents = VecDeque::new();
        parents.push_back(Rc::clone(&root));

        while !parents.is_empty() {
            let parent = parents.pop_front().unwrap();
            let mut index = 0;
            while index < stack.len() {
                let element = &stack[index];
                if element.path == parent.path.join(parent.name.clone()).to_str().unwrap() {
                    let node: Rc<MegaNode> = Rc::new(element.clone().into());
                    if node.is_directory {
                        parents.push_back(Rc::clone(&node))
                    }
                    parent.add_child(&Rc::clone(&node));
                    stack.remove(index);
                } else {
                    index += 1;
                }
            }
        }
        Ok(root)
    }

    pub async fn create_mega_file(&self, file_info: CreateFileInfo) -> Result<(), MegaError> {
        let mut save_trees: Vec<mega_tree::ActiveModel> = Vec::new();
        let mut mega_tree = self
            .get_mega_tree_by_path(&file_info.path)
            .await
            .unwrap()
            .unwrap();
        let mut p_tree: Tree = mega_tree.clone().into();

        let new_item = if file_info.is_directory {
            let blob = converter::generate_git_keep();
            let tree_item = TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: String::from(".gitkeep"),
            };
            let child_tree = Tree::from_tree_items(vec![tree_item]).unwrap();
            TreeItem {
                mode: TreeItemMode::Tree,
                id: child_tree.id,
                name: file_info.name.clone(),
            }
        } else {
            let blob = Blob::from_content(&file_info.content.unwrap());
            TreeItem {
                mode: TreeItemMode::Blob,
                id: blob.id,
                name: file_info.name.clone(),
            }
        };
        p_tree.tree_items.push(new_item);
        let mut new_tree = Tree::from_tree_items(p_tree.tree_items).unwrap();
        let model: mega_tree::Model = new_tree.clone().into();
        save_trees.push(model.into_active_model());

        while let Some(parent_id) = mega_tree.parent_id {
            let replace_name = mega_tree.name;
            mega_tree = self
                .get_mega_tree_by_sha(&parent_id)
                .await
                .unwrap()
                .unwrap();
            let mut tmp: Tree = mega_tree.clone().into();
            if let Some(item) = tmp.tree_items.iter_mut().find(|x| x.name == replace_name) {
                item.id = new_tree.id;
            }

            new_tree = Tree::from_tree_items(tmp.tree_items).unwrap();
            let model: mega_tree::Model = new_tree.clone().into();
            save_trees.push(model.into_active_model());
        }

        // save_trees
        //     .iter()
        //     .for_each(|x| println!("{:?}, {:?}", x.name, x.tree_id));
        let repo = Repo::empty();
        let ref_id = self.get_ref(&repo, "main").await.unwrap();
        let commit = converter::init_commit(
            SHA1::from_str(&mega_tree.tree_id).unwrap(),
            vec![SHA1::from_str(&ref_id).unwrap()],
            &format!("create file {} commit", file_info.name),
        );
        // update ref
        self.update_ref(&repo, "main", &commit.id.to_plain_str())
            .await
            .unwrap();
        self.save_mega_commits(0, MergeStatus::Merged, vec![commit])
            .await
            .unwrap();
        Ok(())
    }

    pub async fn find_git_repo(
        &self,
        repo_path: &str,
    ) -> Result<Option<git_repo::Model>, MegaError> {
        let result = git_repo::Entity::find()
            .filter(git_repo::Column::RepoPath.eq(repo_path))
            .one(self.get_connection())
            .await?;
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

    #[allow(unused)]
    pub async fn update_git_repo(&self, repo: Repo) -> Result<(), MegaError> {
        let git_repo = git_repo::Entity::find_by_id(repo.repo_id)
            .one(self.get_connection())
            .await
            .unwrap();
        let git_repo: git_repo::ActiveModel = git_repo.unwrap().into();
        git_repo.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    async fn save_mega_commits(
        &self,
        mr_id: i64,
        status: MergeStatus,
        commits: Vec<Commit>,
    ) -> Result<(), MegaError> {
        let mega_commits: Vec<mega_commit::Model> =
            commits.into_iter().map(mega_commit::Model::from).collect();
        let mut save_models = Vec::new();
        for mut mega_commit in mega_commits {
            mega_commit.status = status;
            mega_commit.mr_id = mr_id;
            save_models.push(mega_commit.into_active_model());
        }
        batch_save_model(self.get_connection(), save_models)
            .await
            .unwrap();
        Ok(())
    }

    pub async fn get_commit_by_hash(
        &self,
        sha: &str,
        repo: &Repo,
    ) -> Result<Option<mega_commit::Model>, MegaError> {
        Ok(mega_commit::Entity::find()
            .filter(mega_commit::Column::CommitId.eq(sha))
            .filter(mega_commit::Column::RepoId.eq(repo.repo_id))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_commits_by_repo_id(
        &self,
        repo: &Repo,
    ) -> Result<Vec<mega_commit::Model>, MegaError> {
        Ok(mega_commit::Entity::find()
            .filter(mega_commit::Column::RepoId.eq(repo.repo_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    async fn get_mega_tree_by_path(
        &self,
        full_path: &str,
    ) -> Result<Option<mega_tree::Model>, MegaError> {
        Ok(mega_tree::Entity::find()
            .filter(mega_tree::Column::FullPath.eq(full_path))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    async fn get_mega_tree_by_sha(&self, sha: &str) -> Result<Option<mega_tree::Model>, MegaError> {
        Ok(mega_tree::Entity::find()
            .filter(mega_tree::Column::TreeId.eq(sha))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    #[allow(unused)]
    async fn save_mega_trees(&self, trees: Vec<Tree>) -> Result<(), MegaError> {
        let models: Vec<mega_tree::Model> = trees.into_iter().map(|x| x.into()).collect();
        let mut save_models: Vec<mega_tree::ActiveModel> = Vec::new();
        for mut model in models {
            model.status = MergeStatus::Open;
            save_models.push(model.into_active_model());
        }
        batch_save_model(self.get_connection(), save_models)
            .await
            .unwrap();
        Ok(())
    }

    pub async fn get_trees_by_repo_id(
        &self,
        repo: &Repo,
    ) -> Result<Vec<mega_tree::Model>, MegaError> {
        Ok(mega_tree::Entity::find()
            .filter(mega_tree::Column::RepoId.eq(repo.repo_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_blobs_by_repo_id(
        &self,
        repo: &Repo,
    ) -> Result<Vec<mega_blob::Model>, MegaError> {
        Ok(mega_blob::Entity::find()
            .filter(mega_blob::Column::RepoId.eq(repo.repo_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_tags_by_repo_id(
        &self,
        repo: &Repo,
    ) -> Result<Vec<mega_tag::Model>, MegaError> {
        Ok(mega_tag::Entity::find()
            .filter(mega_tag::Column::RepoId.eq(repo.repo_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_obj_count_by_repo_id(&self, repo: &Repo) -> usize {
        let c_count = mega_commit::Entity::find()
            .filter(mega_commit::Column::RepoId.eq(repo.repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let t_count = mega_tree::Entity::find()
            .filter(mega_tree::Column::RepoId.eq(repo.repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        let bids: Vec<String> = self
            .get_blobs_by_repo_id(repo)
            .await
            .unwrap()
            .into_iter()
            .map(|b| b.blob_id)
            .collect();

        let b_count = raw_blob::Entity::find()
            .filter(raw_blob::Column::Sha1.is_in(bids))
            .count(self.get_connection())
            .await
            .unwrap();

        let tag_count = mega_tag::Entity::find()
            .filter(mega_tag::Column::RepoId.eq(repo.repo_id))
            .count(self.get_connection())
            .await
            .unwrap();

        (c_count + t_count + b_count + tag_count)
            .try_into()
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use ganymede::mega_node::MegaNode;
    use ganymede::model::create_file::CreateFileInfo;

    use crate::storage::mega_storage::MegaStorage;

    #[test]
    pub fn test_node_tree() {
        let cf1 = CreateFileInfo {
            is_directory: true,
            name: String::from("root"),
            path: String::from("/"),
            content: None,
        };
        let cf2 = CreateFileInfo {
            is_directory: true,
            name: String::from("projects"),
            path: String::from("/root"),
            content: None,
        };
        let cf3 = CreateFileInfo {
            is_directory: true,
            name: String::from("mega"),
            path: String::from("/root/projects"),
            content: None,
        };
        let cf4 = CreateFileInfo {
            is_directory: false,
            name: String::from("readme"),
            path: String::from("/root"),
            content: Some(String::from("readme")),
        };
        let cf5 = CreateFileInfo {
            is_directory: true,
            name: String::from("import"),
            path: String::from("/root"),
            content: None,
        };
        let cf6 = CreateFileInfo {
            is_directory: true,
            name: String::from("linux"),
            path: String::from("/root/import"),
            content: None,
        };
        let cfs: Vec<CreateFileInfo> = vec![cf1, cf2, cf3, cf4, cf5, cf6];
        let storage = MegaStorage::mock();
        let root = storage.mega_node_tree(cfs).unwrap();
        print_tree(root, 0);
    }

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
