use std::path::PathBuf;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use jupiter::context::Context;
use venus::{
    import_repo::repo::Repo,
    monorepo::mr::{MergeOperation, MergeResult},
};

use crate::{
    api_service::import_service::ImportRepoService,
    model::{
        create_file::CreateFileInfo,
        query::{BlobContentQuery, CodePreviewQuery},
    },
};
use crate::{
    api_service::mono_service::MonorepoService,
    model::objects::{BlobObjects, LatestCommitInfo, TreeCommitInfo},
};
use crate::{api_service::ApiHandler, model::objects::TreeBriefInfo};

#[derive(Clone)]
pub struct ApiServiceState {
    pub context: Context,
}

impl ApiServiceState {
    pub fn monorepo(&self) -> MonorepoService {
        MonorepoService {
            storage: self.context.services.mega_storage.clone(),
        }
    }

    pub async fn api_handler(&self, path: PathBuf) -> Box<dyn ApiHandler> {
        let import_dir = self.context.config.monorepo.import_dir.clone();
        if path.starts_with(&import_dir) && path != import_dir {
            let repo: Repo = self
                .context
                .services
                .git_db_storage
                .find_git_repo(path.to_str().unwrap())
                .await
                .unwrap()
                .unwrap()
                .into();

            Box::new(ImportRepoService {
                storage: self.context.services.git_db_storage.clone(),
                repo,
            })
        } else {
            Box::new(MonorepoService {
                storage: self.context.services.mega_storage.clone(),
            })
        }
    }
}

pub fn routers() -> Router<ApiServiceState> {
    let router_v1 = Router::new()
        .route("/blob", get(get_blob_object))
        .route("/status", get(life_cycle_check))
        .route("/init", get(init))
        .route("/create-file", post(create_file))
        .route("/merge", post(merge));

    let preview_code = Router::new()
        .route("/latest-commit", get(get_latest_commit))
        .route("/tree-commit-info", get(get_tree_commit_info))
        .route("/tree", get(get_tree_info));

    Router::new().merge(router_v1).merge(preview_code)
}

async fn get_blob_object(
    Query(query): Query<BlobContentQuery>,
    state: State<ApiServiceState>,
) -> Result<Json<BlobObjects>, (StatusCode, String)> {
    let res = state
        .monorepo()
        .get_blob_as_string(query.path.into(), &query.name)
        .await
        .unwrap();
    Ok(Json(res))
}

// async fn get_origin_object(
//     Query(query): Query<HashMap<String, String>>,
//     state: State<ApiServiceState>,
// ) -> Result<impl IntoResponse, (StatusCode, String)> {
//     let object_id = query.get("object_id").unwrap();
//     let repo_path = query.get("repo_path").expect("repo_path is required");
//     state
//         .object_service
//         .get_objects_data(object_id, repo_path)
//         .await
// }

async fn life_cycle_check() -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json("http ready"))
}

// async fn get_count_nums(
//     Query(query): Query<HashMap<String, String>>,
//     state: State<ApiServiceState>,
// ) -> Result<Json<GitTypeCounter>, (StatusCode, String)> {
//     let repo_path = query.get("repo_path").unwrap();
//     state.object_service.count_object_num(repo_path).await
// }

async fn init(state: State<ApiServiceState>) {
    state.monorepo().init_monorepo().await
}

async fn create_file(
    state: State<ApiServiceState>,
    Json(json): Json<CreateFileInfo>,
) -> Result<Json<CreateFileInfo>, (StatusCode, String)> {
    state
        .monorepo()
        .create_monorepo_file(json.clone())
        .await
        .unwrap();
    Ok(Json(json))
}

async fn merge(
    state: State<ApiServiceState>,
    Json(json): Json<MergeOperation>,
) -> Result<Json<MergeResult>, (StatusCode, String)> {
    let res = state.monorepo().merge_mr(json.clone()).await.unwrap();
    Ok(Json(res))
}

async fn get_latest_commit(
    Query(query): Query<CodePreviewQuery>,
    state: State<ApiServiceState>,
) -> Result<Json<LatestCommitInfo>, (StatusCode, String)> {
    let res = state
        .api_handler(query.path.clone().into())
        .await
        .get_latest_commit(query.path.into())
        .await
        .unwrap();
    Ok(Json(res))
}

async fn get_tree_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<ApiServiceState>,
) -> Result<Json<TreeBriefInfo>, (StatusCode, String)> {
    let res = state
        .api_handler(query.path.clone().into())
        .await
        .get_tree_info(query.path.into())
        .await
        .unwrap();
    Ok(Json(res))
}

async fn get_tree_commit_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<ApiServiceState>,
) -> Result<Json<TreeCommitInfo>, (StatusCode, String)> {
    let res = state
        .api_handler(query.path.clone().into())
        .await
        .get_tree_commit_info(query.path.into())
        .await
        .unwrap();
    Ok(Json(res))
}
