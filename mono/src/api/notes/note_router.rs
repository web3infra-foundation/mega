use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::note::NoteUpdateRequest;
use serde_json::Value;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, api_doc::SYNC_NOTES_STATE_TAG, error::ApiError};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/organizations",
        OpenApiRouter::new()
            .routes(routes!(show_note))
            .routes(routes!(update_note)),
    )
}

#[utoipa::path(
    get,
    path = "/{org_slug}/notes/{id}/sync_state",
    responses(
        (status = 200, body = ceres::model::note::NoteShowResponse, content_type = "application/json")
    ),
    tag = SYNC_NOTES_STATE_TAG,
)]
async fn show_note(
    state: State<MonoApiServiceState>,
    Path(id): Path<i32>,
) -> Result<Json<ceres::model::note::NoteShowResponse>, ApiError> {
    // TODO: authorize(note, :show?)
    let response = state.monorepo().get_note_sync_state(id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    patch,
    path = "/{org_slug}/notes/{id}/sync_state",
    request_body = NoteUpdateRequest,
    responses(
        (status = 200, body = Value, content_type = "application/json")
    ),
    tag = SYNC_NOTES_STATE_TAG,
)]
async fn update_note(
    state: State<MonoApiServiceState>,
    Path(id): Path<i32>,
    Json(json): Json<NoteUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    // TODO: authorize note access (like in show_note)
    state
        .monorepo()
        .update_note_sync_state(
            id,
            json.description_html.as_str(),
            json.description_state.as_str(),
            json.description_schema_version,
        )
        .await?;

    Ok(Json(serde_json::json!({})))
}
