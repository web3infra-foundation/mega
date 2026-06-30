use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::extract::FromRef;
use ceres::{
    api_service::{
        ApiHandler,
        cache::GitObjectCache,
        import_api_service::ImportApiService,
        mono::{MonoApiService, MonoAppServices},
    },
    application::artifact::ArtifactApplicationService,
    build_trigger::service::BuildTriggerService,
    protocol::repo::Repo,
    transport::ProtocolApiState,
};
use common::errors::ProtocolError;
use jupiter::storage::{Storage, user_storage::UserStorage};
use orion_client::OrionBuildClient;
use saturn::entitystore::EntityStore;
use tower_sessions::MemoryStore;

use crate::api::oauth::api_store::OAuthApiStore;
pub mod api_common;
pub mod api_doc;
pub mod api_router;
pub mod error;
pub mod guard;
pub mod notes;
pub mod oauth;
pub mod router;

#[derive(Clone)]
pub struct MonoApiServiceState {
    services: MonoAppServices,
    storage: Storage,
    git_object_cache: Arc<GitObjectCache>,
    session_store: Option<OAuthApiStore>,
    listen_addr: String,
    entity_store: EntityStore,
    orion_client: Arc<OrionBuildClient>,
}

impl MonoApiServiceState {
    pub fn new(
        storage: Storage,
        git_object_cache: Arc<GitObjectCache>,
        session_store: Option<OAuthApiStore>,
        listen_addr: String,
        entity_store: EntityStore,
        orion_client: Arc<OrionBuildClient>,
    ) -> Self {
        Self {
            services: MonoAppServices::new(storage.clone(), git_object_cache.clone()),
            storage,
            git_object_cache,
            session_store,
            listen_addr,
            entity_store,
            orion_client,
        }
    }

    pub fn listen_addr(&self) -> &str {
        &self.listen_addr
    }

    pub(crate) fn lfs_db_storage(&self) -> jupiter::storage::lfs_db_storage::LfsDbStorage {
        self.storage.lfs_db_storage()
    }

    pub(crate) fn lfs_service(&self) -> jupiter::service::lfs_service::LfsService {
        self.storage.lfs_service.clone()
    }

    pub fn monorepo(&self) -> MonoApiService {
        self.services.monorepo().clone()
    }

    pub fn services(&self) -> &MonoAppServices {
        &self.services
    }

    pub fn artifact_app_service(&self) -> ArtifactApplicationService {
        ArtifactApplicationService::from_storage(&self.storage)
    }

    pub fn build_trigger_service(&self) -> BuildTriggerService {
        BuildTriggerService::new(
            self.storage.clone(),
            self.git_object_cache.clone(),
            self.orion_client.clone(),
        )
    }

    pub(crate) async fn api_handler(
        &self,
        path: &Path,
    ) -> Result<Box<dyn ApiHandler>, ProtocolError> {
        let path = if path.has_root() {
            path.to_path_buf()
        } else {
            PathBuf::from("/").join(path)
        };

        let import_dir = self.monorepo().import_dir();
        if path.starts_with(&import_dir)
            && path != import_dir
            && let Some(model) = self
                .monorepo()
                .find_git_repo_like_path(path.to_str().unwrap())
                .await
                .unwrap()
        {
            let repo: Repo = model.into();
            return Ok(Box::new(ImportApiService {
                storage: self.storage.clone(),
                repo,
                git_object_cache: self.git_object_cache.clone(),
            }));
        }
        let ret: Box<dyn ApiHandler> = Box::<MonoApiService>::new(self.monorepo());

        #[allow(clippy::useless_conversion)]
        Ok(ret.into())
    }
}

impl FromRef<MonoApiServiceState> for MemoryStore {
    fn from_ref(_: &MonoApiServiceState) -> Self {
        MemoryStore::default()
    }
}

impl FromRef<MonoApiServiceState> for OAuthApiStore {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.session_store.clone().unwrap()
    }
}

impl FromRef<MonoApiServiceState> for UserStorage {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.storage.user_storage()
    }
}

impl FromRef<MonoApiServiceState> for EntityStore {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.entity_store.clone()
    }
}

impl From<&MonoApiServiceState> for MonoApiService {
    fn from(state: &MonoApiServiceState) -> Self {
        state.monorepo()
    }
}

impl FromRef<MonoApiServiceState> for ProtocolApiState {
    fn from_ref(state: &MonoApiServiceState) -> ProtocolApiState {
        ProtocolApiState::new(state.storage.clone(), state.git_object_cache.clone())
    }
}
