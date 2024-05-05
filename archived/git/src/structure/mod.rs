use std::path::{Path, PathBuf};

use storage::utils::id_generator::{self, generate_id};
use entity::{commit, node};
use sea_orm::{ActiveValue::NotSet, Set};

use crate::{
    hash::Hash,
    internal::{
        object::{
            blob::Blob,
            commit::Commit,
            meta::Meta,
            tree::{Tree, TreeItem, TreeItemMode},
            ObjectT,
        },
        ObjectType,
    },
};

use self::nodes::{FileNode, Node, TreeNode};

pub mod conversion;
pub mod nodes;
/// only blob and tree should implement this trait
pub trait GitNodeObject {
    fn convert_to_node(
        &self,
        item: Option<&TreeItem>,
        repo_path: PathBuf,
        full_path: PathBuf,
        last_commit: &str,
    ) -> Box<dyn Node>;

    fn generate_id(&self) -> i64 {
        id_generator::generate_id()
    }
}

impl GitNodeObject for Blob {
    fn convert_to_node(
        &self,
        item: Option<&TreeItem>,
        repo_path: PathBuf,
        full_path: PathBuf,
        last_commit: &str,
    ) -> Box<dyn Node> {
        Box::new(FileNode {
            nid: self.generate_id(),
            pid: "".to_owned(),
            git_id: self.id.to_plain_str(),
            last_commit: last_commit.to_owned(),
            repo_path,
            mode: if let Some(item) = item {
                item.mode.to_bytes().to_vec()
            } else {
                Vec::new()
            },
            name: if let Some(item) = item {
                item.name.clone()
            } else {
                "".to_owned()
            },
            size: self.data.len().try_into().unwrap(),
            full_path,
        })
    }

}

impl Commit {
    pub fn subdir_commit(meta: Vec<u8>, tree_id: Hash) -> Commit {
        let mut c = Commit::new_from_meta(Meta::new_from_data_with_object_type(
            ObjectType::Commit,
            meta,
        ))
        .unwrap();
        c.tree_id = tree_id;
        c.parent_commit_ids.clear();
        c.id = Meta::calculate_id(ObjectType::Commit, &c.to_data().unwrap());
        c
    }

    pub fn convert_to_model(&self, repo_path: &Path) -> commit::ActiveModel {
        let pid = self
            .parent_commit_ids
            .iter()
            .map(|id| id.to_plain_str())
            .collect::<Vec<_>>();

        commit::ActiveModel {
            id: NotSet,
            git_id: Set(self.id.to_plain_str()),
            tree: Set(self.tree_id.to_plain_str()),
            pid: Set(pid),
            repo_path: Set(repo_path.to_str().unwrap().to_owned()),
            author: Set(Some(
                String::from_utf8(self.author.to_data().unwrap()).unwrap(),
            )),
            committer: Set(Some(
                String::from_utf8(self.committer.to_data().unwrap()).unwrap(),
            )),
            content: Set(Some(self.message.clone())),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }
}

impl GitNodeObject for Tree {

    fn convert_to_node(
        &self,
        item: Option<&TreeItem>,
        repo_path: PathBuf,
        full_path: PathBuf,
        last_commit: &str,
    ) -> Box<dyn Node> {
        Box::new(TreeNode {
            nid: generate_id(),
            pid: "".to_owned(),
            git_id: self.id.to_plain_str(),
            last_commit: last_commit.to_owned(),
            name: if let Some(item) = item {
                item.name.clone()
            } else {
                "".to_owned()
            },
            repo_path,
            mode: if let Some(item) = item {
                item.mode.to_bytes().to_vec()
            } else {
                Vec::new()
            },
            children: Vec::new(),
            size: self.get_raw().len().try_into().unwrap(),
            full_path,
        })
    }
}

impl TreeItem {
    pub fn convert_from_model(model: node::Model) -> TreeItem {
        let mode = if model.node_type == "tree" {
            TreeItemMode::Tree
        } else {
            TreeItemMode::Blob
        };
        TreeItem {
            mode,
            id: Hash::new_from_bytes(model.git_id.as_bytes()),
            name: model.name.unwrap(),
        }
    }
}

