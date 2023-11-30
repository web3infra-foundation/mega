use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::{
    api_service::obj_service::ObjectService,
    model::{
        object_detail::{BlobObjects, Directories},
        query::DirectoryQuery,
    },
};

use crate::AppState;

pub fn routers<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/blob", get(get_blob_object))
        .route("/tree", get(get_directories))
        .route("/object", get(get_origin_object))
        .with_state(state)
}

async fn get_blob_object(
    Query(query): Query<HashMap<String, String>>,
    state: State<AppState>,
) -> Result<Json<BlobObjects>, (StatusCode, String)> {
    let object_id = query.get("object_id").unwrap();
    let object_service = ObjectService {
        storage: state.storage.clone(),
    };
    object_service.get_blob_objects(object_id).await
}

async fn get_directories(
    Query(query): Query<DirectoryQuery>,
    state: State<AppState>,
) -> Result<Json<Directories>, (StatusCode, String)> {
    let object_service = ObjectService {
        storage: state.storage.clone(),
    };
    object_service.get_directories(query).await
}

async fn get_origin_object(
    Query(query): Query<HashMap<String, String>>,
    state: State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let object_id = query.get("object_id").unwrap();
    let object_service = ObjectService {
        storage: state.storage.clone(),
    };
    object_service.get_objects_data(object_id).await
}
