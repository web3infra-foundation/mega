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

use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use bellatrix::{Bellatrix, orion_client::BuildInfo, orion_client::OrionBuildRequest};
use callisto::{entity_ext::generate_link, mega_cl, raw_blob, sea_orm_active_enums::ConvTypeEnum};
use common::{
    errors::MegaError,
    utils::{self, MEGA_BRANCH_NAME},
};
use git_internal::{
    errors::GitError,
    hash::SHA1,
    internal::{
        object::{ObjectTrait, commit::Commit, tree::Tree, types::ObjectType},
        pack::{encode::PackEncoder, entry::Entry},
    },
};
use jupiter::storage::Storage;
use jupiter::utils::converter::FromMegaModel;

use crate::{
    api_service::{ApiHandler, mono_api_service::MonoApiService},
    merge_checker::CheckerRegistry,
    model::cl::BuckFile,
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

    async fn head_hash(&self) -> (String, Vec<Refs>) {
        let storage = self.storage.mono_storage();

        let result = storage.get_refs(self.path.to_str().unwrap()).await.unwrap();

        let heads_exist = result
            .iter()
            .any(|x| x.ref_name == common::utils::MEGA_BRANCH_NAME);

        let refs = if heads_exist {
            let refs: Vec<Refs> = result.into_iter().map(|x| x.into()).collect();
            refs
        } else {
            let target_path = self.path.clone();
            let refs = storage.get_ref("/").await.unwrap().unwrap();
            let tree_hash = refs.ref_tree_hash.clone();

            let mut tree: Tree =
                Tree::from_mega_model(storage.get_tree_by_hash(&tree_hash).await.unwrap().unwrap());

            let commit: Commit = Commit::from_mega_model(
                storage
                    .get_commit_by_hash(&refs.ref_commit_hash)
                    .await
                    .unwrap()
                    .unwrap(),
            );

            for component in target_path.components() {
                if component != Component::RootDir {
                    let path_name = component.as_os_str().to_str().unwrap();
                    let sha1 = tree
                        .tree_items
                        .iter()
                        .find(|x| x.name == path_name)
                        .map(|x| x.id);
                    if let Some(sha1) = sha1 {
                        tree = Tree::from_mega_model(
                            storage
                                .get_trees_by_hashes(vec![sha1.to_string()])
                                .await
                                .unwrap()[0]
                                .clone(),
                        );
                    } else {
                        return self.find_head_hash(vec![]);
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
            storage
                .save_ref(
                    self.path.to_str().unwrap(),
                    None,
                    &c.id.to_string(),
                    &c.tree_id.to_string(),
                    false,
                )
                .await
                .unwrap();
            storage.save_mega_commits(vec![c.clone()]).await.unwrap();

            vec![Refs {
                ref_name: MEGA_BRANCH_NAME.to_string(),
                ref_hash: c.id.to_string(),
                default_branch: true,
                ..Default::default()
            }]
        };
        self.find_head_hash(refs)
    }

    async fn post_receive_pack(&self) -> Result<(), MegaError> {
        self.save_or_update_cl().await?;
        self.post_cl_operation().await?;
        Ok(())
    }

    async fn save_entry(&self, entry_list: Vec<Entry>) -> Result<(), MegaError> {
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

        for c in want_commits {
            self.traverse(
                want_trees.get(&c.tree_id).unwrap().clone(),
                &mut exist_objs,
                Some(&entry_tx),
            )
            .await;
            entry_tx.send(c.into()).await.unwrap();
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
                storage
                    .save_ref(
                        self.path.to_str().unwrap(),
                        Some(ref_name),
                        &refs.new_id,
                        &c.tree_id.to_string(),
                        true,
                    )
                    .await
                    .unwrap();
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
}

impl MonoRepo {
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
        self.username.clone().unwrap_or(String::from("Admin"))
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
    use std::path::{Component, Path};

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
}
