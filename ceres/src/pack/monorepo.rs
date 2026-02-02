use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    vec,
};

use api_model::common::Pagination;
use async_recursion::async_recursion;
use async_trait::async_trait;
use bellatrix::{
    Bellatrix,
    orion_client::{BuildInfo, OrionBuildRequest, ProjectRelativePath, Status},
};
use callisto::{
    entity_ext::generate_link,
    mega_cl, mega_code_review_anchor, mega_commit, mega_refs,
    sea_orm_active_enums::{ConvTypeEnum, PositionStatusEnum},
};
use common::{
    errors::MegaError,
    utils::{self, ZERO_ID},
};
use futures::{StreamExt, stream};
use git_internal::{
    errors::GitError,
    hash::ObjectHash,
    internal::{
        metadata::{EntryMeta, MetaAttached},
        object::{
            ObjectTrait, commit::Commit, signature::Signature, tree::Tree, types::ObjectType,
        },
        pack::{encode::PackEncoder, entry::Entry},
    },
};
use io_orbit::object_storage::MultiObjectByteStream;
use jupiter::{
    service::reviewer_service::ReviewerService, storage::Storage, utils::converter::FromMegaModel,
};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    api_service::{ApiHandler, cache::GitObjectCache, mono_api_service::MonoApiService, tree_ops},
    merge_checker::CheckerRegistry,
    model::change_list::BuckFile,
    pack::RepoHandler,
    protocol::import_refs::{RefCommand, Refs},
};

pub struct MonoRepo {
    pub storage: Storage,
    pub git_object_cache: Arc<GitObjectCache>,
    pub path: PathBuf,
    pub from_hash: String,
    pub to_hash: String,
    // current_commit only exists when an unpack operation occurs.
    // When only a branch is updated and the pack file is empty, this value will be None.
    pub current_commit: Arc<RwLock<Option<Commit>>>,
    pub cl_link: Arc<RwLock<Option<String>>>,
    pub bellatrix: Arc<Bellatrix>,
    pub username: Option<String>,
}

#[async_trait]
impl RepoHandler for MonoRepo {
    fn is_monorepo(&self) -> bool {
        true
    }

    async fn refs_with_head_hash(&self) -> (String, Vec<Refs>) {
        let storage = self.storage.mono_storage();

        let path_refs = storage
            .get_all_refs(self.path.to_str().unwrap(), false)
            .await
            .unwrap();

        let heads_exist = path_refs
            .iter()
            .any(|x| x.ref_name == common::utils::MEGA_BRANCH_NAME);

        let refs = if heads_exist {
            let refs: Vec<Refs> = path_refs.into_iter().map(|x| x.into()).collect();
            refs
        } else {
            let target_path = self.path.clone();
            let mut refs = vec![];

            let root_refs = storage.get_all_refs("/", true).await.unwrap();

            for root_ref in root_refs {
                let (tree_hash, commit_hash) = (root_ref.ref_tree_hash, root_ref.ref_commit_hash);
                let mut tree: Tree = Tree::from_mega_model(
                    storage.get_tree_by_hash(&tree_hash).await.unwrap().unwrap(),
                );

                let commit: Commit = Commit::from_mega_model(
                    storage
                        .get_commit_by_hash(&commit_hash)
                        .await
                        .unwrap()
                        .unwrap(),
                );

                for component in target_path.components() {
                    if component != Component::RootDir {
                        let path_compo_name = component.as_os_str().to_str().unwrap();
                        let path_compo_hash = tree
                            .tree_items
                            .iter()
                            .find(|x| x.name == path_compo_name)
                            .map(|x| x.id);
                        if let Some(hash) = path_compo_hash {
                            tree = Tree::from_mega_model(
                                storage
                                    .get_tree_by_hash(&hash.to_string())
                                    .await
                                    .unwrap()
                                    .unwrap(),
                            );
                        } else {
                            return (ZERO_ID.to_string(), vec![]);
                        }
                    }
                }
                let c = Commit::new(
                    commit.author,
                    commit.committer,
                    tree.id,
                    vec![],
                    &commit.message,
                );

                let new_mega_ref = mega_refs::Model::new(
                    &self.path,
                    root_ref.ref_name.clone(),
                    c.id.to_string(),
                    c.tree_id.to_string(),
                    false,
                );

                storage
                    .mega_head_hash_with_txn(new_mega_ref.clone(), c)
                    .await
                    .unwrap();

                refs.push(new_mega_ref.into());
            }
            refs
        };
        self.find_head_hash(refs)
    }

    async fn post_receive_pack(&self) -> Result<(), MegaError> {
        self.save_or_update_cl().await?;
        self.traverses_tree_and_update_filepath().await?;
        self.post_cl_operation().await?;
        self.reanchor_code_review_threads().await?;
        Ok(())
    }

    async fn save_entry(
        &self,
        entry_list: Vec<MetaAttached<Entry, EntryMeta>>,
    ) -> Result<(), MegaError> {
        let current_commit = self.current_commit.read().await;
        let commit_id = if let Some(commit) = &*current_commit {
            commit.id.to_string()
        } else {
            String::new()
        };
        let commit_models = self
            .storage
            .mono_service
            .save_entry(&commit_id, entry_list)
            .await?;

        if !commit_models.is_empty() {
            let commits_to_process: Result<Vec<(String, String)>, MegaError> = commit_models
                .into_iter()
                .map(|c| {
                    let model: mega_commit::Model = c.try_into()?;
                    let author_bytes = model.author.as_deref().unwrap_or("").as_bytes();
                    let signature = Signature::from_data(author_bytes.to_vec())?;
                    Ok((model.commit_id, signature.email))
                })
                .collect();

            self.storage
                .mono_storage()
                .process_commit_bindings(&commits_to_process?, self.username.clone().as_deref())
                .await?;
        }
        Ok(())
    }

    async fn update_pack_id(&self, temp_pack_id: &str, pack_id: &str) -> Result<(), MegaError> {
        let storage = self.storage.mono_storage();
        storage.update_pack_id(temp_pack_id, pack_id).await
    }

    async fn check_entry(&self, entry: &Entry) -> Result<(), GitError> {
        if self.current_commit.read().await.is_none() {
            if entry.obj_type == ObjectType::Commit {
                let commit = Commit::from_bytes(&entry.data, entry.hash).unwrap();
                let mut current = self.current_commit.write().await;
                *current = Some(commit);
            }
        } else if entry.obj_type == ObjectType::Commit {
            return Err(GitError::CustomError(
                "only single commit support in each push".to_string(),
            ));
        }
        Ok(())
    }

    // monorepo full pack should follow the shallow clone command 'git clone --depth=1'
    async fn full_pack(&self, want: Vec<String>) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        self.incremental_pack(want, Vec::new()).await
    }

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<ReceiverStream<Vec<u8>>, GitError> {
        let mut want_clone = want.clone();
        let pack_config = &self.storage.config().pack;
        let storage = self.storage.mono_storage();
        let obj_num = AtomicUsize::new(0);

        let mut exist_objs = HashSet::new();

        let mut want_commits: Vec<Commit> = storage
            .get_commits_by_hashes(&want_clone)
            .await
            .unwrap()
            .into_iter()
            .map(Commit::from_mega_model)
            .collect();
        let mut traversal_list: Vec<Commit> = want_commits.clone();

        // traverse commit's all parents to find the commit that client does not have
        while let Some(temp) = traversal_list.pop() {
            for p_commit_id in temp.parent_commit_ids {
                let p_commit_id = p_commit_id.to_string();

                if !have.contains(&p_commit_id) && !want_clone.contains(&p_commit_id) {
                    let parent: Commit = Commit::from_mega_model(
                        storage
                            .get_commit_by_hash(&p_commit_id)
                            .await
                            .unwrap()
                            .unwrap(),
                    );
                    want_commits.push(parent.clone());
                    want_clone.push(p_commit_id);
                    traversal_list.push(parent);
                }
            }
        }

        let want_tree_ids = want_commits.iter().map(|c| c.tree_id.to_string()).collect();
        let want_trees: HashMap<ObjectHash, Tree> = storage
            .get_trees_by_hashes(want_tree_ids)
            .await
            .unwrap()
            .into_iter()
            .map(|m| {
                (
                    ObjectHash::from_str(&m.tree_id).unwrap(),
                    Tree::from_mega_model(m),
                )
            })
            .collect();

        obj_num.fetch_add(want_commits.len(), Ordering::SeqCst);

        let have_commits = storage.get_commits_by_hashes(&have).await.unwrap();
        let have_trees = storage
            .get_trees_by_hashes(have_commits.iter().map(|x| x.tree.clone()).collect())
            .await
            .unwrap();
        for have_tree in have_trees {
            self.traverse(Tree::from_mega_model(have_tree), &mut exist_objs, None)
                .await?;
        }

        let mut counted_obj = HashSet::new();
        // traverse for get obj nums
        for c in want_commits.clone() {
            self.traverse_for_count(
                want_trees.get(&c.tree_id).unwrap().clone(),
                &exist_objs,
                &mut counted_obj,
                &obj_num,
            )
            .await;
        }
        let (entry_tx, entry_rx) = mpsc::channel(pack_config.channel_message_size);
        let (stream_tx, stream_rx) = mpsc::channel(pack_config.channel_message_size);
        let encoder = PackEncoder::new(obj_num.into_inner(), 0, stream_tx);
        encoder.encode_async(entry_rx).await.unwrap();
        // todo: For now, send metadata only for blob objects.
        for c in want_commits {
            self.traverse(
                want_trees.get(&c.tree_id).unwrap().clone(),
                &mut exist_objs,
                Some(&entry_tx),
            )
            .await?;
            entry_tx
                .send(MetaAttached {
                    inner: c.into(),
                    meta: EntryMeta::new(),
                })
                .await
                .unwrap();
        }
        drop(entry_tx);

        Ok(ReceiverStream::new(stream_rx))
    }

    async fn get_trees_by_hashes(&self, hashes: Vec<String>) -> Result<Vec<Tree>, MegaError> {
        Ok(self
            .storage
            .mono_storage()
            .get_trees_by_hashes(hashes)
            .await
            .unwrap()
            .into_iter()
            .map(Tree::from_mega_model)
            .collect())
    }

    async fn get_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<MultiObjectByteStream<'_>, MegaError> {
        Ok(self.storage.git_service.get_objects_stream(hashes))
    }

    async fn get_blob_metadata_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<HashMap<String, EntryMeta>, MegaError> {
        let models = self
            .storage
            .mono_storage()
            .get_mega_blobs_by_hashes(hashes)
            .await?;

        let map = models
            .into_iter()
            .map(|blob| {
                (
                    blob.blob_id.clone(),
                    EntryMeta {
                        pack_id: Some(blob.pack_id.clone()),
                        pack_offset: Some(blob.pack_offset as usize),
                        file_path: Some(blob.file_path.clone()),
                        is_delta: Some(blob.is_delta_in_pack),
                        // TODO: Populate `crc32` once mono blob metadata exposes it.
                        // For now we set it to `None` because `get_mega_blobs_by_hashes` does
                        // not provide CRC32 information for monorepo-backed packs.
                        crc32: None,
                    },
                )
            })
            .collect::<HashMap<String, EntryMeta>>();

        Ok(map)
    }

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError> {
        let storage = self.storage.mono_storage();
        let current_commit = self.current_commit.read().await;
        let cl_link = self.fetch_or_new_cl_link().await?;
        let ref_name = utils::cl_ref_name(&cl_link);

        if let Some(c) = &*current_commit {
            if let Some(mut cl_ref) = storage.get_ref_by_name(&ref_name).await? {
                cl_ref.ref_commit_hash = refs.new_id.clone();
                cl_ref.ref_tree_hash = c.tree_id.to_string();
                storage.update_ref(cl_ref, None).await?;
            } else {
                let refs = mega_refs::Model::new(
                    &self.path,
                    ref_name,
                    refs.new_id.clone(),
                    c.tree_id.to_string(),
                    true,
                );
                storage.save_refs(refs, None).await?;
            }
        }
        Ok(())
    }

    async fn check_commit_exist(&self, hash: &str) -> bool {
        self.storage
            .mono_storage()
            .get_commit_by_hash(hash)
            .await
            .unwrap()
            .is_some()
    }

    async fn check_default_branch(&self) -> bool {
        true
    }

    async fn traverses_tree_and_update_filepath(&self) -> Result<(), MegaError> {
        let commit_guard = self.current_commit.read().await;
        let commit_opt = match commit_guard.as_ref() {
            Some(commit) => commit,
            None => {
                tracing::info!(
                    "Skipping file path update: no current commit available. \
                     This typically occurs when only updating references or pushing empty pack files."
                );
                return Ok(());
            }
        };

        let tree_hashes = vec![commit_opt.tree_id.to_string()];
        let trees = self
            .storage
            .mono_storage()
            .get_trees_by_hashes(tree_hashes)
            .await
            .map_err(|e| {
                MegaError::Other(format!(
                    "Failed to retrieve root tree for commit {}: {}",
                    commit_opt.id, e
                ))
            })?;

        if trees.is_empty() {
            return Err(MegaError::Other(format!(
                "Root tree {} not found for commit {}",
                commit_opt.tree_id, commit_opt.id
            )));
        }

        let root_tree = Tree::from_mega_model(trees[0].clone());

        tracing::info!(
            "Starting file path update for commit {} with root tree {}",
            commit_opt.id,
            commit_opt.tree_id
        );

        self.traverses_and_update_filepath(root_tree, PathBuf::new())
            .await
            .map_err(|e| {
                MegaError::Other(format!(
                    "Failed to update file paths for commit {}: {}",
                    commit_opt.id, e
                ))
            })?;

        tracing::info!(
            "Successfully completed file path update for commit {}",
            commit_opt.id
        );

        Ok(())
    }
}

impl MonoRepo {
    #[async_recursion]
    async fn traverses_and_update_filepath(
        &self,
        tree: Tree,
        path: PathBuf,
    ) -> Result<(), MegaError> {
        for item in tree.tree_items {
            let item_path = path.join(&item.name);

            if item.is_tree() {
                let tree_hash = item.id.to_string();
                let trees = self
                    .storage
                    .mono_storage()
                    .get_trees_by_hashes(vec![tree_hash.clone()])
                    .await
                    .map_err(|e| {
                        MegaError::Other(format!(
                            "Failed to retrieve tree {} at path '{}': {}",
                            tree_hash,
                            item_path.display(),
                            e
                        ))
                    })?;

                if trees.is_empty() {
                    return Err(MegaError::Other(format!(
                        "Tree {} not found at path '{}'",
                        tree_hash,
                        item_path.display()
                    )));
                }

                let child_tree = Tree::from_mega_model(trees[0].clone());

                self.traverses_and_update_filepath(child_tree, item_path.clone())
                    .await
                    .map_err(|e| {
                        MegaError::Other(format!(
                            "Failed to process subtree {} at path '{}': {}",
                            tree_hash,
                            item_path.display(),
                            e
                        ))
                    })?;
            } else {
                let blob_id = item.id.to_string();
                let file_path_str = item_path.to_str().ok_or_else(|| {
                    MegaError::Other(format!(
                        "Invalid UTF-8 path for blob {}: '{}'",
                        blob_id,
                        item_path.display()
                    ))
                })?;

                self.storage
                    .mono_storage()
                    .update_blob_filepath(&blob_id, file_path_str)
                    .await
                    .map_err(|e| {
                        MegaError::Other(format!(
                            "Failed to update file path for blob {} at '{}': {}",
                            blob_id, file_path_str, e
                        ))
                    })?;

                tracing::debug!(
                    "Updated file path for blob {} to '{}'",
                    blob_id,
                    file_path_str
                );
            }
        }

        Ok(())
    }
    async fn fetch_or_new_cl_link(&self) -> Result<String, MegaError> {
        let storage = self.storage.cl_storage();
        let path_str = self.path.to_str().unwrap();
        let cl_link = match storage
            .get_open_cl_by_path(path_str, &self.username())
            .await?
        {
            Some(cl) => cl.link.clone(),
            None => {
                if self.from_hash == "0".repeat(40) {
                    return Err(MegaError::Other(
                        "Can not init directory under monorepo directory!".to_string(),
                    ));
                }
                generate_link()
            }
        };
        let mut lock = self.cl_link.write().await;
        *lock = Some(cl_link.clone());
        Ok(cl_link)
    }

    async fn update_existing_cl(&self, cl: mega_cl::Model) -> Result<(), MegaError> {
        let cl_stg = self.storage.cl_storage();
        let comment_stg = self.storage.conversation_storage();

        let from_same = cl.from_hash == self.from_hash;
        let to_same = cl.to_hash == self.to_hash;

        if from_same && to_same {
            tracing::info!("repeat commit with cl: {}, do nothing", cl.id);
            return Ok(());
        }

        if from_same {
            let username = self.username();
            let old_hash = &cl.to_hash[..6];
            let new_hash = &self.to_hash[..6];

            comment_stg
                .add_conversation(
                    &cl.link,
                    &username,
                    Some(format!(
                        "{} updated the cl automatic from {} to {}",
                        username, old_hash, new_hash
                    )),
                    ConvTypeEnum::ForcePush,
                )
                .await?;

            cl_stg.update_cl_to_hash(cl, &self.to_hash).await?;
        } else {
            // Freeze CL base for Open CL: do NOT auto-update from_hash here.
            // Only update to_hash to reflect latest edits, and prompt user to run Update Branch.
            let username = self.username();
            let old_base = &cl.from_hash[..6];
            let new_target = &self.from_hash[..6];
            comment_stg
                .add_conversation(
                    &cl.link,
                    &username,
                    Some(format!(
                        "{} detected upstream changes (base {} → {}). Use Update Branch to sync.",
                        username, old_base, new_target
                    )),
                    ConvTypeEnum::Comment,
                )
                .await?;
            cl_stg.update_cl_to_hash(cl, &self.to_hash).await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn search_buck_under_cl(&self, cl_path: &Path) -> Result<Vec<BuckFile>, MegaError> {
        let mut res = vec![];
        let mono_stg = self.storage.mono_storage();
        let mono_api_service: MonoApiService = self.into();

        let mut path = Some(cl_path);
        let mut path_q = Vec::new();
        while let Some(p) = path {
            path_q.push(p);
            path = p.parent();
        }
        if path_q.len() > 2 {
            path_q.pop();
            path_q.pop();

            let p = path_q[path_q.len() - 1];
            if p.parent().is_some()
                && let Some(tree) = tree_ops::search_tree_by_path(&mono_api_service, p, None)
                    .await
                    .ok()
                    .flatten()
                && let Some(buck) = self.try_extract_buck(tree, cl_path)
            {
                return Ok(vec![buck]);
            };
            return Ok(vec![]);
        }

        let mut search_trees: Vec<(PathBuf, Tree)> = vec![];

        let diff_trees = self.diff_trees_from_cl().await?;
        for (path, new, old) in diff_trees {
            match (new, old) {
                (None, _) => {
                    continue;
                }
                (Some(sha1), _) => {
                    let tree = mono_stg.get_tree_by_hash(&sha1.to_string()).await?.unwrap();
                    search_trees.push((path, Tree::from_mega_model(tree)));
                }
            }
        }

        for (path, tree) in search_trees {
            if let Some(buck) = self.try_extract_buck(tree, &cl_path.join(path)) {
                res.push(buck);
            }
        }

        Ok(res)
    }

    fn try_extract_buck(&self, tree: Tree, cl_path: &Path) -> Option<BuckFile> {
        let mut buck = None;
        let mut buck_config = None;
        for item in tree.tree_items {
            if item.is_blob() && item.name == "BUCK" {
                buck = Some(item.id)
            }
            if item.is_blob() && item.name == ".buckconfig" {
                buck_config = Some(item.id)
            }
        }
        match (buck, buck_config) {
            (Some(buck), Some(buck_config)) => Some(BuckFile {
                buck,
                buck_config,
                path: cl_path.to_path_buf(),
            }),
            _ => None,
        }
    }

    async fn diff_trees_from_cl(
        &self,
    ) -> Result<Vec<(PathBuf, Option<ObjectHash>, Option<ObjectHash>)>, MegaError> {
        let mono_stg = self.storage.mono_storage();
        let from_c = mono_stg.get_commit_by_hash(&self.from_hash).await?.unwrap();
        let from_tree: Tree =
            Tree::from_mega_model(mono_stg.get_tree_by_hash(&from_c.tree).await?.unwrap());
        let to_c = mono_stg.get_commit_by_hash(&self.to_hash).await?.unwrap();
        let to_tree: Tree =
            Tree::from_mega_model(mono_stg.get_tree_by_hash(&to_c.tree).await?.unwrap());
        diff_trees(&to_tree, &from_tree)
    }

    pub fn username(&self) -> String {
        self.username.clone().unwrap_or(String::from("Anonymous"))
    }

    pub async fn save_or_update_cl(&self) -> Result<(), MegaError> {
        let storage = self.storage.cl_storage();
        let path_str = self.path.to_string_lossy();
        let username = self.username();

        let is_new_cl = match storage.get_open_cl_by_path(&path_str, &username).await? {
            Some(cl) => {
                self.update_existing_cl(cl).await?;
                false
            }
            None => {
                let link_guard = self.cl_link.read().await;
                let cl_link = link_guard.as_ref().ok_or_else(|| {
                    MegaError::Other(
                        "CL link not available. This may occur if refs update failed.".to_string(),
                    )
                })?;

                let commit_guard = self.current_commit.read().await;
                let title = if let Some(commit) = commit_guard.as_ref() {
                    commit.format_message()
                } else {
                    String::new()
                };
                storage
                    .new_cl(
                        &path_str,
                        cl_link,
                        &title,
                        &self.from_hash,
                        &self.to_hash,
                        &username,
                    )
                    .await?;
                true
            }
        };

        // Auto-assign reviewers for new CL
        if is_new_cl && let Err(e) = self.assign_system_reviewers().await {
            tracing::warn!("Failed to assign Cedar reviewers: {}", e);
        }

        // Resync reviewers when existing CL updates policy files
        if !is_new_cl && let Err(e) = self.resync_current_cl_reviewers_if_policy_changed().await {
            tracing::warn!("Failed to resync Cedar reviewers: {}", e);
        }

        Ok(())
    }

    /// Resync reviewers when policy files are modified in an existing CL.
    async fn resync_current_cl_reviewers_if_policy_changed(&self) -> Result<(), MegaError> {
        let changed_files = self.get_changed_files().await?;

        let link_guard = self.cl_link.read().await;
        let cl_link = link_guard
            .as_ref()
            .ok_or_else(|| MegaError::Other("CL link not available".to_string()))?;

        let policy_contents = self.collect_policy_contents(&changed_files).await;
        if policy_contents.is_empty() {
            return Ok(());
        }

        let reviewer_service = ReviewerService::from_storage(self.storage.reviewer_storage());
        reviewer_service
            .sync_system_reviewers(cl_link, &policy_contents, &changed_files)
            .await?;

        Ok(())
    }

    /// Get list of files changed between from_hash and to_hash commits.
    /// Returns paths relative to the CL root directory with forward slashes.
    async fn get_changed_files(&self) -> Result<Vec<String>, MegaError> {
        let mono_api_service: MonoApiService = self.into();

        let old_files = mono_api_service.get_commit_blobs(&self.from_hash).await?;
        let new_files = mono_api_service.get_commit_blobs(&self.to_hash).await?;
        let changed = mono_api_service.cl_files_list(old_files, new_files).await?;

        // Normalize CL root path to use forward slashes
        let cl_root = self.path.to_string_lossy().replace('\\', "/");
        let cl_root_normalized = cl_root.trim_start_matches('/');

        let file_paths: Vec<String> = changed
            .iter()
            .map(|f| {
                let full_path = f.path().to_string_lossy().replace('\\', "/");
                let full_path_normalized = full_path.trim_start_matches('/');

                // Strip CL root prefix to get relative path
                if let Some(rel) = full_path_normalized.strip_prefix(cl_root_normalized) {
                    rel.trim_start_matches('/').to_string()
                } else {
                    full_path.to_string()
                }
            })
            .collect();

        Ok(file_paths)
    }

    /// Collect Cedar policy files from directories of all changed files.
    /// Also collects policies from parent directories up to Monorepo root for inheritance.
    /// Tries from_hash first for security, then falls back to to_hash for new directories.
    /// Returns list of (policy_path, content) tuples, ordered from root to leaf.
    async fn collect_policy_contents(&self, changed_files: &[String]) -> Vec<(PathBuf, String)> {
        let mono_api_service: MonoApiService = self.into();
        let mut all_policy_dirs: HashSet<PathBuf> = HashSet::new();

        // Always include the CL root directory
        all_policy_dirs.insert(PathBuf::new());

        // Collect ancestor directories from all changed files
        for file_path in changed_files {
            let relative_path = file_path.trim_start_matches('/').replace('\\', "/");
            let path = PathBuf::from(&relative_path);

            let parent = path.parent().unwrap_or(std::path::Path::new(""));

            // Skip .cedar directory itself, use its parent
            let logical_parent = if parent.file_name().map(|n| n == ".cedar").unwrap_or(false) {
                parent.parent().unwrap_or(std::path::Path::new(""))
            } else {
                parent
            };

            for ancestor in logical_parent.ancestors() {
                let ancestor_str = ancestor.to_string_lossy();
                if ancestor_str.contains(".cedar") {
                    continue;
                }
                let normalized = PathBuf::from(ancestor_str.replace('\\', "/"));
                all_policy_dirs.insert(normalized);
            }
        }

        // Sort by depth for correct override semantics (root policies first)
        let mut sorted_dirs: Vec<PathBuf> = all_policy_dirs.into_iter().collect();
        sorted_dirs.sort_by_key(|p| p.components().count());

        let mut policy_contents: Vec<(PathBuf, String)> = Vec::new();
        let mut seen_policies: HashSet<String> = HashSet::new();

        let self_path_str = self.path.to_string_lossy().replace('\\', "/");
        let self_path_normalized = self_path_str.trim_start_matches('/');

        // Step 1: Collect parent policies from Monorepo root down to CL directory
        // This enables inheritance from e.g. /project/.cedar/policies.cedar
        let parent_dirs = self.collect_parent_policy_dirs();

        for parent_dir in parent_dirs {
            // Use absolute path as key to avoid collision with CL-local policies
            let absolute_policy_path = if parent_dir.is_empty() {
                "/.cedar/policies.cedar".to_string()
            } else {
                format!("/{}/.cedar/policies.cedar", parent_dir)
            };

            if seen_policies.contains(&absolute_policy_path) {
                continue;
            }

            // For parent policies, we use a rooted MonoApiService
            if let Some(content) = self
                .get_parent_policy_content(&mono_api_service, &parent_dir)
                .await
            {
                seen_policies.insert(absolute_policy_path.clone());
                policy_contents.push((PathBuf::from(&absolute_policy_path), content));
            }
        }

        // Step 2: Collect policies within the CL directory
        for dir in sorted_dirs {
            let policy_relative_path = if dir.as_os_str().is_empty() {
                ".cedar/policies.cedar".to_string()
            } else {
                let dir_str = dir.to_string_lossy().replace('\\', "/");
                format!("{}/.cedar/policies.cedar", dir_str)
            };

            // Build absolute path for deduplication
            let absolute_policy_path = if self_path_normalized.is_empty() {
                format!("/{}", policy_relative_path)
            } else {
                format!("/{}/{}", self_path_normalized, policy_relative_path)
            };

            // Skip if already seen from parent collection
            if seen_policies.contains(&absolute_policy_path) {
                continue;
            }

            let lookup_path = PathBuf::from(&policy_relative_path);

            // Fetch policy content: try from_hash for existing, fall back to to_hash for new
            let content = if self.from_hash != ZERO_ID {
                if let Ok(Some(content)) = mono_api_service
                    .get_blob_as_string(lookup_path.clone(), Some(&self.from_hash))
                    .await
                {
                    Some(content)
                } else {
                    mono_api_service
                        .get_blob_as_string(lookup_path, Some(&self.to_hash))
                        .await
                        .ok()
                        .flatten()
                }
            } else {
                mono_api_service
                    .get_blob_as_string(lookup_path, Some(&self.to_hash))
                    .await
                    .ok()
                    .flatten()
            };

            if let Some(content) = content {
                seen_policies.insert(absolute_policy_path.clone());
                policy_contents.push((PathBuf::from(&absolute_policy_path), content));
            }
        }

        policy_contents
    }

    /// Collect parent directory paths from Monorepo root to CL directory (exclusive).
    fn collect_parent_policy_dirs(&self) -> Vec<String> {
        let self_path_str = self.path.to_string_lossy().replace('\\', "/");
        let self_path_normalized = self_path_str.trim_start_matches('/');

        if self_path_normalized.is_empty() {
            return vec![];
        }

        let mut parent_dirs = Vec::new();
        let components: Vec<&str> = self_path_normalized.split('/').collect();

        // Add root directory
        parent_dirs.push(String::new());

        // Add each parent level except the CL directory itself
        let mut current_path = String::new();
        for (i, component) in components.iter().enumerate() {
            if i == components.len() - 1 {
                break;
            }
            if current_path.is_empty() {
                current_path = component.to_string();
            } else {
                current_path = format!("{}/{}", current_path, component);
            }
            parent_dirs.push(current_path.clone());
        }

        parent_dirs
    }

    /// Get policy content from a parent directory using storage directly.
    async fn get_parent_policy_content(
        &self,
        _mono_api_service: &MonoApiService,
        parent_dir: &str,
    ) -> Option<String> {
        let storage = self.storage.mono_storage();

        // Get the main ref for the parent directory
        let parent_path = if parent_dir.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", parent_dir)
        };

        let refs = storage.get_main_ref(&parent_path).await.ok()??;

        // Create a temporary MonoApiService for the parent path
        let parent_mono = MonoApiService {
            storage: self.storage.clone(),
            git_object_cache: self.git_object_cache.clone(),
        };

        // Look up .cedar/policies.cedar in the parent directory
        let policy_path = PathBuf::from(".cedar/policies.cedar");
        parent_mono
            .get_blob_as_string(policy_path, Some(&refs.ref_commit_hash))
            .await
            .ok()
            .flatten()
    }

    /// Auto-assign system required reviewers based on Cedar policy files.
    async fn assign_system_reviewers(&self) -> Result<(), MegaError> {
        let link_guard = self.cl_link.read().await;
        let cl_link = link_guard
            .as_ref()
            .ok_or_else(|| MegaError::Other("CL link not available".to_string()))?;

        let changed_files = self.get_changed_files().await?;
        let policy_contents = self.collect_policy_contents(&changed_files).await;

        if policy_contents.is_empty() {
            return Ok(());
        }

        let reviewer_service = ReviewerService::from_storage(self.storage.reviewer_storage());
        reviewer_service
            .assign_system_reviewers(cl_link, &policy_contents, &changed_files)
            .await?;

        Ok(())
    }

    pub async fn post_cl_operation(&self) -> Result<(), MegaError> {
        let link_guard = self.cl_link.read().await;
        let link = link_guard.as_ref().ok_or_else(|| {
            MegaError::Other(
                "CL link not available. This may occur if refs update failed.".to_string(),
            )
        })?;
        let cl_info = self
            .storage
            .cl_storage()
            .get_cl(link)
            .await?
            .ok_or_else(|| MegaError::Other(format!("CL not found for link: {}", link)))?;

        if self.bellatrix.enable_build() {
            let old_files = self.get_commit_blobs(&cl_info.from_hash).await?;
            let new_files = self.get_commit_blobs(&cl_info.to_hash).await?;
            let cl_diff_files = self.cl_files_list(old_files, new_files.clone()).await?;

            let cl_base = PathBuf::from(&cl_info.path);
            let changes = cl_diff_files
                .into_iter()
                .map(|m| {
                    let mut item: crate::model::change_list::ClFilesRes = m.into();
                    item.path = cl_base.join(item.path).to_string_lossy().to_string();
                    item
                })
                .collect::<Vec<_>>();

            let path_str = cl_base.to_str().ok_or_else(|| {
                MegaError::Other(format!("CL base path is not valid UTF-8: {:?}", cl_base))
            })?;
            let counter_changes: Vec<_> = changes
                .iter()
                .filter(|&s| PathBuf::from(&s.path).starts_with(&cl_base))
                .map(|s| {
                    let path = ProjectRelativePath::from_abs(&s.path, path_str).unwrap();
                    if s.action == "new" {
                        Status::Added(path)
                    } else if s.action == "deleted" {
                        Status::Removed(path)
                    } else if s.action == "modified" {
                        Status::Modified(path)
                    } else {
                        unreachable!()
                    }
                })
                .collect();

            tracing::info!(
                "Trigger bellatrix build for cl: {}, changes: {:?}, repo: {}",
                cl_info.id,
                counter_changes,
                path_str
            );

            let req: OrionBuildRequest = OrionBuildRequest {
                cl_link: link.clone(),
                mount_path: path_str.to_string(),
                cl: cl_info.id,
                builds: vec![BuildInfo {
                    changes: counter_changes,
                }],
            };
            let bellatrix = self.bellatrix.clone();
            tokio::spawn(async move {
                let _ = bellatrix.on_post_receive(req).await;
            });
        }

        let check_reg = CheckerRegistry::new(self.storage.clone().into(), self.username());
        check_reg.run_checks(cl_info.clone().into()).await?;
        Ok(())
    }

    pub async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        let api_service: MonoApiService = self.into();
        api_service.get_commit_blobs(commit_hash).await
    }

    pub async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<crate::model::change_list::ClDiffFile>, MegaError> {
        let api_service: MonoApiService = self.into();
        api_service.cl_files_list(old_files, new_files).await
    }

    // Mark code review threads whose anchors may be affected by this change as outdated.
    // These threads will require reanchoring to restore accurate code positions.
    pub async fn reanchor_code_review_threads(&self) -> Result<(), MegaError> {
        let mono_api_service: MonoApiService = self.into();
        let link_guard = self.cl_link.read().await;
        let cl_link = link_guard
            .as_ref()
            .ok_or_else(|| MegaError::Other("CL link not available".to_string()))?;

        // Marks code review threads as outdated if their file paths
        // are affected by the latest change list.
        let changed_files = self.get_changed_files().await?;
        let files_with_threads = self
            .storage
            .code_review_thread_storage()
            .get_files_with_threads_by_link(cl_link)
            .await?;

        let files_with_threads_set: HashSet<&String> = files_with_threads.iter().collect();

        // Intersection: files that are changed AND have threads
        let affected_files: Vec<String> = changed_files
            .into_iter()
            .filter(|file| files_with_threads_set.contains(file))
            .collect();

        tracing::info!(
            "Reanchor code review thread in cl_link: {}, affected files: {:?}",
            cl_link,
            affected_files
        );

        let pending_reanchor_threads = self
            .storage
            .code_review_thread_storage()
            .find_threads_by_file_paths(affected_files)
            .await?;

        let pending_reanchor_thread_ids: Vec<i64> = pending_reanchor_threads
            .iter()
            .map(|thread| thread.id)
            .collect();

        // Mark as PendingReanchor
        self.storage
            .code_review_thread_storage()
            .mark_positions_status_by_thread_ids(
                &pending_reanchor_thread_ids,
                PositionStatusEnum::PendingReanchor,
            )
            .await?;

        // Start reanchor
        let anchors = self
            .storage
            .code_review_thread_storage()
            .get_anchors_by_thread_ids(&pending_reanchor_thread_ids)
            .await?;

        let mono_api_service = Arc::new(mono_api_service);
        let mut anchors_map: HashMap<i64, Vec<mega_code_review_anchor::Model>> = HashMap::new();
        for anchor in anchors {
            anchors_map
                .entry(anchor.thread_id)
                .or_default()
                .push(anchor);
        }

        let reanchor_tasks: Vec<_> = pending_reanchor_threads
            .into_iter()
            .map(|thread| {
                let mono_api_service = Arc::clone(&mono_api_service);
                let anchors_map = anchors_map.clone();
                let to_hash = self.to_hash.clone();

                async move {
                    let thread_id = thread.id;

                    let thread_anchors = match anchors_map.get(&thread_id) {
                        Some(anchors) => anchors,
                        None => {
                            tracing::warn!("Thread {} has no anchors", thread_id);
                            return Err(MegaError::Other(format!(
                                "Thread {} has no anchors",
                                thread_id
                            )));
                        }
                    };

                    let (diff_content, _) = mono_api_service
                        .paged_content_diff(cl_link, Pagination::default())
                        .await?;

                    let mut blob_cache: HashMap<String, String> = HashMap::new();

                    for anchor in thread_anchors {
                        let file_path = anchor.file_path.clone();

                        // Fetch blob once per file
                        let latest_blob = if let Some(blob) = blob_cache.get(&file_path) {
                            blob.clone()
                        } else {
                            let blob = mono_api_service
                                .get_blob_as_string(PathBuf::from(&file_path), Some(&to_hash))
                                .await?
                                .expect("latest blob must exist");

                            blob_cache.insert(file_path.clone(), blob.clone());
                            blob
                        };

                        // Reanchor
                        if let Err(e) = self
                            .storage
                            .code_review_service
                            .reanchor_thread(
                                anchor,
                                Some(latest_blob),
                                diff_content.clone(),
                                &self.to_hash,
                            )
                            .await
                        {
                            tracing::error!("Reanchor failed for anchor {}: {:?}", anchor.id, e);
                        }
                    }

                    Ok(())
                }
            })
            .collect::<Vec<_>>();

        let results: Vec<Result<(), MegaError>> = stream::iter(reanchor_tasks)
            .buffer_unordered(self.storage.get_recommended_batch_concurrency())
            .collect()
            .await;

        for res in results {
            if let Err(e) = res {
                tracing::error!("Reanchor task failed: {:?}", e);
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
type DiffResult = Vec<(PathBuf, Option<ObjectHash>, Option<ObjectHash>)>;

#[allow(dead_code)]
fn diff_trees(theirs: &Tree, base: &Tree) -> Result<DiffResult, MegaError> {
    let their_items: HashMap<_, _> = get_plain_items(theirs).into_iter().collect();
    let base_items: HashMap<_, _> = get_plain_items(base).into_iter().collect();
    let all_paths: HashSet<_> = their_items.keys().chain(base_items.keys()).collect();

    let mut diffs = Vec::new();

    for path in all_paths {
        let their_hash = their_items.get(path).cloned();
        let base_hash = base_items.get(path).cloned();
        if their_hash != base_hash {
            diffs.push((path.clone(), their_hash, base_hash));
        }
    }
    Ok(diffs)
}

#[allow(dead_code)]
fn get_plain_items(tree: &Tree) -> Vec<(PathBuf, ObjectHash)> {
    let mut items = Vec::new();
    for item in tree.tree_items.iter() {
        if item.is_tree() {
            items.push((PathBuf::from(item.name.clone()), item.id));
        }
    }
    items
}
