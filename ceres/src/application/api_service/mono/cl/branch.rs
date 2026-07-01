//! Branch update / rebase operations for [`ClApplicationService`](super::service::ClApplicationService).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use callisto::{
    mega_cl,
    sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum},
};
use common::{errors::MegaError, utils::ZERO_ID};
use git_internal::{
    errors::GitError,
    hash::ObjectHash,
    internal::object::tree::{Tree, TreeItem, TreeItemMode},
};
use jupiter::utils::converter::FromMegaModel;
use tracing::debug;

use crate::{
    application::{
        api_service::mono::{
            ClApplicationService,
            types::{ApplyChangeContext, RefUpdate, TreeUpdateResult},
        },
        code_edit::utils as edit_utils,
    },
    model::change_list::{ClDiffFile, UpdateBranchStatusRes},
};

impl ClApplicationService {
    pub(crate) async fn apply_changes_as_single_commit(
        &self,
        cl: &mega_cl::Model,
        changes: &[ClDiffFile],
        target_head: &str,
    ) -> Result<String, GitError> {
        let mono_storage = self.storage().mono_storage();

        // Load base commit and its root tree
        let base_commit = mono_storage
            .get_commit_by_hash(target_head)
            .await?
            .ok_or_else(|| GitError::CustomError(format!("Commit not found: {target_head}")))?;

        let base_tree_model = mono_storage
            .get_tree_by_hash(&base_commit.tree)
            .await?
            .ok_or_else(|| GitError::CustomError("Root tree not found".to_string()))?;
        let mut root_tree = Tree::from_mega_model(base_tree_model);

        // Cache trees by path to reuse updated versions
        let mut tree_cache: HashMap<PathBuf, Tree> = HashMap::new();
        tree_cache.insert(PathBuf::from("/"), root_tree.clone());

        // Collect all new trees we generate (dedup by hash)
        let mut new_trees: HashMap<ObjectHash, Tree> = HashMap::new();

        for diff in changes {
            let operations: Vec<(PathBuf, Option<ObjectHash>)> = match diff {
                ClDiffFile::New(path, new_hash) => vec![(path.clone(), Some(*new_hash))],
                ClDiffFile::Modified(path, _old, new_hash) => {
                    vec![(path.clone(), Some(*new_hash))]
                }
                ClDiffFile::Deleted(path, _old) => vec![(path.clone(), None)],
                ClDiffFile::Renamed(old_path, new_path, _old_hash, new_hash, _similarity)
                | ClDiffFile::Moved(old_path, new_path, _old_hash, new_hash, _similarity) => {
                    vec![
                        (old_path.clone(), None),
                        (new_path.clone(), Some(*new_hash)),
                    ]
                }
            };

            for (file_path, op) in operations {
                // Reject absolute or parent-traversing paths to avoid writing outside repo root.
                if file_path.is_absolute()
                    || file_path
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    return Err(GitError::CustomError(format!(
                        "Invalid path (traversal/absolute) in CL diff: {:?}",
                        file_path
                    )));
                }

                let file_name = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| GitError::CustomError("Invalid file name".to_string()))?;
                // Normalize root parent to "/".
                let parent_path = match file_path.parent() {
                    Some(p) if !p.as_os_str().is_empty() => p,
                    _ => Path::new("/"),
                };

                // Build chain of trees from root to parent, using updated cache when available
                let components: Vec<String> = parent_path
                    .components()
                    .filter_map(|c| match c {
                        std::path::Component::RootDir => None,
                        other => other.as_os_str().to_str().map(|s| s.to_string()),
                    })
                    .collect();

                let mut chain_paths: Vec<PathBuf> = vec![PathBuf::from("/")];
                let mut chain_trees: Vec<Tree> = vec![
                    tree_cache
                        .get(&PathBuf::from("/"))
                        .cloned()
                        .ok_or_else(|| {
                            GitError::CustomError("Root tree cache missing".to_string())
                        })?,
                ];

                let mut cursor = PathBuf::from("/");
                let mut missing_components: Option<Vec<String>> = None;
                for (idx, comp) in components.iter().enumerate() {
                    let parent_tree = chain_trees
                        .last()
                        .ok_or_else(|| GitError::CustomError("Empty tree chain".to_string()))?;

                    let maybe_child = parent_tree.tree_items.iter().find(|it| it.name == *comp);
                    let child_tree = if let Some(child_item) = maybe_child {
                        if child_item.mode != TreeItemMode::Tree {
                            return Err(GitError::CustomError(format!(
                                "Type conflict: '{}' is not a directory",
                                comp
                            )));
                        }
                        cursor = cursor.join(comp);
                        let child_hash = child_item.id;
                        if let Some(cached) = tree_cache.get(&cursor) {
                            cached.clone()
                        } else {
                            let model = mono_storage
                                .get_tree_by_hash(&child_hash.to_string())
                                .await?
                                .ok_or_else(|| {
                                    GitError::CustomError(format!(
                                        "Tree not found for path '{}' with hash {}",
                                        cursor.to_string_lossy(),
                                        child_hash
                                    ))
                                })?;
                            Tree::from_mega_model(model)
                        }
                    } else {
                        missing_components = Some(components[idx..].to_vec());
                        break;
                    };

                    chain_paths.push(cursor.clone());
                    chain_trees.push(child_tree);
                }

                if let Some(missing) = missing_components {
                    let mut ctx = ApplyChangeContext {
                        components: &components,
                        chain_paths: &chain_paths,
                        chain_trees: &chain_trees,
                        tree_cache: &mut tree_cache,
                        new_trees: &mut new_trees,
                    };
                    if let Some(updated_root) =
                        Self::apply_missing_path_update(&cl.link, missing, op, file_name, &mut ctx)?
                    {
                        root_tree = updated_root;
                    }
                    continue;
                }

                let parent_dir_abs = cursor.clone();

                // Update parent tree with the file change
                let parent_tree = chain_trees
                    .pop()
                    .ok_or_else(|| GitError::CustomError("Parent tree missing".to_string()))?;
                chain_paths.pop();

                let mut items = parent_tree.tree_items.clone();
                match op {
                    Some(new_hash) => {
                        if let Some(idx) = items.iter().position(|it| it.name == file_name) {
                            items[idx].id = new_hash;
                        } else {
                            items.push(TreeItem::new(
                                TreeItemMode::Blob,
                                new_hash,
                                file_name.to_string(),
                            ));
                        }
                    }
                    None => {
                        items.retain(|it| it.name != file_name);
                    }
                }

                let updated_tree = Tree::from_tree_items(items)
                    .map_err(|e| GitError::CustomError(e.to_string()))?;
                // If parent tree id did not change (no-op), skip propagation for this diff.
                if updated_tree.id == parent_tree.id {
                    // keep cache consistent even for no-ops
                    tree_cache.insert(parent_dir_abs.clone(), parent_tree.clone());
                    debug!(
                        cl_link = %cl.link,
                        parent_dir = %parent_dir_abs.to_string_lossy(),
                        "apply_changes: no-op diff skipped"
                    );
                    continue;
                }
                Self::record_tree(
                    parent_dir_abs,
                    &updated_tree,
                    &mut tree_cache,
                    &mut new_trees,
                );

                // Propagate updated hashes up to root
                root_tree = Self::propagate_up(
                    &cl.link,
                    updated_tree,
                    &components,
                    &chain_paths,
                    &chain_trees,
                    &mut tree_cache,
                    &mut new_trees,
                )?;
            }
        }

        let result = TreeUpdateResult {
            updated_trees: new_trees.values().cloned().collect(),
            ref_updates: vec![RefUpdate {
                path: cl.path.clone(),
                tree_id: root_tree.id,
            }],
        };

        self.apply_update_result_cl_only(
            &result,
            "update-branch: rebase",
            &cl.link,
            Some(ObjectHash::from_str(target_head).map_err(|e| {
                GitError::CustomError(format!(
                    "Invalid target_head ObjectHash '{}': {}",
                    target_head, e
                ))
            })?),
        )
        .await
    }

    fn apply_missing_path_update(
        cl_link: &str,
        missing: Vec<String>,
        op: Option<ObjectHash>,
        file_name: &str,
        ctx: &mut ApplyChangeContext<'_>,
    ) -> Result<Option<Tree>, GitError> {
        debug_assert!(
            !missing.iter().any(|c| c == file_name),
            "missing path components should not include file name"
        );
        if op.is_none() {
            debug!(
                cl_link,
                missing_path = %missing.join("/"),
                "apply_changes: delete on missing path (no-op)"
            );
            return Ok(None);
        }

        let new_hash = op.ok_or_else(|| {
            GitError::CustomError("Missing blob hash for new/modified file".to_string())
        })?;

        if missing.is_empty() {
            // No missing directories: update directly under the last existing parent.
            let parent_path = ctx.chain_paths.last().cloned().unwrap_or_else(PathBuf::new);
            let parent_tree = ctx
                .chain_trees
                .last()
                .cloned()
                .ok_or_else(|| GitError::CustomError("Root tree missing".to_string()))?;
            let updated_tree = Self::update_parent_tree(
                cl_link,
                &parent_tree,
                file_name,
                TreeItemMode::Blob,
                new_hash,
                None,
            )?;
            Self::record_tree(parent_path, &updated_tree, ctx.tree_cache, ctx.new_trees);

            return Ok(Some(Self::propagate_up(
                cl_link,
                updated_tree,
                ctx.components,
                ctx.chain_paths,
                ctx.chain_trees,
                ctx.tree_cache,
                ctx.new_trees,
            )?));
        }

        // Build missing subtree from leaf (parent dir) upward without empty trees.
        let file_item = TreeItem::new(TreeItemMode::Blob, new_hash, file_name.to_string());
        let mut updated_tree = Tree::from_tree_items(vec![file_item])
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        let mut missing_paths: Vec<PathBuf> = Vec::new();
        let mut base = ctx.chain_paths.last().cloned().unwrap_or_else(PathBuf::new);
        for comp in &missing {
            base = base.join(comp);
            missing_paths.push(base.clone());
        }

        if let Some(parent_path) = missing_paths.last() {
            Self::record_tree(
                parent_path.clone(),
                &updated_tree,
                ctx.tree_cache,
                ctx.new_trees,
            );
        } else {
            Self::record_tree(PathBuf::new(), &updated_tree, ctx.tree_cache, ctx.new_trees);
        }

        for (child_name, path) in missing
            .iter()
            .rev()
            .skip(1)
            .zip(missing_paths.iter().rev().skip(1))
        {
            let wrapper = Tree::from_tree_items(vec![TreeItem::new(
                TreeItemMode::Tree,
                updated_tree.id,
                child_name.clone(),
            )])
            .map_err(|e| GitError::CustomError(e.to_string()))?;
            updated_tree = wrapper;
            Self::record_tree(path.clone(), &updated_tree, ctx.tree_cache, ctx.new_trees);
        }

        // Attach the newly built subtree to the last existing parent.
        let parent_tree = ctx
            .chain_trees
            .last()
            .cloned()
            .ok_or_else(|| GitError::CustomError("Root tree missing".to_string()))?;
        let attach_name = missing
            .first()
            .ok_or_else(|| GitError::CustomError("Missing component chain empty".to_string()))?;
        updated_tree = Self::update_parent_tree(
            cl_link,
            &parent_tree,
            attach_name,
            TreeItemMode::Tree,
            updated_tree.id,
            None,
        )?;
        let parent_path = ctx.chain_paths.last().cloned().unwrap_or_else(PathBuf::new);
        Self::record_tree(parent_path, &updated_tree, ctx.tree_cache, ctx.new_trees);

        Ok(Some(Self::propagate_up(
            cl_link,
            updated_tree,
            ctx.components,
            ctx.chain_paths,
            ctx.chain_trees,
            ctx.tree_cache,
            ctx.new_trees,
        )?))
    }

    fn update_parent_tree(
        cl_link: &str,
        parent_tree: &Tree,
        name: &str,
        mode: TreeItemMode,
        id: ObjectHash,
        debug_parent_path: Option<&PathBuf>,
    ) -> Result<Tree, GitError> {
        let mut parent_items = parent_tree.tree_items.clone();
        if let Some(pos) = parent_items.iter().position(|it| it.name == name) {
            parent_items[pos].id = id;
        } else {
            parent_items.push(TreeItem::new(mode, id, name.to_string()));
            parent_items.sort_by(|a, b| a.name.cmp(&b.name));
            if let Some(path) = debug_parent_path {
                debug!(
                    cl_link,
                    parent_path = %path.to_string_lossy(),
                    created_entry = %name,
                    "apply_changes: inserted missing parent entry"
                );
            }
        }

        Tree::from_tree_items(parent_items).map_err(|e| GitError::CustomError(e.to_string()))
    }

    fn record_tree(
        path: PathBuf,
        tree: &Tree,
        tree_cache: &mut HashMap<PathBuf, Tree>,
        new_trees: &mut HashMap<ObjectHash, Tree>,
    ) {
        tree_cache.insert(path, tree.clone());
        new_trees.insert(tree.id, tree.clone());
    }

    fn propagate_up(
        cl_link: &str,
        mut updated_tree: Tree,
        components: &[String],
        chain_paths: &[PathBuf],
        chain_trees: &[Tree],
        tree_cache: &mut HashMap<PathBuf, Tree>,
        new_trees: &mut HashMap<ObjectHash, Tree>,
    ) -> Result<Tree, GitError> {
        debug_assert!(
            components.len() >= chain_trees.len().saturating_sub(1),
            "components length must cover parent chain"
        );

        for parent_index in (0..chain_trees.len().saturating_sub(1)).rev() {
            let comp = components
                .get(parent_index)
                .ok_or_else(|| GitError::CustomError("Tree path chain underflow".to_string()))?;

            let parent_tree = Self::update_parent_tree(
                cl_link,
                &chain_trees[parent_index],
                comp,
                TreeItemMode::Tree,
                updated_tree.id,
                chain_paths.get(parent_index),
            )?;

            let parent_path_idx = chain_paths
                .get(parent_index)
                .cloned()
                .ok_or_else(|| GitError::CustomError("Tree path chain underflow".to_string()))?;
            Self::record_tree(parent_path_idx, &parent_tree, tree_cache, new_trees);
            updated_tree = parent_tree;
        }

        Ok(updated_tree)
    }
    /// Return Update Branch status for a CL: only checks whether main/trunk moved past the CL base.
    pub async fn update_branch_status(
        &self,
        cl_link: &str,
    ) -> Result<UpdateBranchStatusRes, MegaError> {
        let stg = self.storage().cl_service.cl_store();
        let cl = stg
            .get_cl(cl_link)
            .await?
            .ok_or_else(|| MegaError::Other("CL Not Found".to_string()))?;

        let main_ref = match self.storage().mono_storage().get_main_ref(&cl.path).await? {
            Some(r) => r,
            None if crate::application::api_service::mono::cl_merge::path_lacks_main_ref(
                self.git_ops(),
                &cl.path,
            )
            .await? =>
            {
                return Ok(UpdateBranchStatusRes {
                    base_commit: cl.from_hash.clone(),
                    target_head: ZERO_ID.to_string(),
                    outdated: false,
                });
            }
            None => return Err(MegaError::Other("Main ref not found".to_string())),
        };
        let target_head = main_ref.ref_commit_hash;

        Ok(UpdateBranchStatusRes {
            base_commit: cl.from_hash.clone(),
            target_head: target_head.clone(),
            outdated: cl.from_hash != target_head,
        })
    }

    /// Update Branch (rebase-like) for Open CL: applies CL file changes onto latest target head
    /// and updates CL's base/head commits. Returns new head commit id on success.
    pub async fn update_branch(&self, username: &str, cl_link: &str) -> Result<String, GitError> {
        let stg = self.storage().cl_service.cl_store();
        let conv_stg = self.storage().cl_service.conversation_store();

        let cl = stg
            .get_cl(cl_link)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?
            .ok_or_else(|| GitError::CustomError("CL Not Found".to_string()))?;

        if cl.status != MergeStatusEnum::Open {
            return Err(GitError::CustomError(
                "Only Open CL can update branch".to_string(),
            ));
        }

        let main_ref = self
            .storage()
            .mono_storage()
            .get_main_ref(&cl.path)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?
            .ok_or_else(|| GitError::CustomError("Main ref not found".to_string()))?;
        let target_head = main_ref.ref_commit_hash;

        if target_head == cl.from_hash {
            return Ok("Already up-to-date".to_string());
        }

        // Detect file-level conflicts
        let conflicts = self.detect_update_conflicts(&cl, &target_head).await?;

        if !conflicts.is_empty() {
            // Record conflict info on the CL conversation for visibility.
            let conflict_msg = format!(
                "{} failed to update branch: conflicts on {}",
                username,
                conflicts.join(", ")
            );
            if let Err(e) = conv_stg
                .add_conversation(cl_link, username, Some(conflict_msg), ConvTypeEnum::Comment)
                .await
            {
                tracing::warn!("Failed to add conflict comment to conversation: {}", e);
            }
            return Err(GitError::CustomError(format!(
                "Update conflict on files: {}",
                conflicts.join(", ")
            )));
        }

        // Apply CL diffs onto latest target head
        let old_blobs = self
            .get_commit_blobs(&cl.from_hash)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let new_blobs = self
            .get_commit_blobs(&cl.to_hash)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let cl_changed = self
            .cl_files_list(old_blobs.clone(), new_blobs.clone())
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        if cl_changed.is_empty() {
            // No-op rebase: just advance base hash and log.
            stg.update_cl_hash(cl.clone(), &target_head, &cl.to_hash)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;
            conv_stg
                .add_conversation(
                    cl_link,
                    username,
                    Some(format!(
                        "{} updated branch (no changes) to {}",
                        username,
                        &target_head[..6]
                    )),
                    ConvTypeEnum::Comment,
                )
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;
            return Ok(cl.to_hash);
        }

        // Apply all changes in-memory atop target_head and emit a single commit for the CL ref.
        let new_head = self
            .apply_changes_as_single_commit(&cl, &cl_changed, &target_head)
            .await?;

        // Update cl hashes and log
        stg.update_cl_hash(cl.clone(), &target_head, &new_head)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        conv_stg
            .add_conversation(
                cl_link,
                username,
                Some(format!(
                    "{} updated branch to {}",
                    username,
                    &target_head[..6]
                )),
                ConvTypeEnum::Comment,
            )
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        Ok(new_head)
    }

    /// Detect file-level update conflicts between the CL changes and target head.
    /// A conflict is reported if any file path modified by the CL is also changed
    /// between `from_hash` and `target_head`.
    pub(crate) async fn detect_update_conflicts(
        &self,
        cl: &mega_cl::Model,
        target_head: &str,
    ) -> Result<Vec<String>, GitError> {
        let old_blobs = self
            .get_commit_blobs(&cl.from_hash)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let new_blobs = self
            .get_commit_blobs(&cl.to_hash)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        // Keep conflict checks path-based so renames cover both old and new paths.
        let cl_changed = edit_utils::cl_files_list(old_blobs.clone(), new_blobs.clone())
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        let target_blobs = self
            .get_commit_blobs(target_head)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let base_vs_target = edit_utils::cl_files_list(old_blobs.clone(), target_blobs.clone())
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        let cl_paths: std::collections::HashSet<String> = cl_changed
            .iter()
            .map(|f| f.path().to_string_lossy().replace('\\', "/"))
            .collect();
        let target_paths: std::collections::HashSet<String> = base_vs_target
            .iter()
            .map(|f| f.path().to_string_lossy().replace('\\', "/"))
            .collect();

        Ok(cl_paths.intersection(&target_paths).cloned().collect())
    }
}
