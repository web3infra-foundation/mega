use std::path::PathBuf;

use ceres::api_service::{
    import_api_service::ImportApiService, mono_api_service::MonoApiService, ApiHandler,
};
use jupiter::context::Context;
use venus::import_repo::repo::Repo;

pub mod api_router;
pub mod mr_router;

#[derive(Clone)]
pub struct ApiServiceState {
    pub context: Context,
}

impl ApiServiceState {
    pub fn monorepo(&self) -> MonoApiService {
        MonoApiService {
            context: self.context.clone(),
        }
    }

    pub async fn api_handler(&self, path: PathBuf) -> Box<dyn ApiHandler> {
        let import_dir = self.context.config.monorepo.import_dir.clone();
        if path.starts_with(&import_dir) && path != import_dir {
            if let Some(model) = self
                .context
                .services
                .git_db_storage
                .find_git_repo(path.to_str().unwrap())
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
