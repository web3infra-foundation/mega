use std::{cell::RefCell, collections::HashMap, str::FromStr};

use callisto::{
    git_blob, git_commit, git_tag, git_tree, mega_blob, mega_commit, mega_refs, mega_tag, mega_tree,
};
use common::{
    config::MonoConfig,
    utils::{MEGA_BRANCH_NAME, generate_id},
};
use git_internal::{
    hash::ObjectHash,
    internal::{
        metadata::EntryMeta,
        object::{
            ObjectTrait,
            blob::Blob,
            commit::Commit,
            signature::Signature,
            tag::Tag,
            tree::{Tree, TreeItem, TreeItemMode},
            types::ObjectType,
        },
        pack::entry::Entry,
    },
};

/// Helper function to convert commit model data to Commit object
fn commit_from_model(
    commit_id: &str,
    tree: &str,
    parents_id: &serde_json::Value,
    author: Option<String>,
    committer: Option<String>,
    content: Option<String>,
) -> Commit {
    // Parse parents_id JSON array into Vec<ObjectHash>
    let parent_commit_ids: Vec<ObjectHash> =
        match serde_json::from_value::<Vec<String>>(parents_id.clone()) {
            Ok(parents_array) => parents_array
                .into_iter()
                .filter(|s: &String| !s.is_empty())
                .map(|s: String| ObjectHash::from_str(&s).unwrap())
                .collect(),
            Err(_) => Vec::new(),
        };

    Commit {
        id: ObjectHash::from_str(commit_id).unwrap(),
        tree_id: ObjectHash::from_str(tree).unwrap(),
        parent_commit_ids,
        author: Signature::from_data(author.unwrap().into_bytes()).unwrap(),
        committer: Signature::from_data(committer.unwrap().into_bytes()).unwrap(),
        message: content.unwrap(),
    }
}

pub trait IntoMegaModel {
    type MegaTarget;
    fn into_mega_model(self, ext_meta: EntryMeta) -> Self::MegaTarget;
}

pub trait IntoGitModel {
    type GitTarget;
    fn into_git_model(self, ext_meta: EntryMeta) -> Self::GitTarget;
}

pub trait FromMegaModel {
    type MegaSource;
    fn from_mega_model(model: Self::MegaSource) -> Self;
}

pub trait FromGitModel {
    type GitSource;
    fn from_git_model(model: Self::GitSource) -> Self;
}

#[derive(PartialEq, Debug, Clone)]
pub enum GitObject {
    Commit(Commit),
    Tree(Tree),
    Blob(Blob),
    Tag(Tag),
}

#[derive(PartialEq, Debug, Clone)]
pub enum GitObjectModel {
    Commit(git_commit::Model),
    Tree(git_tree::Model),
    Blob(git_blob::Model, Vec<u8>),
    Tag(git_tag::Model),
}

pub enum MegaObjectModel {
    Commit(mega_commit::Model),
    Tree(mega_tree::Model),
    Blob(mega_blob::Model, Vec<u8>),
    Tag(mega_tag::Model),
}

impl GitObject {
    pub fn convert_to_mega_model(self, meta: EntryMeta) -> MegaObjectModel {
        match self {
            GitObject::Commit(commit) => MegaObjectModel::Commit(commit.into_mega_model(meta)),
            GitObject::Tree(tree) => MegaObjectModel::Tree(tree.into_mega_model(meta)),
            GitObject::Blob(blob) => {
                let blob_data = blob.data.clone();
                let mega_model = blob.into_mega_model(meta);
                MegaObjectModel::Blob(mega_model, blob_data)
            }
            GitObject::Tag(tag) => MegaObjectModel::Tag(tag.into_mega_model(meta)),
        }
    }

    pub fn convert_to_git_model(self, meta: EntryMeta) -> GitObjectModel {
        match self {
            GitObject::Commit(commit) => GitObjectModel::Commit(commit.into_git_model(meta)),
            GitObject::Tree(tree) => GitObjectModel::Tree(tree.into_git_model(meta)),
            GitObject::Blob(blob) => {
                let blob_data = blob.data.clone();
                let git_model = blob.into_git_model(meta);
                GitObjectModel::Blob(git_model, blob_data)
            }
            GitObject::Tag(tag) => GitObjectModel::Tag(tag.into_git_model(meta)),
        }
    }
}

pub fn process_entry(entry: Entry) -> GitObject {
    match entry.obj_type {
        ObjectType::Commit => {
            GitObject::Commit(Commit::from_bytes(&entry.data, entry.hash).unwrap())
        }
        ObjectType::Tree => GitObject::Tree(Tree::from_bytes(&entry.data, entry.hash).unwrap()),
        ObjectType::Blob => GitObject::Blob(Blob::from_bytes(&entry.data, entry.hash).unwrap()),
        ObjectType::Tag => GitObject::Tag(Tag::from_bytes(&entry.data, entry.hash).unwrap()),
        _ => unreachable!("can not parse delta!"),
    }
}

impl IntoMegaModel for Blob {
    type MegaTarget = mega_blob::Model;

    /// Converts a Blob object to a mega_blob::Model
    ///
    /// This function creates a new mega_blob::Model from a Blob object.
    /// The resulting model will have a newly generated ID, the blob's ID as string,
    /// and default values for size, commit_id, and name.
    ///
    /// # Returns
    ///
    /// A new mega_blob::Model instance populated with data from the blob
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_blob::Model {
            id: generate_id(),
            blob_id: self.id.to_string(),
            size: 0,
            commit_id: String::new(),
            name: String::new(),
            pack_id: meta.pack_id.unwrap_or_default(),
            file_path: meta.file_path.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            is_delta_in_pack: meta.is_delta.unwrap_or(false),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoMegaModel for Commit {
    type MegaTarget = mega_commit::Model;

    /// Converts a Commit object to a mega_commit::Model
    ///
    /// This function transforms a Commit object into a mega_commit::Model for database storage.
    /// It preserves all relevant commit metadata including tree reference, parent commit IDs,
    /// author information, committer information, and commit message.
    ///
    /// # Returns
    ///
    /// A new mega_commit::Model instance populated with data from the commit
    ///
    /// # Panics
    ///
    /// This function will panic if author or committer signature data cannot be converted to bytes
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_commit::Model {
            id: generate_id(),
            commit_id: self.id.to_string(),
            tree: self.tree_id.to_string(),
            parents_id: self
                .parent_commit_ids
                .iter()
                .map(|x| x.to_string())
                .collect(),
            author: Some(String::from_utf8_lossy(&self.author.to_data().unwrap()).to_string()),
            committer: Some(
                String::from_utf8_lossy(&self.committer.to_data().unwrap()).to_string(),
            ),
            content: Some(self.message.clone()),
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoMegaModel for Tag {
    type MegaTarget = mega_tag::Model;

    /// Converts a Tag object to a mega_tag::Model
    ///
    /// This function transforms a Tag object into a mega_tag::Model for database storage.
    /// It preserves all tag metadata including the referenced object hash, object type,
    /// tag name, tagger information, and tag message.
    ///
    /// # Returns
    ///
    /// A new mega_tag::Model instance populated with data from the tag
    ///
    /// # Panics
    ///
    /// This function will panic if tagger signature data cannot be converted to bytes
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_tag::Model {
            id: generate_id(),
            tag_id: self.id.to_string(),
            object_id: self.object_hash.to_string(),
            object_type: self.object_type.to_string(),
            tag_name: self.tag_name,
            tagger: String::from_utf8_lossy(&self.tagger.to_data().unwrap()).to_string(),
            message: self.message,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoMegaModel for Tree {
    type MegaTarget = mega_tree::Model;

    /// Converts a Tree object to a mega_tree::Model
    ///
    /// This function transforms a Tree object into a mega_tree::Model for database storage.
    /// It serializes the tree structure into binary data and stores essential metadata
    /// like the tree ID. Size is set to 0 and commit_id to an empty string by default.
    ///
    /// # Returns
    ///
    /// A new mega_tree::Model instance populated with data from the tree
    ///
    /// # Panics
    ///
    /// This function will panic if the tree's data cannot be serialized
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_tree::Model {
            id: generate_id(),
            tree_id: self.id.to_string(),
            sub_trees: self.to_data().unwrap(),
            size: 0,
            commit_id: String::new(),
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Blob {
    type GitTarget = git_blob::Model;

    /// Converts a Blob object to a git_blob::Model
    ///
    /// This function creates a new git_blob::Model from a Blob object.
    /// The resulting model will have a newly generated ID, the blob's ID as string,
    /// repository ID set to 0, and default values for size and name.
    ///
    /// # Returns
    ///
    /// A new git_blob::Model instance populated with data from the blob
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_blob::Model {
            id: generate_id(),
            repo_id: 0,
            blob_id: self.id.to_string(),
            size: 0,
            name: None,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            file_path: meta.file_path.unwrap_or_default(),
            is_delta_in_pack: meta.is_delta.unwrap_or(false),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Commit {
    type GitTarget = git_commit::Model;

    /// Converts a Commit object to a git_commit::Model
    ///
    /// This function transforms a Commit object into a git_commit::Model for Git repository
    /// database storage. It preserves all relevant commit metadata including tree reference,
    /// parent commit IDs, author information, committer information, and commit message.
    /// The repository ID is set to 0 by default.
    ///
    /// # Returns
    ///
    /// A new git_commit::Model instance populated with data from the commit
    ///
    /// # Panics
    ///
    /// This function will panic if author or committer signature data cannot be converted to bytes
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_commit::Model {
            id: generate_id(),
            repo_id: 0,
            commit_id: self.id.to_string(),
            tree: self.tree_id.to_string(),
            parents_id: self
                .parent_commit_ids
                .iter()
                .map(|x| x.to_string())
                .collect(),
            author: Some(String::from_utf8_lossy(&self.author.to_data().unwrap()).to_string()),
            committer: Some(
                String::from_utf8_lossy(&self.committer.to_data().unwrap()).to_string(),
            ),
            content: Some(self.message.clone()),
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Tag {
    type GitTarget = git_tag::Model;

    /// Converts a Tag object to a git_tag::Model
    ///
    /// This function transforms a Tag object into a git_tag::Model for Git repository
    /// database storage. It preserves all tag metadata including the referenced object hash,
    /// object type, tag name, tagger information, and tag message.
    /// The repository ID is set to 0 by default.
    ///
    /// # Returns
    ///
    /// A new git_tag::Model instance populated with data from the tag
    ///
    /// # Panics
    ///
    /// This function will panic if tagger signature data cannot be converted to bytes
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_tag::Model {
            id: generate_id(),
            repo_id: 0,
            tag_id: self.id.to_string(),
            object_id: self.object_hash.to_string(),
            object_type: self.object_type.to_string(),
            tag_name: self.tag_name,
            tagger: String::from_utf8_lossy(&self.tagger.to_data().unwrap()).to_string(),
            message: self.message,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Tree {
    type GitTarget = git_tree::Model;

    /// Converts a Tree object to a git_tree::Model
    ///
    /// This function transforms a Tree object into a git_tree::Model for Git repository
    /// database storage. It serializes the tree structure into binary data and stores
    /// essential metadata like the tree ID. The repository ID is set to 0 and size
    /// is set to 0 by default.
    ///
    /// # Returns
    ///
    /// A new git_tree::Model instance populated with data from the tree
    ///
    /// # Panics
    ///
    /// This function will panic if the tree's data cannot be serialized
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_tree::Model {
            id: generate_id(),
            repo_id: 0,
            tree_id: self.id.to_string(),
            sub_trees: self.to_data().unwrap(),
            size: 0,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

pub fn generate_git_keep() -> Blob {
    let git_keep_content = String::from("This file was used to maintain the git tree");
    Blob::from_content(&git_keep_content)
}

pub fn generate_git_keep_with_timestamp() -> Blob {
    let git_keep_content = format!(
        "This file was used to maintain the git tree, generate at:{}",
        chrono::Utc::now().naive_utc()
    );
    Blob::from_content(&git_keep_content)
}

pub fn init_trees(
    mono_config: &MonoConfig,
) -> (HashMap<ObjectHash, Tree>, HashMap<ObjectHash, Blob>, Tree) {
    let mut root_items = Vec::new();
    let mut trees = Vec::new();
    let mut blobs = Vec::new();
    for dir in mono_config.root_dirs.clone() {
        let entity_str =
            saturn::entitystore::generate_entity(&mono_config.admin, &format!("/{dir}")).unwrap();
        let blob = Blob::from_content(&entity_str);

        let tree_item = TreeItem {
            mode: TreeItemMode::Blob,
            id: blob.id,
            name: String::from(".mega_cedar.json"),
        };
        let tree = Tree::from_tree_items(vec![tree_item.clone()]).unwrap();
        root_items.push(TreeItem {
            mode: TreeItemMode::Tree,
            id: tree.id,
            name: dir,
        });
        trees.push(tree);
        blobs.push(blob);
    }

    inject_root_buck_files(&mut root_items, &mut blobs, &mono_config.root_dirs);

    // Ensure the `toolchains` cell has a BUCK file at repo initialization time.
    if let Some(toolchains_root_idx) = root_items.iter().position(|item| {
        item.mode == TreeItemMode::Tree
            && item
                .name
                .trim_start_matches('/')
                .trim_start_matches("./")
                .trim_end_matches('/')
                == "toolchains"
    }) {
        let toolchains_tree_id = root_items[toolchains_root_idx].id;
        if let Some(toolchains_tree_idx) = trees.iter().position(|t| t.id == toolchains_tree_id) {
            let mut toolchains_items = trees[toolchains_tree_idx].tree_items.clone();
            inject_toolchains_buck_file(&mut toolchains_items, &mut blobs);
            let toolchains_tree = Tree::from_tree_items(toolchains_items).unwrap();
            trees[toolchains_tree_idx] = toolchains_tree.clone();
            root_items[toolchains_root_idx].id = toolchains_tree.id;
        }
    }

    let root = Tree::from_tree_items(root_items).unwrap();
    (
        trees.into_iter().map(|x| (x.id, x)).collect(),
        blobs.into_iter().map(|x| (x.id, x)).collect(),
        root,
    )
}

/// Injects Buck configuration files (.buckroot and .buckconfig) into the root directory.
fn inject_root_buck_files(
    root_items: &mut Vec<TreeItem>,
    blobs: &mut Vec<Blob>,
    root_dirs: &[String],
) {
    // .buckroot
    let buckroot_content = generate_buckroot_content();
    let buckroot_blob = Blob::from_content(&buckroot_content);
    root_items.push(TreeItem {
        mode: TreeItemMode::Blob,
        id: buckroot_blob.id,
        name: String::from(".buckroot"),
    });
    blobs.push(buckroot_blob);

    // .buckconfig
    let buckconfig_content = generate_buckconfig_content(root_dirs);
    let buckconfig_blob = Blob::from_content(&buckconfig_content);
    root_items.push(TreeItem {
        mode: TreeItemMode::Blob,
        id: buckconfig_blob.id,
        name: String::from(".buckconfig"),
    });
    blobs.push(buckconfig_blob);
}

/// Injects a BUCK file into the toolchains directory.
fn inject_toolchains_buck_file(toolchains_items: &mut Vec<TreeItem>, blobs: &mut Vec<Blob>) {
    let toolchains_content = generate_toolchains_buck_content();
    let toolchains_blob = Blob::from_content(&toolchains_content);
    toolchains_items.push(TreeItem {
        mode: TreeItemMode::Blob,
        id: toolchains_blob.id,
        name: String::from("BUCK"),
    });
    blobs.push(toolchains_blob);
}

fn generate_toolchains_buck_content() -> String {
    r#"load("@prelude//toolchains:demo.bzl", "system_demo_toolchains")

# All the default toolchains, suitable for a quick demo or early prototyping.
# Most real projects should copy/paste the implementation to configure them.
system_demo_toolchains()
"#
    .to_string()
}

fn generate_buckroot_content() -> String {
    // The .buckroot file is usually empty or contains a simple identifier.
    String::new()
}

/// Directories that should be excluded from Buck cell definitions.
/// Currently only "doc" is excluded to preserve existing behavior.
const EXCLUDED_BUCK_CELL_DIRS: &[&str] = &["doc"];

fn generate_buckconfig_content(root_dirs: &[String]) -> String {
    let normalized_root_dirs = root_dirs
        .iter()
        .map(|d| {
            d.trim_start_matches('/')
                .trim_start_matches("./")
                .trim_end_matches('/')
                .to_string()
        })
        .filter(|d| !EXCLUDED_BUCK_CELL_DIRS.contains(&d.as_str()))
        .collect::<std::collections::HashSet<_>>();

    let mut cells_lines = vec!["  root = .".to_string(), "  prelude = prelude".to_string()];

    for dir in ["toolchains", "project", "release", "third-party"] {
        if normalized_root_dirs.contains(dir) {
            cells_lines.push(format!("  {dir} = {dir}"));
        }
    }

    cells_lines.push("  buckal = toolchains/buckal-bundles".to_string());
    cells_lines.push("  none = none".to_string());

    let cells = cells_lines.join("\n");

    format!(
        r#"[cells]
{cells}

[cell_aliases]
  config = prelude
  ovr_config = prelude
  fbcode = none
  fbsource = none
  fbcode_macros = none
  buck = none

# Uses a copy of the prelude bundled with the buck2 binary. You can alternatively delete this
# section and vendor a copy of the prelude to the `prelude` directory of your project.
[external_cells]
  prelude = bundled

[parser]
  target_platform_detector_spec = target:root//...->prelude//platforms:default \
    target:prelude//...->prelude//platforms:default \
    target:toolchains//...->prelude//platforms:default

[build]
  execution_platforms = prelude//platforms:default
  default_target_platforms = prelude//platforms:default
"#
    )
}

pub struct MegaModelConverter {
    pub commit: Commit,
    pub root_tree: Tree,
    pub tree_maps: HashMap<ObjectHash, Tree>,
    pub blob_maps: HashMap<ObjectHash, Blob>,
    pub mega_trees: RefCell<HashMap<ObjectHash, mega_tree::ActiveModel>>,
    pub mega_blobs: RefCell<HashMap<ObjectHash, mega_blob::ActiveModel>>,
    pub raw_blobs: RefCell<Vec<Blob>>,
    pub refs: mega_refs::ActiveModel,
}

impl MegaModelConverter {
    fn traverse_from_root(&self) {
        let root_tree = &self.root_tree;
        let mut mega_tree: mega_tree::Model = root_tree.clone().into_mega_model(EntryMeta::new());
        mega_tree.commit_id = self.commit.id.to_string();
        self.mega_trees
            .borrow_mut()
            .insert(root_tree.id, mega_tree.clone().into());
        self.traverse_for_update(root_tree);
    }

    fn traverse_for_update(&self, tree: &Tree) {
        for item in &tree.tree_items {
            if item.mode == TreeItemMode::Tree {
                let child_tree = self.tree_maps.get(&item.id).unwrap();
                let mut mega_tree: mega_tree::Model =
                    child_tree.clone().into_mega_model(EntryMeta::new());
                mega_tree.commit_id = self.commit.id.to_string();
                self.mega_trees
                    .borrow_mut()
                    .insert(child_tree.id, mega_tree.clone().into());
                self.traverse_for_update(child_tree);
            } else {
                let blob = self.blob_maps.get(&item.id).unwrap();
                let mut mega_blob: mega_blob::Model =
                    blob.clone().into_mega_model(EntryMeta::new());
                mega_blob.commit_id = self.commit.id.to_string();
                self.mega_blobs
                    .borrow_mut()
                    .insert(blob.id, mega_blob.clone().into());

                self.raw_blobs.borrow_mut().push(blob.clone());
            }
        }
    }

    pub fn init(mono_config: &MonoConfig) -> Self {
        let (tree_maps, blob_maps, root_tree) = init_trees(mono_config);
        let commit = Commit::from_tree_id(root_tree.id, vec![], "\nInit Mega Directory");

        let mega_ref = mega_refs::Model {
            id: generate_id(),
            path: "/".to_owned(),
            ref_name: MEGA_BRANCH_NAME.to_owned(),
            ref_commit_hash: commit.id.to_string(),
            ref_tree_hash: commit.tree_id.to_string(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            is_cl: false,
        };

        let converter = MegaModelConverter {
            commit,
            root_tree,
            tree_maps,
            blob_maps,
            mega_trees: RefCell::new(HashMap::new()),
            mega_blobs: RefCell::new(HashMap::new()),
            raw_blobs: RefCell::new(Vec::new()),
            refs: mega_ref.into(),
        };
        converter.traverse_from_root();
        converter
    }
}

// Reverse conversion implementations
impl FromMegaModel for Tag {
    type MegaSource = mega_tag::Model;

    /// Converts a mega_tag::Model to a Tag object
    ///
    /// This function reconstructs a Tag object from a mega_tag::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to a Signature.
    ///
    /// # Arguments
    ///
    /// * `model` - The mega_tag::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tag instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tag_id string cannot be parsed into a valid ObjectHash
    /// - The object_id string cannot be parsed into a valid ObjectHash
    /// - The object_type string is not a recognized ObjectType
    /// - The tagger string cannot be converted into a valid Signature
    fn from_mega_model(model: Self::MegaSource) -> Self {
        Tag {
            id: ObjectHash::from_str(&model.tag_id).expect("Invalid tag_id in database"),
            object_hash: ObjectHash::from_str(&model.object_id).unwrap(),
            object_type: ObjectType::from_string(&model.object_type).unwrap(),
            tag_name: model.tag_name,
            tagger: Signature::from_data(model.tagger.into_bytes()).unwrap(),
            message: model.message,
        }
    }
}

impl FromGitModel for Tag {
    type GitSource = git_tag::Model;

    /// Converts a git_tag::Model to a Tag object
    ///
    /// This function reconstructs a Tag object from a git_tag::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to a Signature.
    ///
    /// # Arguments
    ///
    /// * `model` - The git_tag::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tag instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tag_id string cannot be parsed into a valid ObjectHash
    /// - The object_id string cannot be parsed into a valid ObjectHash
    /// - The object_type string is not a recognized ObjectType
    /// - The tagger string cannot be converted into a valid Signature
    fn from_git_model(model: Self::GitSource) -> Self {
        Tag {
            id: ObjectHash::from_str(&model.tag_id).unwrap(),
            object_hash: ObjectHash::from_str(&model.object_id).unwrap(),
            object_type: ObjectType::from_string(&model.object_type).unwrap(),
            tag_name: model.tag_name,
            tagger: Signature::from_data(model.tagger.into_bytes()).unwrap(),
            message: model.message,
        }
    }
}

impl FromMegaModel for Tree {
    type MegaSource = mega_tree::Model;

    /// Converts a mega_tree::Model to a Tree object
    ///
    /// This function reconstructs a Tree object from a mega_tree::Model retrieved from the database.
    /// It parses the binary sub_trees data back into a structured Tree object and
    /// uses the tree_id string to recreate the ObjectHash identifier.
    ///
    /// # Arguments
    ///
    /// * `model` - The mega_tree::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tree instance reconstructed from the model data
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tree_id string cannot be parsed into a valid ObjectHash
    /// - The binary sub_trees data cannot be parsed into a valid Tree structure
    fn from_mega_model(model: Self::MegaSource) -> Self {
        Tree::from_bytes(
            &model.sub_trees,
            ObjectHash::from_str(&model.tree_id).unwrap(),
        )
        .unwrap()
    }
}

impl FromGitModel for Tree {
    type GitSource = git_tree::Model;

    /// Converts a git_tree::Model to a Tree object
    ///
    /// This function reconstructs a Tree object from a git_tree::Model retrieved from the database.
    /// It parses the binary sub_trees data back into a structured Tree object and
    /// uses the tree_id string to recreate the ObjectHash hash identifier.
    ///
    /// # Arguments
    ///
    /// * `model` - The git_tree::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tree instance reconstructed from the model data
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tree_id string cannot be parsed into a valid ObjectHash
    /// - The binary sub_trees data cannot be parsed into a valid Tree structure
    fn from_git_model(model: Self::GitSource) -> Self {
        Tree::from_bytes(
            &model.sub_trees,
            ObjectHash::from_str(&model.tree_id).unwrap(),
        )
        .unwrap()
    }
}

impl FromMegaModel for Commit {
    type MegaSource = mega_commit::Model;

    /// Converts a mega_commit::Model to a Commit object
    ///
    /// This function reconstructs a Commit object from a mega_commit::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to Signature objects.
    ///
    /// # Arguments
    ///
    /// * `model` - The mega_commit::Model to convert
    ///
    /// # Returns
    ///
    /// A new Commit instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The commit_id string cannot be parsed into a valid ObjectHash
    /// - The tree string cannot be parsed into a valid ObjectHash
    /// - Any parent ID in parents_id cannot be parsed into a valid ObjectHash
    /// - The author or committer strings cannot be converted into valid Signatures
    fn from_mega_model(model: Self::MegaSource) -> Self {
        commit_from_model(
            &model.commit_id,
            &model.tree,
            &model.parents_id,
            model.author,
            model.committer,
            model.content,
        )
    }
}

impl FromGitModel for Commit {
    type GitSource = git_commit::Model;

    /// Converts a git_commit::Model to a Commit object
    ///
    /// This function reconstructs a Commit object from a git_commit::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to Signature objects.
    ///
    /// # Arguments
    ///
    /// * `model` - The git_commit::Model to convert
    ///
    /// # Returns
    ///
    /// A new Commit instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The commit_id string cannot be parsed into a valid ObjectHash hash
    /// - The tree string cannot be parsed into a valid ObjectHash hash
    /// - Any parent ID in parents_id cannot be parsed into a valid ObjectHash hash
    /// - The author or committer strings cannot be converted into valid Signatures
    fn from_git_model(model: Self::GitSource) -> Self {
        commit_from_model(
            &model.commit_id,
            &model.tree,
            &model.parents_id,
            model.author,
            model.committer,
            model.content,
        )
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use common::config::MonoConfig;
    use git_internal::{hash::ObjectHash, internal::object::commit::Commit};

    use crate::utils::converter::MegaModelConverter;

    #[test]
    pub fn test_init_mega_dir() {
        let mut mono_config = MonoConfig::default();
        if !mono_config.root_dirs.iter().any(|d| d == "toolchains") {
            mono_config.root_dirs.push("toolchains".to_string());
        }
        let converter = MegaModelConverter::init(&mono_config);
        let mega_trees = converter.mega_trees.borrow().clone();
        let mega_blobs = converter.mega_blobs.borrow().clone();
        let dir_nums = mono_config.root_dirs.len();
        assert_eq!(mega_trees.len(), dir_nums + 1);
        assert_eq!(mega_blobs.len(), dir_nums + 3);
    }

    #[test]
    pub fn test_init_commit() {
        let commit = Commit::from_tree_id(
            ObjectHash::from_str("bd4a28f2d8b2efc371f557c3b80d320466ed83f3").unwrap(),
            vec![],
            "\nInit Mega Directory",
        );
        println!("{commit}");
    }
}
