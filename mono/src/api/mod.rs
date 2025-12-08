use axum::extract::FromRef;
use oauth2::{
    Client, EndpointNotSet, EndpointSet, StandardRevocableToken,
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenResponse,
    },
};
use saturn::entitystore::EntityStore;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Semaphore;
use tower_sessions::MemoryStore;

use crate::api::oauth::campsite_store::CampsiteApiStore;
use ceres::{
    api_service::{
        ApiHandler, cache::GitObjectCache, import_api_service::ImportApiService,
        mono_api_service::MonoApiService, state::ProtocolApiState,
    },
    protocol::repo::Repo,
};
use common::errors::ProtocolError;
use jupiter::storage::{
    Storage, buck_storage::BuckStorage, cl_storage::ClStorage,
    conversation_storage::ConversationStorage, dynamic_sidebar_storage::DynamicSidebarStorage,
    issue_storage::IssueStorage, user_storage::UserStorage,
};
use jupiter::storage::{gpg_storage::GpgStorage, note_storage::NoteStorage};
pub mod api_common;
pub mod api_router;
pub mod error;
pub mod guard;
pub mod notes;
pub mod oauth;
pub mod router;

pub type GithubClient<
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointSet,
> = Client<
    BasicErrorResponse,
    BasicTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

#[derive(Clone)]
pub struct MonoApiServiceState {
    pub storage: Storage,
    pub git_object_cache: Arc<GitObjectCache>,
    pub oauth_client: Option<GithubClient>,
    pub session_store: Option<CampsiteApiStore>,
    pub listen_addr: String,
    pub entity_store: EntityStore,
    /// Buck upload concurrency limiter
    pub buck_upload_semaphore: Arc<Semaphore>,
    /// Buck upload large file concurrency limiter
    pub buck_large_file_semaphore: Arc<Semaphore>,
    /// Large file threshold in bytes
    pub buck_large_file_threshold: u64,
}

impl FromRef<MonoApiServiceState> for MemoryStore {
    fn from_ref(_: &MonoApiServiceState) -> Self {
        MemoryStore::default()
    }
}

impl FromRef<MonoApiServiceState> for CampsiteApiStore {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.session_store.clone().unwrap()
    }
}

impl FromRef<MonoApiServiceState> for GithubClient {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.oauth_client.clone().unwrap()
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

    fn conv_stg(&self) -> ConversationStorage {
        self.storage.conversation_storage()
    }

    fn note_stg(&self) -> NoteStorage {
        self.storage.note_storage()
    }

    fn dynamic_sidebar_stg(&self) -> DynamicSidebarStorage {
        self.storage.dynamic_sidebar_storage()
    }

    fn buck_stg(&self) -> BuckStorage {
        self.storage.buck_storage()
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

pub mod util {
    use std::path::PathBuf;

    use axum::extract::State;

    use cedar_policy::Context;
    use ceres::api_service::ApiHandler;
    use saturn::{ActionEnum, context::CedarContext, entitystore::EntityStore, util::SaturnEUid};

    use crate::api::MonoApiServiceState;

    pub async fn get_entitystore(path: PathBuf, state: State<MonoApiServiceState>) -> EntityStore {
        let mut entities: EntityStore = EntityStore::new();
        for component in path.ancestors() {
            if component != std::path::Path::new("/") {
                let cedar_path = component.join(".mega_cedar.json");
                let entity_str = state
                    .monorepo()
                    .get_blob_as_string(cedar_path, None)
                    .await
                    .unwrap();
                if let Some(entity_str) = entity_str {
                    entities.merge(serde_json::from_str(&entity_str).unwrap());
                }
            }
        }
        entities
    }

    pub async fn check_permissions(
        username: &str,
        path: &str,
        operation: ActionEnum,
        state: State<MonoApiServiceState>,
    ) -> Result<(), saturn::context::SaturnContextError> {
        let entities = get_entitystore(path.into(), state).await;
        let cedar_context = CedarContext::new(entities).unwrap();
        cedar_context.is_authorized(
            format!(r#"User::"{username}""#)
                .to_owned()
                .parse::<SaturnEUid>()
                .unwrap(),
            format!(r#"Action::"{operation}""#)
                .parse::<SaturnEUid>()
                .unwrap(),
            format!(r#"Repository::"{path}""#)
                .parse::<SaturnEUid>()
                .unwrap(),
            Context::empty(),
        )
    }
}
