use std::sync::Arc;

use futures::{StreamExt, stream};

use git_internal::internal::object::blob::Blob;
use sea_orm::IntoActiveModel;
use tokio::sync::Mutex;

use callisto::{mega_blob, mega_commit, mega_tag, mega_tree};
use common::errors::MegaError;
use git_internal::internal::metadata::{EntryMeta, MetaAttached};

use git_internal::internal::pack::entry::Entry;

use crate::service::git_service::GitService;
use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::mono_storage::MonoStorage;
use crate::utils::converter::{IntoMegaModel, MegaObjectModel, process_entry};

#[derive(Clone)]
pub struct MonoService {
    pub mono_storage: MonoStorage,
    pub git_service: GitService,
}

#[derive(Debug, Default)]
pub struct GitMegaObjects {
    commits: Vec<mega_commit::ActiveModel>,
    trees: Vec<mega_tree::ActiveModel>,
    blobs: Vec<mega_blob::ActiveModel>,
    tags: Vec<mega_tag::ActiveModel>,
}

impl MonoService {
    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        let mono_storage = MonoStorage { base: mock.clone() };
        let git_service = GitService::mock();

        Self {
            mono_storage,
            git_service,
        }
    }

    pub async fn save_entry(
        &self,
        commit_id: &str,
        entry_list: Vec<MetaAttached<Entry, EntryMeta>>,
    ) -> Result<Vec<mega_commit::ActiveModel>, MegaError> {
        let git_objects = Arc::new(Mutex::new(GitMegaObjects::default()));

        let results: Vec<Result<(), MegaError>> = stream::iter(entry_list)
            .map(|entry| {
                let git_objects = git_objects.clone();
                async move {
                    let raw_obj = process_entry(entry.inner);

                    let model = raw_obj.convert_to_mega_model(entry.meta);
                    match model {
                        MegaObjectModel::Commit(commit) => git_objects
                            .lock()
                            .await
                            .commits
                            .push(commit.into_active_model()),
                        MegaObjectModel::Tree(mut tree) => {
                            commit_id.clone_into(&mut tree.commit_id);
                            git_objects
                                .lock()
                                .await
                                .trees
                                .push(tree.into_active_model());
                        }
                        MegaObjectModel::Blob(mut blob, raw) => {
                            commit_id.clone_into(&mut blob.commit_id);

                            self.git_service
                                .save_object_from_model(raw, &blob.blob_id)
                                .await?;
                            git_objects
                                .lock()
                                .await
                                .blobs
                                .push(blob.into_active_model());
                        }
                        MegaObjectModel::Tag(tag) => {
                            git_objects.lock().await.tags.push(tag.into_active_model())
                        }
                    }
                    Ok(())
                }
            })
            .buffer_unordered(16)
            .collect()
            .await;

        if let Some(err) = results.into_iter().find_map(Result::err) {
            tracing::error!("at least one blob upload to object storage failed");
            return Err(err);
        }

        let git_objects = Arc::try_unwrap(git_objects)
            .expect("Failed to unwrap Arc")
            .into_inner();

        self.mono_storage
            .batch_save_model(git_objects.commits.clone())
            .await?;
        self.mono_storage
            .batch_save_model(git_objects.trees)
            .await?;
        self.mono_storage
            .batch_save_model(git_objects.blobs)
            .await?;
        self.mono_storage.batch_save_model(git_objects.tags).await?;

        Ok(git_objects.commits)
    }

    pub async fn save_blobs(&self, commit_id: &str, blobs: Vec<Blob>) -> Result<(), MegaError> {
        let mega_blobs: Vec<mega_blob::ActiveModel> = blobs
            .iter()
            .map(|b| (*b).clone().into_mega_model(EntryMeta::default()))
            .map(|mut m: mega_blob::Model| {
                m.commit_id = commit_id.to_owned();
                m.into_active_model()
            })
            .collect();
        self.mono_storage.batch_save_model(mega_blobs).await?;
        self.git_service.put_objects(blobs).await
    }
}
