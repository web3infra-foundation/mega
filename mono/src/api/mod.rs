use async_session::MemoryStore;
use axum::extract::FromRef;
use oauth2::{
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenResponse,
    },
    Client, EndpointNotSet, EndpointSet, StandardRevocableToken,
};
use std::path::Path;

use crate::api::oauth::campsite_store::CampsiteApiStore;
use ceres::{
    api_service::{
        import_api_service::ImportApiService, mono_api_service::MonoApiService, ApiHandler,
    },
    protocol::repo::Repo,
};
use common::errors::ProtocolError;
use jupiter::storage::note_storage::NoteStorage;
use jupiter::storage::{
    conversation_storage::ConversationStorage, issue_storage::IssueStorage, mr_storage::MrStorage,
    user_storage::UserStorage, Storage,
};

pub mod api_common;
pub mod api_router;
pub mod conversation;
pub mod error;
pub mod issue;
pub mod label;
pub mod lfs;
pub mod mr;
pub mod notes;
pub mod oauth;
pub mod user;

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
    pub oauth_client: Option<GithubClient>,
    pub session_store: Option<CampsiteApiStore>,
    pub listen_addr: String,
}

impl FromRef<MonoApiServiceState> for MemoryStore {
    fn from_ref(_: &MonoApiServiceState) -> Self {
        MemoryStore::new()
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

impl MonoApiServiceState {
    fn monorepo(&self) -> MonoApiService {
        MonoApiService {
            storage: self.storage.clone(),
        }
    }

    fn issue_stg(&self) -> IssueStorage {
        self.storage.issue_storage()
    }

    fn mr_stg(&self) -> MrStorage {
        self.storage.mr_storage()
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

    async fn api_handler(&self, path: &Path) -> Result<Box<dyn ApiHandler>, ProtocolError> {
        let import_dir = self.storage.config().monorepo.import_dir.clone();
        if path.starts_with(&import_dir) && path != import_dir {
            if let Some(model) = self
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
                }));
            }
        }
        let ret: Box<dyn ApiHandler> = Box::new(MonoApiService {
            storage: self.storage.clone(),
        });

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
    use saturn::{context::CedarContext, entitystore::EntityStore, util::EntityUid, ActionEnum};

    use crate::api::MonoApiServiceState;

    pub async fn get_entitystore(path: PathBuf, state: State<MonoApiServiceState>) -> EntityStore {
        let mut entities: EntityStore = EntityStore::new();
        for component in path.ancestors() {
            if component != std::path::Path::new("/") {
                let cedar_path = component.join(".mega_cedar.json");
                let entity_str = state
                    .monorepo()
                    .get_blob_as_string(cedar_path)
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
    ) -> Result<(), saturn::context::Error> {
        let entities = get_entitystore(path.into(), state).await;
        let cedar_context = CedarContext::new(entities).unwrap();
        cedar_context.is_authorized(
            format!(r#"User::"{username}""#)
                .to_owned()
                .parse::<EntityUid>()
                .unwrap(),
            format!(r#"Action::"{operation}""#)
                .parse::<EntityUid>()
                .unwrap(),
            format!(r#"Repository::"{path}""#)
                .parse::<EntityUid>()
                .unwrap(),
            Context::empty(),
        )
    }
}
