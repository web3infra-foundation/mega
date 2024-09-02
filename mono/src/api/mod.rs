use std::path::PathBuf;

use async_session::MemoryStore;
use axum::extract::FromRef;
use ceres::{
    api_service::{
        import_api_service::ImportApiService, mono_api_service::MonoApiService, ApiHandler,
    },
    protocol::repo::Repo,
};
use common::model::CommonOptions;
use jupiter::{context::Context, storage::user_storage::UserStorage};
use oauth2::basic::BasicClient;

pub mod api_router;
pub mod mr_router;
pub mod oauth;
pub mod user;
pub mod lfs;

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

    async fn api_handler(&self, path: PathBuf) -> Box<dyn ApiHandler> {
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
                return Box::new(ImportApiService {
                    context: self.context.clone(),
                    repo,
                });
            }
        }
        Box::new(MonoApiService {
            context: self.context.clone(),
        })
    }
}
