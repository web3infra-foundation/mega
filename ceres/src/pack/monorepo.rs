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
use async_recursion::async_recursion;
use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use bellatrix::{Bellatrix, orion_client::BuildInfo, orion_client::OrionBuildRequest};
use callisto::{
    entity_ext::generate_link, mega_cl, mega_refs, raw_blob, sea_orm_active_enums::ConvTypeEnum,
};
use common::{
    errors::MegaError,
    utils::{self, ZERO_ID},
};
use git_internal::{
    errors::GitError,
    hash::SHA1,
    internal::{
        object::{ObjectTrait, commit::Commit, tree::Tree, types::ObjectType},
        pack::{encode::PackEncoder, entry::Entry},
    },
};
use git_internal::internal::metadata::{EntryMeta, MetaAttached};
use jupiter::storage::Storage;
use jupiter::utils::converter::FromMegaModel;

use crate::{
    api_service::{ApiHandler, mono_api_service::MonoApiService},
    merge_checker::CheckerRegistry,
    model::change_list::BuckFile,
    pack::RepoHandler,
    protocol::import_refs::{RefCommand, Refs},
};

pub struct MonoRepo {
    pub storage: Storage,
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

                storage.save_refs(new_mega_ref.clone()).await.unwrap();
                storage.save_mega_commits(vec![c.clone()]).await.unwrap();

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
        Ok(())
    }

    async fn save_entry(&self, entry_list: Vec<MetaAttached<Entry,EntryMeta>>) -> Result<(), MegaError> {
        let storage = self.storage.mono_storage();
        let current_commit = self.current_commit.read().await;
        let commit_id = if let Some(commit) = &*current_commit {
            commit.id.to_string()
        } else {
            String::new()
        };
        storage
            .save_entry(&commit_id, entry_list, self.username.clone())
            .await
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
        let want_trees: HashMap<SHA1, Tree> = storage
            .get_trees_by_hashes(want_tree_ids)
            .await
            .unwrap()
            .into_iter()
            .map(|m| {
                (
                    SHA1::from_str(&m.tree_id).unwrap(),
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
                .await;
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
            .await;
            entry_tx.send(MetaAttached{inner:c.into(),meta:EntryMeta::new()}).await.unwrap();
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
    ) -> Result<Vec<raw_blob::Model>, MegaError> {
        self.storage
            .raw_db_storage()
            .get_raw_blobs_by_hashes(hashes)
            .await
    }

    async fn get_blob_metadata_by_hashes(&self, hashes: Vec<String>) -> Result<HashMap<String,EntryMeta>, MegaError> {
        let models = self.storage
            .mono_storage()
            .get_mega_blobs_by_hashes(hashes).await?;
        
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
                    },
                )
            })
            .collect::<HashMap<String, EntryMeta>>();
        
        Ok(map)
    }

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError> {
        let storage = self.storage.mono_storage();
        let current_commit = self.current_commit.read().await;
        let cl_link = self.fetch_or_new_cl_link().await.unwrap();
        let ref_name = utils::cl_ref_name(&cl_link);
        if let Some(c) = &*current_commit {
            if let Some(mut cl_ref) = storage.get_ref_by_name(&ref_name).await.unwrap() {
                cl_ref.ref_commit_hash = refs.new_id.clone();
                cl_ref.ref_tree_hash = c.tree_id.to_string();
                storage.update_ref(cl_ref).await.unwrap();
            } else {
                let refs = mega_refs::Model::new(
                    &self.path,
                    ref_name,
                    refs.new_id.clone(),
                    c.tree_id.to_string(),
                    true,
                );
                storage.save_refs(refs).await.unwrap();
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
        let trees = self.storage
            .mono_storage()
            .get_trees_by_hashes(tree_hashes)
            .await
            .map_err(|e| {
                MegaError::with_message(&format!(
                    "Failed to retrieve root tree for commit {}: {}", 
                    commit_opt.id, e
                ))
            })?;

        if trees.is_empty() {
            return Err(MegaError::with_message(&format!(
                "Root tree {} not found for commit {}", 
                commit_opt.tree_id, commit_opt.id
            )));
        }

        let root_tree = Tree::from_mega_model(trees[0].clone());
        
        tracing::info!(
            "Starting file path update for commit {} with root tree {}", 
            commit_opt.id, commit_opt.tree_id
        );

        
        self.traverses_and_update_filepath(root_tree, PathBuf::new()).await
            .map_err(|e| {
                MegaError::with_message(&format!(
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
    async fn traverses_and_update_filepath(&self, tree: Tree, path: PathBuf) -> Result<(), MegaError> {
        for item in tree.tree_items {
            let item_path = path.join(&item.name);
            
            if item.is_tree() {
                // 处理子树
                let tree_hash = item.id.to_string();
                let trees = self.storage
                    .mono_storage()
                    .get_trees_by_hashes(vec![tree_hash.clone()])
                    .await
                    .map_err(|e| {
                        MegaError::with_message(&format!(
                            "Failed to retrieve tree {} at path '{}': {}", 
                            tree_hash, item_path.display(), e
                        ))
                    })?;

                if trees.is_empty() {
                    return Err(MegaError::with_message(&format!(
                        "Tree {} not found at path '{}'", 
                        tree_hash, item_path.display()
                    )));
                }

                let child_tree = Tree::from_mega_model(trees[0].clone());
                
                // 递归处理子树
                self.traverses_and_update_filepath(child_tree, item_path.clone()).await
                    .map_err(|e| {
                        MegaError::with_message(&format!(
                            "Failed to process subtree {} at path '{}': {}", 
                            tree_hash, item_path.display(), e
                        ))
                    })?;
            } else {
                // 处理 blob 文件
                let blob_id = item.id.to_string();
                let file_path_str = item_path.to_str().ok_or_else(|| {
                    MegaError::with_message(&format!(
                        "Invalid UTF-8 path for blob {}: '{}'", 
                        blob_id, item_path.display()
                    ))
                })?;

                // 更新 blob 的文件路径
                self.storage.mono_storage()
                    .update_blob_filepath(&blob_id, file_path_str)
                    .await
                    .map_err(|e| {
                        MegaError::with_message(&format!(
                            "Failed to update file path for blob {} at '{}': {}", 
                            blob_id, file_path_str, e
                        ))
                    })?;

                tracing::debug!(
                    "Updated file path for blob {} to '{}'", 
                    blob_id, file_path_str
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
            .await
            .unwrap()
        {
            Some(cl) => cl.link.clone(),
            None => {
                if self.from_hash == "0".repeat(40) {
                    return Err(MegaError::with_message(
                        "Can not init directory under monorepo directory!",
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
            cl_stg
                .update_cl_hash(cl, &self.from_hash, &self.to_hash)
                .await?;
        }
        Ok(())
    }

    async fn search_buck_under_cl(&self, cl_path: &Path) -> Result<Vec<BuckFile>, MegaError> {
        let mut res = vec![];
        let mono_stg = self.storage.mono_storage();
        let mono_api_service = MonoApiService {
            storage: self.storage.clone(),
        };

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

        // no buck file found
        if res.is_empty() {
            let mut path = Some(cl_path);
            while let Some(p) = path {
                if p.parent().is_some()
                    && let Some(tree) = mono_api_service.search_tree_by_path(p).await.ok().flatten()
                    && let Some(buck) = self.try_extract_buck(tree, cl_path)
                {
                    return Ok(vec![buck]);
                };

                path = p.parent();
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
    ) -> Result<Vec<(PathBuf, Option<SHA1>, Option<SHA1>)>, MegaError> {
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
        let path_str = self.path.to_str().unwrap();
        let username = self.username();

        match storage.get_open_cl_by_path(path_str, &username).await? {
            Some(cl) => {
                self.update_existing_cl(cl).await?;
            }
            None => {
                let link_guard = self.cl_link.read().await;
                let cl_link = link_guard.as_ref().unwrap();
                let commit_guard = self.current_commit.read().await;
                let title = if let Some(commit) = commit_guard.as_ref() {
                    commit.format_message()
                } else {
                    String::new()
                };
                storage
                    .new_cl(
                        path_str,
                        cl_link,
                        &title,
                        &self.from_hash,
                        &self.to_hash,
                        &username,
                    )
                    .await?;
            }
        };
        Ok(())
    }

    pub async fn post_cl_operation(&self) -> Result<(), MegaError> {
        let link_guard = self.cl_link.read().await;
        let link = link_guard.as_ref().unwrap();
        let cl_info = self
            .storage
            .cl_storage()
            .get_cl(link)
            .await?
            .ok_or_else(|| MegaError::with_message(format!("CL not found for link: {}", link)))?;

        if self.bellatrix.enable_build() {
            let buck_files = self.search_buck_under_cl(&self.path).await?;
            if buck_files.is_empty() {
                tracing::error!(
                    "Search BUCK file under {:?} failed, please manually check BUCK file exists!!",
                    self.path
                );
            } else {
                for buck_file in buck_files {
                    let req = OrionBuildRequest {
                        repo: buck_file.path.to_str().unwrap().to_string(),
                        cl_link: link.to_string(),
                        cl: cl_info.id,
                        task_name: None,
                        template: None,
                        builds: vec![BuildInfo {
                            buck_hash: buck_file.buck.to_string(),
                            buckconfig_hash: buck_file.buck_config.to_string(),
                            args: Some(vec![]),
                        }],
                    };
                    let bellatrix = self.bellatrix.clone();
                    tokio::spawn(async move {
                        let _ = bellatrix.on_post_receive(req).await;
                    });
                }
            }
        }

        let check_reg = CheckerRegistry::new(self.storage.clone().into(), self.username());
        check_reg.run_checks(cl_info.clone().into()).await?;
        Ok(())
    }
}

type DiffResult = Vec<(PathBuf, Option<SHA1>, Option<SHA1>)>;

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

fn get_plain_items(tree: &Tree) -> Vec<(PathBuf, SHA1)> {
    let mut items = Vec::new();
    for item in tree.tree_items.iter() {
        if item.is_tree() {
            items.push((PathBuf::from(item.name.clone()), item.id));
        }
    }
    items
}

#[cfg(test)]
mod test {
    use std::path::{Component, Path, PathBuf};
    use std::str::FromStr;
    use std::sync::{Arc};
    use git_internal::hash::SHA1;
    use git_internal::internal::object::tree::{Tree, TreeItem, TreeItemMode};
    use tokio::sync::RwLock;
    use crate::pack::monorepo::MonoRepo;
    use crate::pack::RepoHandler;

    #[test]
    fn get_component_reverse() {
        let reversed: Vec<_> = Path::new("/a/b/c/d.txt")
            .components()
            .filter_map(|c| match c {
                Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                _ => None,
            })
            .rev()
            .collect();

        assert_eq!(vec!["d.txt", "c", "b", "a"], reversed); // ["d.txt", "c", "b", "a"]
    }

    // 创建测试用的 MonoRepo 实例
    async fn create_test_mono_repo() -> MonoRepo {
        use common::config::BuildConfig;
        use bellatrix::Bellatrix;
        use jupiter::tests::test_storage;
        use tempfile::TempDir;
        
        // 创建临时目录和测试存储
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let storage = test_storage(temp_dir.path()).await;
        
        // 创建测试 Bellatrix
        let bellatrix = Arc::new(Bellatrix::new(BuildConfig::default()));
        
        MonoRepo {
            storage,
            path: PathBuf::from("/test/repo"),
            from_hash: "from_hash".to_string(),
            to_hash: "to_hash".to_string(),
            current_commit: Arc::new(RwLock::new(None)),
            cl_link: Arc::new(RwLock::new(None)),
            bellatrix,
            username: Some("test_user".to_string()),
        }
    }

    #[tokio::test]
    async fn test_traverses_tree_and_update_filepath_with_no_commit() {
        let mono_repo = create_test_mono_repo().await;

        // 测试当 current_commit 为 None 时的情况
        let result = mono_repo.traverses_tree_and_update_filepath().await;

        // 应该优雅地处理这种情况，记录日志并跳过更新
        assert!(result.is_ok(), "Should handle None current_commit gracefully");
    }

    #[tokio::test]
    async fn test_traverses_and_update_filepath_with_files() {
        let mono_repo = create_test_mono_repo().await;

        // 创建测试树结构
        let blob_sha1 = SHA1::from_str("1234567890abcdef1234567890abcdef12345678").unwrap();
        let tree_items = vec![
            TreeItem {
                mode: TreeItemMode::Blob,
                name: "test_file.txt".to_string(),
                id: blob_sha1,
                
            }
        ];

        let tree = Tree {
            id: SHA1::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap(),
            tree_items,
        };

        let path = PathBuf::from("src");

        // 测试遍历和更新文件路径
        let result = mono_repo.traverses_and_update_filepath(tree, path).await;

        // 注意：这个测试需要配置正确的数据库环境才能真正验证
        // 在实际测试环境中，应该验证数据库中的 file_path 字段是否正确更新
        println!("Test result: {:?}", result);
    }

    

    // 验证 UTF-8 路径处理
    #[test]
    fn test_utf8_path_handling() {
        let path = PathBuf::from("src/测试文件.txt");
        let path_str = path.to_str();

        assert!(path_str.is_some(), "Should handle UTF-8 paths correctly");
        assert_eq!(path_str.unwrap(), "src/测试文件.txt");
    }

}
