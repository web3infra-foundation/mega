use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::extract::FromRef;
use bellatrix::Bellatrix;
use ceres::{
    api_service::{
        ApiHandler, cache::GitObjectCache, import_api_service::ImportApiService,
        mono_api_service::MonoApiService, state::ProtocolApiState,
    },
    build_trigger::service::BuildTriggerService,
    protocol::repo::Repo,
};
use common::errors::ProtocolError;
use jupiter::{
    service::webhook_service::WebhookService,
    storage::{
        NotificationStorage, Storage, cl_storage::ClStorage,
        conversation_storage::ConversationStorage, dynamic_sidebar_storage::DynamicSidebarStorage,
        gpg_storage::GpgStorage, issue_storage::IssueStorage, note_storage::NoteStorage,
        user_storage::UserStorage, webhook_storage::WebhookStorage,
    },
};
use saturn::entitystore::EntityStore;
use tower_sessions::MemoryStore;

use crate::api::oauth::api_store::OAuthApiStore;
pub mod api_common;
pub mod api_router;
pub mod error;
pub mod guard;
pub mod notes;
pub mod oauth;
pub mod router;

#[derive(Clone)]
pub struct MonoApiServiceState {
    pub storage: Storage,
    pub git_object_cache: Arc<GitObjectCache>,
    pub session_store: Option<OAuthApiStore>,
    pub listen_addr: String,
    pub entity_store: EntityStore,
    pub bellatrix: Arc<Bellatrix>,
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
        MonoApiService {
            storage: state.storage.clone(),
            git_object_cache: state.git_object_cache.clone(),
        }
    }
}

impl FromRef<MonoApiServiceState> for ProtocolApiState {
    fn from_ref(state: &MonoApiServiceState) -> ProtocolApiState {
        ProtocolApiState {
            storage: state.storage.clone(),
            git_object_cache: state.git_object_cache.clone(),
        }
    }
}

impl MonoApiServiceState {
    fn monorepo(&self) -> MonoApiService {
        self.into()
    }

    fn issue_stg(&self) -> IssueStorage {
        self.storage.issue_storage()
    }

    fn gpg_stg(&self) -> GpgStorage {
        self.storage.gpg_storage()
    }

    fn cl_stg(&self) -> ClStorage {
        self.storage.cl_storage()
    }

    fn user_stg(&self) -> UserStorage {
        self.storage.user_storage()
    }

    fn notification_stg(&self) -> NotificationStorage {
        self.storage.notification_storage()
    }

    fn conv_stg(&self) -> ConversationStorage {
        self.storage.conversation_storage()
    }

    fn note_stg(&self) -> NoteStorage {
        self.storage.note_storage()
    }

    fn webhook_stg(&self) -> WebhookStorage {
        self.storage.webhook_storage()
    }

    fn webhook_svc(&self) -> WebhookService {
        self.storage.webhook_service.clone()
    }

    fn dynamic_sidebar_stg(&self) -> DynamicSidebarStorage {
        self.storage.dynamic_sidebar_storage()
    }

    pub fn build_trigger_service(&self) -> BuildTriggerService {
        BuildTriggerService::new(
            self.storage.clone(),
            self.git_object_cache.clone(),
            self.bellatrix.clone(),
        )
    }

    async fn api_handler(&self, path: &Path) -> Result<Box<dyn ApiHandler>, ProtocolError> {
        // Normalize path to ensure it has a root component
        let path = if path.has_root() {
            path.to_path_buf()
        } else {
            PathBuf::from("/").join(path)
        };

        let import_dir = self.storage.config().monorepo.import_dir.clone();
        if path.starts_with(&import_dir)
            && path != import_dir
            && let Some(model) = self
                .storage
                .git_db_storage()
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
        let ret: Box<dyn ApiHandler> = Box::<MonoApiService>::new(self.into());

        // Rust-analyzer cannot infer the type of `ret` correctly and always reports an error.
        // Use `.into()` to workaround this issue.
        #[allow(clippy::useless_conversion)]
        Ok(ret.into())
    }
}
