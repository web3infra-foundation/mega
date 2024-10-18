use std::path::PathBuf;

use async_session::MemoryStore;
use axum::extract::FromRef;
use oauth2::basic::BasicClient;

use ceres::{
    api_service::{
        import_api_service::ImportApiService, mono_api_service::MonoApiService, ApiHandler,
    },
    protocol::repo::Repo,
};
use common::{errors::ProtocolError, model::CommonOptions};
use jupiter::{context::Context, storage::user_storage::UserStorage};

pub mod api_router;
pub mod error;
pub mod lfs;
pub mod model;
pub mod mr_router;
pub mod oauth;
pub mod user;

#[derive(Clone)]
pub struct MonoApiServiceState {
    pub context: Context,
    pub common: CommonOptions,
    pub oauth_client: Option<BasicClient>,
    // TODO: Remove MemoryStore
    pub store: Option<MemoryStore>,
}

impl FromRef<MonoApiServiceState> for MemoryStore {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.store.clone().unwrap()
    }
}

impl FromRef<MonoApiServiceState> for BasicClient {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.oauth_client.clone().unwrap()
    }
}

impl FromRef<MonoApiServiceState> for UserStorage {
    fn from_ref(state: &MonoApiServiceState) -> Self {
        state.context.services.user_storage.clone()
    }
}

impl MonoApiServiceState {
    fn monorepo(&self) -> MonoApiService {
        MonoApiService {
            context: self.context.clone(),
        }
    }

    async fn api_handler(&self, path: PathBuf) -> Result<Box<dyn ApiHandler>, ProtocolError> {
        let import_dir = self.context.config.monorepo.import_dir.clone();
        if path.starts_with(&import_dir) && path != import_dir {
            if let Some(model) = self
                .context
                .services
                .git_db_storage
                .find_git_repo_like_path(path.to_str().unwrap())
                .await
                .unwrap()
            {
                let repo: Repo = model.into();
                return Ok(Box::new(ImportApiService {
                    context: self.context.clone(),
                    repo,
                }));
            }
        }
        Ok(Box::new(MonoApiService {
            context: self.context.clone(),
        }))
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
        let crate_root = env!("CARGO_MANIFEST_DIR");
        let cedar_context = CedarContext::new(
            entities,
            format!("{}/../saturn/mega.cedarschema", crate_root),
            format!("{}/../saturn/mega_policies.cedar", crate_root),
        )
        .unwrap();
        cedar_context.is_authorized(
            format!(r#"User::"{}""#, username)
                .to_owned()
                .parse::<EntityUid>()
                .unwrap(),
            format!(r#"Action::"{}""#, operation)
                .parse::<EntityUid>()
                .unwrap(),
            format!(r#"Repository::"{}""#, path)
                .parse::<EntityUid>()
                .unwrap(),
            Context::empty(),
        )
    }
}
