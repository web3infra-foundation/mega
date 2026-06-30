use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use api_model::common::Pagination;
use async_trait::async_trait;
use callisto::{mega_cl, mega_code_review_anchor};
use common::{errors::MegaError, utils::ZERO_ID};
use futures::{StreamExt, stream};
use jupiter::storage::Storage;
use orion_client::OrionBuildClient;

use crate::{
    application::{
        api_service::{
            ApiHandler,
            cache::GitObjectCache,
            mono::{MonoApiService, cl_merge},
        },
        code_edit::{on_push::OnpushCodeEdit, utils::get_changed_files},
    },
    bus::{ApplicationEventHandler, TransportEvent},
};

/// Handles CL creation, bootstrap, build triggers, and code-review reanchoring after mono push.
pub async fn dispatch_mono_receive_pack_finalized(
    storage: Storage,
    git_object_cache: Arc<GitObjectCache>,
    repo_path: PathBuf,
    base_branch: String,
    from_hash: String,
    to_hash: String,
    username: Option<String>,
) -> Result<(), MegaError> {
    let username = username.unwrap_or_else(|| String::from("Anonymous"));
    let mono_api_service = MonoApiService {
        storage: storage.clone(),
        git_object_cache: git_object_cache.clone(),
    };
    let repo_path_str = repo_path
        .to_str()
        .ok_or_else(|| MegaError::Other("invalid repo path".to_string()))?;

    let editor = OnpushCodeEdit::from(repo_path_str, &base_branch, &from_hash, &mono_api_service);
    let cl = editor
        .update_or_create_cl(&storage, &from_hash, &to_hash, &username)
        .await?;

    if from_hash == ZERO_ID && repo_path_str.starts_with("/project/") {
        cl_merge::bootstrap_monorepo_path(&mono_api_service, repo_path_str, Some(&cl)).await?;
    }

    let orion_client = Arc::new(OrionBuildClient::new(storage.config().build.clone()));
    if orion_client.enable_build() {
        editor
            .trigger_build_and_check(
                storage.clone(),
                git_object_cache.clone(),
                orion_client,
                &cl,
                &username,
            )
            .await?;
    }

    reanchor_code_review_threads(&storage, &mono_api_service, &cl, &to_hash).await
}

async fn reanchor_code_review_threads(
    storage: &Storage,
    mono_api_service: &MonoApiService,
    cl: &mega_cl::Model,
    to_hash: &str,
) -> Result<(), MegaError> {
    let cl_link = cl.link.clone();
    let changed_files = get_changed_files(mono_api_service, cl).await?;
    let files_with_threads = storage
        .code_review_thread_storage()
        .get_files_with_threads_by_link(&cl_link)
        .await?;

    let files_with_threads_set: HashSet<&String> = files_with_threads.iter().collect();

    let affected_files: Vec<String> = changed_files
        .into_iter()
        .filter(|file| files_with_threads_set.contains(file))
        .collect();

    tracing::info!(
        "Reanchor code review thread in cl_link: {}, affected files: {:?}",
        cl_link,
        affected_files
    );

    let pending_reanchor_threads = storage
        .code_review_thread_storage()
        .find_threads_by_file_paths(affected_files)
        .await?;

    let pending_reanchor_thread_ids: Vec<i64> = pending_reanchor_threads
        .iter()
        .map(|thread| thread.id)
        .collect();

    storage
        .code_review_thread_storage()
        .mark_positions_status_by_thread_ids(
            &pending_reanchor_thread_ids,
            callisto::sea_orm_active_enums::PositionStatusEnum::PendingReanchor,
        )
        .await?;

    let anchors = storage
        .code_review_thread_storage()
        .get_anchors_by_thread_ids(&pending_reanchor_thread_ids)
        .await?;

    let mono_api_service = Arc::new(mono_api_service.clone());
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
            let cl_link = cl_link.clone();
            let mono_api_service = Arc::clone(&mono_api_service);
            let anchors_map = anchors_map.clone();
            let to_hash = to_hash.to_string();
            let storage = storage.clone();

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
                    .paged_content_diff(&cl_link, Pagination::default())
                    .await?;

                let mut blob_cache: HashMap<String, String> = HashMap::new();

                for anchor in thread_anchors {
                    let file_path = anchor.file_path.clone();

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

                    if let Err(e) = storage
                        .code_review_service
                        .reanchor_thread(anchor, Some(latest_blob), diff_content.clone(), &to_hash)
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
        .buffer_unordered(storage.get_recommended_batch_concurrency())
        .collect()
        .await;

    for res in results {
        if let Err(e) = res {
            tracing::error!("Reanchor task failed: {:?}", e);
        }
    }

    Ok(())
}

/// Application handler that dispatches transport events using storage + cache context.
pub struct RuntimeApplicationHandler {
    storage: Storage,
    git_object_cache: Arc<GitObjectCache>,
}

impl RuntimeApplicationHandler {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            storage,
            git_object_cache,
        }
    }
}

#[async_trait]
impl ApplicationEventHandler for RuntimeApplicationHandler {
    async fn handle(&self, event: TransportEvent) -> Result<(), MegaError> {
        match event {
            TransportEvent::MonoReceivePackFinalized {
                repo_path,
                base_branch,
                from_hash,
                to_hash,
                username,
            } => {
                dispatch_mono_receive_pack_finalized(
                    self.storage.clone(),
                    self.git_object_cache.clone(),
                    repo_path,
                    base_branch,
                    from_hash,
                    to_hash,
                    username,
                )
                .await
            }
            TransportEvent::ImportReceivePackFinalized {
                repo_path,
                repo_id,
                commands,
                unpack_redlock,
                extra_timings,
            } => {
                super::import::dispatch_import_receive_pack_finalized(
                    self.storage.clone(),
                    self.git_object_cache.clone(),
                    repo_path,
                    repo_id,
                    commands,
                    unpack_redlock,
                    extra_timings,
                )
                .await
            }
        }
    }
}
