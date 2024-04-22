use std::{env, path::PathBuf, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use ganymede::model::create_file::CreateFileInfo;
use jupiter::storage::{git_db_storage::GitDbStorage, mega_storage::MegaStorage};
use venus::monorepo::mr::{MergeOperation, MergeResult};

use crate::api_service::ApiHandler;
use crate::{api_service::mono_service::MonorepoService, model::objects::LatestCommitInfo};
use crate::{api_service::obj_service::ObjectService, model::query::CodePreviewQuery};

#[derive(Clone)]
pub struct ApiServiceState {
    pub mega_storage: Arc<MegaStorage>,
    pub git_db_storage: Arc<GitDbStorage>,
}

impl ApiServiceState {
    pub fn monorepo(&self) -> MonorepoService {
        MonorepoService {
            storage: self.mega_storage.clone(),
        }
    }

    pub fn api_handler(&self, path: PathBuf) -> Box<dyn ApiHandler> {
        let import_dir = PathBuf::from(env::var("MEGA_IMPORT_DIRS").unwrap());
        if path.starts_with(import_dir.clone()) && path != import_dir {
            Box::new(ObjectService {
                storage: self.git_db_storage.clone(),
            })
        } else {
            Box::new(MonorepoService {
                storage: self.mega_storage.clone(),
            })
        }
    }
}

pub fn routers() -> Router<ApiServiceState> {
    let router_v1 = Router::new()
        // .route("/blob", get(get_blob_object))
        // .route("/tree", get(get_directories))
        // .route("/object", get(get_origin_object))
        .route("/status", get(life_cycle_check))
        // .route("/count-objs", get(get_count_nums))
        .route("/init", get(init))
        .route("/create-file", post(create_file))
        .route("/merge", post(merge));

    let preview_code = Router::new().route("/latest-commit", get(get_latest_commit))
    .route("/tree-commit-info", get(get_latest_commit))
    .route("/tree", get(get_latest_commit));

    Router::new().merge(router_v1).merge(preview_code)
}

// async fn get_blob_object(
//     Query(query): Query<HashMap<String, String>>,
//     state: State<ApiServiceState>,
// ) -> Result<Json<BlobObjects>, (StatusCode, String)> {
//     let object_id = query.get("object_id").unwrap();
//     state.object_service.get_blob_objects(object_id).await
// }

// async fn get_directories(
//     Query(query): Query<DirectoryQuery>,
//     state: State<ApiServiceState>,
// ) -> Result<Json<Directories>, (StatusCode, String)> {
//     state.object_service.get_directories(query).await
// }

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
        .create_mega_file(json.clone())
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
        .api_handler(query.path.into())
        .get_latest_commit()
        .await
        .unwrap();
    Ok(Json(res))
}
