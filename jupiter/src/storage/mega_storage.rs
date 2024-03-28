use std::collections::VecDeque;
use std::rc::Rc;
use std::{env, sync::Arc};

use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter,
};

use callisto::db_enums::{ConvType, MergeStatus};
use callisto::{
    git_repo, mega_blob, mega_commit, mega_mr, mega_mr_comment, mega_mr_conv, mega_refs, mega_tag,
    mega_tree, raw_blob,
};
use common::errors::MegaError;
use common::utils::generate_id;
use ganymede::mega_node::MegaNode;
use ganymede::model::converter::MegaModelConverter;
use ganymede::model::create_file::CreateFileInfo;
use venus::internal::object::GitObjectModel;
use venus::internal::{object::commit::Commit, pack::entry::Entry};
use venus::monorepo::mega_refs::MegaRefs;
use venus::monorepo::mr::MergeRequest;
use venus::repo::Repo;

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

    pub async fn remove_ref(&self, path: &str, ref_commit_hash: &str) -> Result<(), MegaError> {
        mega_refs::Entity::delete_many()
            .filter(mega_refs::Column::Path.eq(path))
            .filter(mega_refs::Column::RefCommitHash.eq(ref_commit_hash))
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

    pub async fn update_ref(
        &self,
        refs: MegaRefs
    ) -> Result<(), MegaError> {
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
                GitObjectModel::Commit(mut commit) => {
                    commit.repo_id = 0;
                    commits.push(commit.into_active_model())
                }
                GitObjectModel::Tree(mut tree) => {
                    tree.repo_id = 0;
                    trees.push(tree.clone().into_active_model());
                }
                GitObjectModel::Blob(mut blob, raw) => {
                    blob.repo_id = 0;
                    blobs.push(blob.clone().into_active_model());
                    raw_blobs.push(raw.into_active_model());
                }
                GitObjectModel::Tag(mut tag) => {
                    tag.repo_id = 0;
                    tags.push(tag.into_active_model())
                }
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

    pub async fn create_mega_file(&self, _file_info: CreateFileInfo) -> Result<(), MegaError> {
        // let mut save_trees: Vec<mega_tree::ActiveModel> = Vec::new();
        // let mut mega_tree = self
        //     .get_tree_by_path(&file_info.path, "")
        //     .await
        //     .unwrap()
        //     .unwrap();
        // let mut p_tree: Tree = mega_tree.clone().into();

        // let new_item = if file_info.is_directory {
        //     let blob = converter::generate_git_keep();
        //     let tree_item = TreeItem {
        //         mode: TreeItemMode::Blob,
        //         id: blob.id,
        //         name: String::from(".gitkeep"),
        //     };
        //     let child_tree = Tree::from_tree_items(vec![tree_item]).unwrap();
        //     TreeItem {
        //         mode: TreeItemMode::Tree,
        //         id: child_tree.id,
        //         name: file_info.name.clone(),
        //     }
        // } else {
        //     let blob = Blob::from_content(&file_info.content.unwrap());
        //     TreeItem {
        //         mode: TreeItemMode::Blob,
        //         id: blob.id,
        //         name: file_info.name.clone(),
        //     }
        // };
        // p_tree.tree_items.push(new_item);
        // let mut new_tree = Tree::from_tree_items(p_tree.tree_items).unwrap();
        // let model: mega_tree::Model = new_tree.clone().into();
        // save_trees.push(model.into_active_model());

        // while let Some(parent_id) = mega_tree.parent_id {
        //     let replace_name = mega_tree.name;
        //     mega_tree = self
        //         .get_tree_by_hash(&Repo::empty(), &parent_id)
        //         .await
        //         .unwrap()
        //         .unwrap();
        //     let mut tmp: Tree = mega_tree.clone().into();
        //     if let Some(item) = tmp.tree_items.iter_mut().find(|x| x.name == replace_name) {
        //         item.id = new_tree.id;
        //     }

        //     new_tree = Tree::from_tree_items(tmp.tree_items).unwrap();
        //     let model: mega_tree::Model = new_tree.clone().into();
        //     save_trees.push(model.into_active_model());
        // }

        // // save_trees
        // //     .iter()
        // //     .for_each(|x| println!("{:?}, {:?}", x.name, x.tree_id));
        // let repo = Repo::empty();
        // let refs = &self.get_ref(&repo).await.unwrap()[0];
        // let commit = Commit::from_tree_id(
        //     SHA1::from_str(&mega_tree.tree_id).unwrap(),
        //     vec![SHA1::from_str(&refs.ref_hash).unwrap()],
        //     &format!("create file {} commit", file_info.name),
        // );
        // // update ref
        // self.update_ref(&repo, "main", &commit.id.to_plain_str())
        //     .await
        //     .unwrap();
        // self.save_mega_commits(&Repo::empty(), &MergeRequest::empty(), vec![commit])
        //     .await
        //     .unwrap();
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

    pub async fn save_mega_commits(
        &self,
        repo: &Repo,
        commits: Vec<Commit>,
    ) -> Result<(), MegaError> {
        let mega_commits: Vec<mega_commit::Model> =
            commits.into_iter().map(mega_commit::Model::from).collect();
        let mut save_models = Vec::new();
        for mut mega_commit in mega_commits {
            mega_commit.repo_id = repo.repo_id;
            save_models.push(mega_commit.into_active_model());
        }
        batch_save_model(self.get_connection(), save_models)
            .await
            .unwrap();
        Ok(())
    }

    pub async fn get_commit_by_hash(
        &self,
        repo: &Repo,
        hash: &str,
    ) -> Result<Option<mega_commit::Model>, MegaError> {
        Ok(mega_commit::Entity::find()
            .filter(mega_commit::Column::RepoId.eq(repo.repo_id))
            .filter(mega_commit::Column::CommitId.eq(hash))
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

    pub async fn get_tree_by_path(
        &self,
        full_path: &str,
        ref_commit_hash: &str,
    ) -> Result<Option<mega_tree::Model>, MegaError> {
        Ok(mega_tree::Entity::find()
            .filter(mega_tree::Column::FullPath.eq(full_path))
            .filter(mega_tree::Column::CommitId.eq(ref_commit_hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_tree_by_hash(
        &self,
        repo: &Repo,
        sha: &str,
    ) -> Result<Option<mega_tree::Model>, MegaError> {
        Ok(mega_tree::Entity::find()
            .filter(mega_tree::Column::RepoId.eq(repo.repo_id))
            .filter(mega_tree::Column::TreeId.eq(sha))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    // #[allow(unused)]
    // async fn save_mega_trees(&self, trees: Vec<Tree>) -> Result<(), MegaError> {
    //     let models: Vec<mega_tree::Model> = trees.into_iter().map(|x| x.into()).collect();
    //     let mut save_models: Vec<mega_tree::ActiveModel> = Vec::new();
    //     for mut model in models {
    //         model.status = MergeStatus::Open;
    //         save_models.push(model.into_active_model());
    //     }
    //     batch_save_model(self.get_connection(), save_models)
    //         .await
    //         .unwrap();
    //     Ok(())
    // }

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
