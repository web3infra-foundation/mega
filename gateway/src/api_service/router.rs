use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use git::internal::pack::counter::GitTypeCounter;

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
}

pub fn routers<S>(state: ApiServiceState) -> Router<S> {
    Router::new()
        .route("/blob", get(get_blob_object))
        .route("/tree", get(get_directories))
        .route("/object", get(get_origin_object))
        .route("/status", get(life_cycle_check))
        .route("/count-nums", get(get_count_nums))
        .with_state(state)
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
    state.object_service.get_objects_data(object_id).await
}
