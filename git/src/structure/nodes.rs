use std::{
    any::Any,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Ok;
use async_recursion::async_recursion;
use common::errors::MegaError;
use database::{
    driver::ObjectStorage,
    utils::id_generator::{self, generate_id},
};
use entity::{commit, node};
use sea_orm::{ActiveValue::NotSet, Set};

use crate::{
    hash::Hash,
    internal::object::{
        blob::Blob,
        commit::Commit,
        tree::{Tree, TreeItemMode},
        ObjectT,
    },
};

use super::GitNodeObject;

pub struct NodeBuilder {
    // pub repo_root: Box<dyn Node>,
    pub storage: Arc<dyn ObjectStorage>,
    pub tree_map: HashMap<Hash, Tree>,
    pub blob_map: HashMap<Hash, Blob>,
    pub repo_path: PathBuf,
    pub commits: Vec<Commit>,
}

pub struct TreeNode {
    pub nid: i64,
    pub pid: String,
    pub git_id: String,
    pub last_commit: String,
    pub name: String,
    pub repo_path: PathBuf,
    pub mode: Vec<u8>,
    pub children: Vec<Box<dyn Node>>,
    pub size: i32,
    pub full_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub nid: i64,
    pub pid: String,
    pub git_id: String,
    pub last_commit: String,
    pub name: String,
    pub repo_path: PathBuf,
    pub mode: Vec<u8>,
    pub size: i32,
    pub full_path: PathBuf,
}

/// define the node common behaviour
pub trait Node: Send {
    fn get_id(&self) -> i64;

    fn get_pid(&self) -> &str;

    fn get_git_id(&self) -> &str;

    fn get_commit_id(&self) -> &str;

    fn get_name(&self) -> &str;

    fn get_mode(&self) -> Vec<u8>;

    fn get_children(&self) -> &Vec<Box<dyn Node>>;

    fn generate_id(&self) -> i64 {
        id_generator::generate_id()
    }

    fn new(name: String, pid: String) -> Self
    where
        Self: Sized;

    fn find_child(&mut self, name: &str) -> Option<&mut Box<dyn Node>>;

    fn add_child(&mut self, child: Box<dyn Node>);

    fn is_a_directory(&self) -> bool;

    fn as_any(&self) -> &dyn Any;

    // since we use lazy load, need manually fetch data, and might need to use a LRU cache to store the data?
    fn read_data(&self) -> String {
        "".to_string()
    }

    fn convert_to_model(&self) -> node::ActiveModel;

    fn set_commit_id(&mut self, commit_id: String);

}

impl Node for TreeNode {
    fn get_id(&self) -> i64 {
        self.nid
    }
    fn get_pid(&self) -> &str {
        &self.pid
    }

    fn get_git_id(&self) -> &str {
        &self.git_id
    }

    fn get_commit_id(&self) -> &str {
        &self.last_commit
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_mode(&self) -> Vec<u8> {
        self.mode.clone()
    }

    fn get_children(&self) -> &Vec<Box<dyn Node>> {
        &self.children
    }

    fn new(name: String, pid: String) -> TreeNode {
        TreeNode {
            nid: generate_id(),
            pid,
            name,
            repo_path: PathBuf::new(),
            full_path: PathBuf::new(),
            mode: Vec::new(),
            git_id: String::new(),
            last_commit: String::new(),
            children: Vec::new(),
            size: 0,
        }
    }

    /// convert children relations to data vec
    fn convert_to_model(&self) -> node::ActiveModel {
        // tracing::info!("tree {}", Arc::strong_count(&self.data));
        // tracing::info!("tree {}", Arc::strong_count(&Arc::clone(&self.data)));
        node::ActiveModel {
            id: NotSet,
            node_id: Set(self.nid),
            git_id: Set(self.git_id.clone()),
            last_commit: Set(self.last_commit.clone()),
            node_type: Set("tree".to_owned()),
            name: Set(Some(self.name.to_string())),
            mode: Set(self.mode.clone()),
            content_sha: NotSet,
            repo_path: Set(self.repo_path.to_string_lossy().into_owned()),
            full_path: Set(self.full_path.to_string_lossy().into_owned()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
            size: Set(self.size),
        }
    }

    fn find_child(&mut self, name: &str) -> Option<&mut Box<dyn Node>> {
        self.children.iter_mut().find(|c| c.get_name() == name)
    }

    fn add_child(&mut self, content: Box<dyn Node>) {
        self.children.push(content);
    }

    fn is_a_directory(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn set_commit_id(&mut self, commit_id: String) {
        self.last_commit = commit_id
    }

}

impl Node for FileNode {
    fn get_id(&self) -> i64 {
        self.nid
    }

    fn get_pid(&self) -> &str {
        &self.pid
    }

    fn get_git_id(&self) -> &str {
        &self.git_id
    }

    fn get_commit_id(&self) -> &str {
        &self.last_commit
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_mode(&self) -> Vec<u8> {
        self.mode.clone()
    }

    fn get_children(&self) -> &Vec<Box<dyn Node>> {
        panic!("not supported")
    }

    fn new(name: String, pid: String) -> FileNode {
        FileNode {
            nid: generate_id(),
            pid,
            last_commit: String::new(),
            repo_path: PathBuf::new(),
            full_path: PathBuf::new(),
            name,
            git_id: String::new(),
            mode: Vec::new(),
            size: 0,
        }
    }

    fn convert_to_model(&self) -> node::ActiveModel {
        node::ActiveModel {
            id: NotSet,
            node_id: Set(self.nid),
            git_id: Set(self.git_id.clone()),
            last_commit: Set(self.last_commit.clone()),
            node_type: Set("blob".to_owned()),
            name: Set(Some(self.name.clone())),
            mode: Set(self.mode.clone()),
            content_sha: NotSet,
            repo_path: Set(self.repo_path.to_string_lossy().into_owned()),
            full_path: Set(self.full_path.to_string_lossy().into_owned()),
            size: Set(self.size),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }

    fn find_child(&mut self, _: &str) -> Option<&mut Box<dyn Node>> {
        panic!("not supported")
    }

    fn add_child(&mut self, _: Box<dyn Node>) {
        panic!("not supported")
    }

    fn is_a_directory(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn set_commit_id(&mut self, commit_id: String) {
        self.last_commit = commit_id
    }

}

impl TreeNode {
    // since root tree doesn't have name, we can only use node id to build it.
    pub fn get_root_from_nid(nid: i64) -> Box<dyn Node> {
        Box::new(TreeNode {
            nid,
            pid: "".to_owned(),
            git_id: String::new(),
            last_commit: String::new(),
            name: "".to_owned(),
            repo_path: PathBuf::from("/"),
            full_path: PathBuf::from("/"),
            mode: Vec::new(),
            children: Vec::new(),
            size: 0,
        })
    }
}

impl NodeBuilder {
    /// this method is used to build node tree and persist node data to database. Conversion order:
    /// 1. Git TreeItem => Struct Node => DB Model
    /// 2. Git Blob => DB Model
    /// current: protocol => storage => structure
    /// expected: protocol => structure => storage
    pub async fn build_node_tree(&self) -> Result<Vec<node::ActiveModel>, anyhow::Error> {
        let mut nodes = Vec::new();
        let mut tree_build_cache = HashSet::new();
        // let mut root_node_map = HashMap::new();

        for commit in self.commits.iter().rev() {
            let root_tree_id = commit.tree_id;
            let mut root_node;

            if !tree_build_cache.contains(&root_tree_id) {
                //fetch the tree which commit points to

                let tree = match self.tree_map.get(&root_tree_id) {
                    Some(tree) => tree.to_owned(),
                    None => {
                        let model = self
                            .storage
                            .get_obj_data_by_id(&root_tree_id.to_plain_str())
                            .await
                            .unwrap()
                            .unwrap();
                        let mut obj = Tree::new_from_data(model.data.clone());
                        let hash = Hash::new_from_str(&model.git_id);
                        obj.set_hash(hash);
                        obj
                    }
                };

                root_node =
                    tree.convert_to_node(None, self.repo_path.clone(), self.repo_path.clone(), &commit.id.to_plain_str());
                self.convert_tree_to_node(
                    &tree,
                    &mut root_node,
                    &mut self.repo_path.clone(),
                    &mut tree_build_cache,
                )
                .await;

                nodes.extend(convert_node_to_model(root_node.as_ref(), 0));
                // root_node_map.insert(root_tree_id, root_node);
                tree_build_cache.insert(root_tree_id);
            } else {
                // update related last commit
                // root_node_map.get_mut(&root_tree_id).unwrap().set_commit_id(commit.id.to_plain_str());
            }
        }

        // for (_, root_node) in root_node_map {
        //     nodes.extend(convert_node_to_model(root_node.as_ref(), 0));
        // }
        Ok(nodes)
    }

    /// convert Git TreeItem => Struct Node and build node tree
    #[async_recursion]
    pub async fn convert_tree_to_node(
        &self,
        tree: &Tree,
        node: &mut Box<dyn Node>,
        full_path: &mut PathBuf,
        tree_build_cache: &mut HashSet<Hash>,
    ) {
        for item in &tree.tree_items {
            if tree_build_cache.get(&item.id).is_some() {
                continue;
            }
            full_path.push(item.name.clone());
            if item.mode == TreeItemMode::Tree {
                let sub_tree = match self.tree_map.get(&item.id) {
                    Some(tree) => tree,
                    None => continue,
                };

                node.add_child(sub_tree.convert_to_node(
                    Some(item),
                    self.repo_path.clone(),
                    full_path.clone(),
                    node.get_commit_id()
                ));
                let child_node = match node.find_child(&item.name) {
                    Some(child) => child,
                    None => panic!("Something wrong!:{}", &item.name),
                };
                self.convert_tree_to_node(sub_tree, child_node, full_path, tree_build_cache)
                    .await;
            } else {
                let blob = match self.blob_map.get(&item.id) {
                    Some(blob) => blob,
                    None => continue,
                };
                node.add_child(blob.convert_to_node(
                    Some(item),
                    self.repo_path.to_path_buf(),
                    full_path.clone(),
                    node.get_commit_id()
                ));
            }
            full_path.pop();

            tree_build_cache.insert(item.id);
        }
    }

    pub async fn save_commits(&self) -> Result<bool, MegaError> {
        let save_models: Vec<commit::ActiveModel> = self
            .commits
            .iter()
            .map(|commit| commit.convert_to_model(&self.repo_path))
            .collect();
        self.storage.save_commits(save_models).await
    }

    pub async fn save_nodes(&self, nodes: Vec<node::ActiveModel>) -> Result<bool, MegaError> {
        self.storage.save_nodes(nodes).await
    }
}

/// conver Node to db entity and for later persistent
pub fn convert_node_to_model(node: &dyn Node, _depth: u32) -> Vec<node::ActiveModel> {
    // print_node(node, depth);
    let mut nodes: Vec<node::ActiveModel> = Vec::new();
    nodes.push(node.convert_to_model());
    if node.is_a_directory() {
        for child in node.get_children() {
            nodes.extend(convert_node_to_model(child.as_ref(), _depth + 1));
        }
    }
    nodes
}

// Model => Node => Tree ?
// pub fn model_to_node(nodes_model: &Vec<node::Model>, pid: &str) -> Vec<Box<dyn Node>> {
//     let mut nodes: Vec<Box<dyn Node>> = Vec::new();
//     for model in nodes_model {
//         if model.pid == pid {
//             if model.node_type == "blob" {
//                 nodes.push(FileNode::convert_from_model(model.clone(), Vec::new()));
//             } else {
//                 let childs = model_to_node(nodes_model, &model.pid);
//                 nodes.push(TreeNode::convert_from_model(model.clone(), childs));
//             }
//         }
//     }
//     nodes
// }

/// Print a node with format.
#[allow(unused)]
pub fn print_node(node: &dyn Node, depth: u32) {
    if depth == 0 {
        tracing::debug!("{}", node.get_name());
    } else {
        tracing::debug!(
            "{:indent$}└── {} {}",
            "",
            node.get_name(),
            node.get_id(),
            indent = ((depth as usize) - 1) * 4
        );
    }
}

#[cfg(test)]
mod test {
    // use crate::mega::driver::{
    //     structure::nodes::{Node, TreeNode},
    //     utils::id_generator,
    // };
    use std::path::PathBuf;

    use database::utils::id_generator;

    use super::{FileNode, Node, TreeNode};

    #[test]
    pub fn main() {
        // Form our INPUT:  a list of paths.
        let paths = vec![
            PathBuf::from("child1/grandchild1.txt"),
            PathBuf::from("child1/grandchild2.txt"),
            PathBuf::from("child2/grandchild3.txt"),
            PathBuf::from("child3"),
        ];
        println!("Input Paths:\n{:#?}\n", paths);
        id_generator::set_up_options().unwrap();
        // let mut root = init_root();
        // for path in paths.iter() {
        //     build_tree(&mut root, path, 0)
        // }

        // let mut save_models: Vec<node::ActiveModel> = Vec::new();

        // traverse_node(root.as_ref(), 0, &mut save_models);
    }

    #[allow(unused)]
    fn build_tree(node: &mut Box<dyn Node>, path: &PathBuf, depth: usize) {
        let parts: Vec<&str> = path.to_str().unwrap().split('/').collect();

        if depth < parts.len() {
            let child_name = parts[depth];

            let child = match node.find_child(child_name) {
                Some(child) => child,
                None => {
                    if path.is_file() {
                        node.add_child(Box::new(FileNode::new(
                            child_name.to_owned(),
                            "".to_owned(),
                        )));
                    } else {
                        node.add_child(Box::new(TreeNode::new(
                            child_name.to_owned(),
                            "".to_owned(),
                        )));
                    };
                    match node.find_child(child_name) {
                        Some(child) => child,
                        None => panic!("Something wrong!:{}, {}", &child_name, depth),
                    }
                }
            };
            build_tree(child, path, depth + 1);
        }
    }
}
