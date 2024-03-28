use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use ganymede::model::create_file::CreateFileInfo;
use git::internal::pack::counter::GitTypeCounter;
use ceres::monorepo::service::MonorepoService;
use venus::monorepo::mr::{MergeOperation, MergeResult};

use crate::{
    api_service::obj_service::ObjectService,
    model::{
        objects::{BlobObjects, Directories},
        query::DirectoryQuery,
    },
};

#[derive(Clone)]
pub struct ApiServiceState {
    pub object_service: ObjectService,
    pub monorepo_service: MonorepoService,
}

pub fn routers() -> Router<ApiServiceState> {
    Router::new()
        .route("/blob", get(get_blob_object))
        .route("/tree", get(get_directories))
        .route("/object", get(get_origin_object))
        .route("/status", get(life_cycle_check))
        .route("/count-objs", get(get_count_nums))
        .route("/init", get(init))
        .route("/create_file", post(create_file))
        .route("/merge", post(merge))
}

async fn get_blob_object(
    Query(query): Query<HashMap<String, String>>,
    state: State<ApiServiceState>,
) -> Result<Json<BlobObjects>, (StatusCode, String)> {
    let object_id = query.get("object_id").unwrap();
    state.object_service.get_blob_objects(object_id).await
}

async fn get_directories(
    Query(query): Query<DirectoryQuery>,
    state: State<ApiServiceState>,
) -> Result<Json<Directories>, (StatusCode, String)> {
    state.object_service.get_directories(query).await
}

async fn get_origin_object(
    Query(query): Query<HashMap<String, String>>,
    state: State<ApiServiceState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let object_id = query.get("object_id").unwrap();
    let repo_path = query.get("repo_path").expect("repo_path is required");
    state
        .object_service
        .get_objects_data(object_id, repo_path)
        .await
}

async fn life_cycle_check() -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json("http ready"))
}

async fn get_count_nums(
    Query(query): Query<HashMap<String, String>>,
    state: State<ApiServiceState>,
) -> Result<Json<GitTypeCounter>, (StatusCode, String)> {
    let repo_path = query.get("repo_path").unwrap();
    state.object_service.count_object_num(repo_path).await
}

async fn init(state: State<ApiServiceState>) {
    state.monorepo_service.init_monorepo().await
}

async fn create_file(
    state: State<ApiServiceState>,
    Json(json): Json<CreateFileInfo>,
) -> Result<Json<CreateFileInfo>, (StatusCode, String)> {
    state
        .monorepo_service
        .create_mega_file(json.clone())
        .await
        .unwrap();
    Ok(Json(json))
}

async fn merge(
    state: State<ApiServiceState>,
    Json(json): Json<MergeOperation>,
) -> Result<Json<MergeResult>, (StatusCode, String)> {
    let res = state.monorepo_service.merge_mr(json.clone()).await.unwrap();
    Ok(Json(res))
}
