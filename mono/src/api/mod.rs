use std::path::PathBuf;

use ceres::{
    api_service::{
        import_api_service::ImportApiService, mono_api_service::MonoApiService, ApiHandler,
    },
    protocol::repo::Repo,
};
use common::model::CommonOptions;
use jupiter::context::Context;

pub mod api_router;
pub mod mr_router;
pub mod oauth;

#[derive(Clone)]
pub struct MonoApiServiceState {
    pub context: Context,
    pub common: CommonOptions,
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
