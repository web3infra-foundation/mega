use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::Value;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{
        MonoApiServiceState,
        error::ApiError,
        notes::model::{ShowResponse, UpdateRequest},
    },
    server::http_server::SYNC_NOTES_STATE_TAG,
};

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
        (status = 200, body = ShowResponse, content_type = "application/json")
    ),
    tag = SYNC_NOTES_STATE_TAG,
)]
async fn show_note(
    state: State<MonoApiServiceState>,
    Path(id): Path<i32>,
) -> Result<Json<ShowResponse>, ApiError> {
    let note = state.note_stg().get_note_by_id(id.into()).await?;
    if note.is_none() {
        return Err(ApiError::from(anyhow::anyhow!("Note not found")));
    }
    let note = note.unwrap();

    // TODO: authorize(note, :show?)

    let response = ShowResponse {
        public_id: note.public_id,
        description_schema_version: note.description_schema_version,
        description_state: match &note.description_state {
            Some(state) if !state.is_empty() => Some(state.clone()),
            _ => None,
        },
        description_html: match &note.description_html {
            Some(html) if !html.is_empty() => html.clone(),
            _ => String::new(),
        },
    };
    Ok(Json(response))
}

#[utoipa::path(
    patch,
    path = "/{org_slug}/notes/{id}/sync_state",
    request_body = UpdateRequest,
    responses(
        (status = 200, body = Value, content_type = "application/json")
    ),
    tag = SYNC_NOTES_STATE_TAG,
)]
async fn update_note(
    state: State<MonoApiServiceState>,
    Path(id): Path<i32>,
    Json(json): Json<UpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    // Get the note first
    let note = state.note_stg().get_note_by_id(id.into()).await?;
    if note.is_none() {
        return Err(ApiError::from(anyhow::anyhow!(format!(
            "Note with ID {} not found",
            id
        ))));
    }
    let note = note.unwrap();

    // TODO: authorize note access (like in show_note)

    // Check schema version compatibility
    if json.description_schema_version < note.description_schema_version {
        return Err(ApiError::from(anyhow::anyhow!(
            "Invalid schema version: provided ({}) is older than current ({})",
            json.description_schema_version,
            note.description_schema_version
        )));
    }

    // Update the note
    let _res_id = state
        .note_stg()
        .update_note(
            id,
            json.description_html.as_str(),
            json.description_state.as_str(),
            json.description_schema_version,
        )
        .await?;

    Ok(Json(serde_json::json!({})))
}
