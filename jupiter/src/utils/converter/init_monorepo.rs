use std::{cell::RefCell, collections::HashMap};

use callisto::{mega_blob, mega_refs, mega_tree};
use common::{
    config::MonoConfig,
    utils::{MEGA_BRANCH_NAME, generate_id},
};
use git_internal::{
    hash::ObjectHash,
    internal::{
        metadata::EntryMeta,
        object::{
            blob::Blob,
            commit::Commit,
            tree::{Tree, TreeItem, TreeItemMode},
        },
    },
};

use super::traits::IntoMegaModel;

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

    // Create unique .gitkeep for each root directory to ensure different tree hashes
    for dir in mono_config.root_dirs.clone() {
        let gitkeep_content = format!("Placeholder file for /{} directory", dir);
        let gitkeep_blob = Blob::from_content(&gitkeep_content);
        blobs.push(gitkeep_blob.clone());

        let tree_item = TreeItem {
            mode: TreeItemMode::Blob,
            id: gitkeep_blob.id,
            name: String::from(".gitkeep"),
        };
        let tree = Tree::from_tree_items(vec![tree_item]).unwrap();
        root_items.push(TreeItem {
            mode: TreeItemMode::Tree,
            id: tree.id,
            name: dir,
        });
        trees.push(tree);
    }

    // Create global .mega_cedar.json in root directory
    let entity_str = saturn::entitystore::generate_entity(&mono_config.admin, "/").unwrap();
    let cedar_blob = Blob::from_content(&entity_str);
    root_items.push(TreeItem {
        mode: TreeItemMode::Blob,
        id: cedar_blob.id,
        name: String::from(".mega_cedar.json"),
    });
    blobs.push(cedar_blob);

    inject_cedar_policy_dir(&mut root_items, &mut trees, &mut blobs, &mono_config.admin);

    inject_root_buck_files(&mut root_items, &mut blobs);

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
fn inject_root_buck_files(root_items: &mut Vec<TreeItem>, blobs: &mut Vec<Blob>) {
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
    let buckconfig_content = generate_buckconfig_content();
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

/// Generates Cedar policy content that sets admins as default reviewers.
///
/// Creates a policy rule that requires all admin users to review changes
/// across the entire repository (empty path pattern matches all paths).
fn generate_cedar_policy_content(admin_users: &[String]) -> String {
    if admin_users.is_empty() {
        return String::new();
    }

    // Format reviewer list: ["user1", "user2", ...]
    let reviewers_formatted = admin_users
        .iter()
        .map(|u| format!(r#""{}""#, u))
        .collect::<Vec<_>>()
        .join(", ");

    generate_cedar_policy_template().replace("{}", &reviewers_formatted)
}

/// Returns the default Cedar policy template for the repository root.
fn generate_cedar_policy_template() -> &'static str {
    r#"permit(action == "code:review", principal, resource)
    when { resource.path.startsWith("") }
    to [{}];
"#
}

fn inject_cedar_policy_dir(
    root_items: &mut Vec<TreeItem>,
    trees: &mut Vec<Tree>,
    blobs: &mut Vec<Blob>,
    admin_users: &[String],
) {
    let policy_content = generate_cedar_policy_content(admin_users);
    let policy_blob = Blob::from_content(&policy_content);
    blobs.push(policy_blob.clone());

    // Create .cedar directory with policies.cedar file
    let cedar_tree_item = TreeItem {
        mode: TreeItemMode::Blob,
        id: policy_blob.id,
        name: String::from("policies.cedar"),
    };
    let cedar_tree = Tree::from_tree_items(vec![cedar_tree_item]).unwrap();
    trees.push(cedar_tree.clone());

    // Add .cedar directory to root
    root_items.push(TreeItem {
        mode: TreeItemMode::Tree,
        id: cedar_tree.id,
        name: String::from(".cedar"),
    });
}

fn generate_buckconfig_content() -> String {
    let cells = [
        "  root = .",
        "  prelude = prelude",
        "  toolchains = toolchains",
        "  buckal = toolchains/buckal-bundles",
        "  none = none",
    ]
    .join("\n");

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
