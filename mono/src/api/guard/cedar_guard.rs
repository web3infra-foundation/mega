use std::{path::Path, str::FromStr};

use axum::{
    extract::{FromRef, Request, State},
    middleware::Next,
    response::Response,
};
use cedar_policy::{Context, EntityId, EntityTypeName, EntityUid};
use common::errors::MegaError;
use http::StatusCode;
use once_cell::sync::Lazy;
use saturn::{ActionEnum, util::SaturnEUid};
use saturn::{context::CedarContext, entitystore::EntityStore};
use std::collections::HashMap;

use crate::api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser};

// TODO: All users are temporary allowed during development stage
const POLICY_CONTENT: &str = r#"
permit (
    principal,  
    action,
    resource
);"#;

/// Maps mutating CL router handlers to the Saturn action they require.
pub static CL_ROUTER_ACTIONS: Lazy<HashMap<&'static str, ActionEnum>> = Lazy::new(|| {
    HashMap::from([
        ("reopen", ActionEnum::EditMergeRequest),
        ("close", ActionEnum::EditMergeRequest),
        ("merge", ActionEnum::ApproveMergeRequest),
        ("comment", ActionEnum::EditMergeRequest),
        ("title", ActionEnum::EditMergeRequest),
        ("labels", ActionEnum::EditMergeRequest),
        ("assignees", ActionEnum::EditMergeRequest),
        ("reviewers", ActionEnum::EditMergeRequest),
        ("approve", ActionEnum::ApproveMergeRequest),
        ("resolve", ActionEnum::EditMergeRequest),
        ("status", ActionEnum::EditMergeRequest),
    ])
});

pub async fn cedar_guard(
    State(state): State<MonoApiServiceState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let request_path = req.uri().path().trim_end_matches('/').to_owned();
    let mut segments = request_path.rsplitn(3, '/');
    tracing::debug!(?segments, "route segments");

    let method = segments.next().unwrap_or_default();
    //TODO: use link to get repository path
    // let _link = match segments.next() {
    //     Some(segment) if !segment.is_empty() => segment.to_string(),
    //     _ => {
    //         return Err(MegaError::with_message(format!(
    //             "Unable to extract change list link from path: {}",
    //             request_path
    //         ))
    //         .into());
    //     }
    // };

    let action = match CL_ROUTER_ACTIONS.get(method) {
        Some(a) => {
            tracing::debug!("Cedar guard action: {:?}", method);
            a
        }
        None => {
            tracing::warn!("Unknown method '{}', skipping Cedar guard.", method);
            return Ok(next.run(req).await);
        }
    };

    // TODO: use link to get repository path
    // let cl_model = state
    //     .cl_stg()
    //     .get_cl(&link)
    //     .await?
    //     .ok_or_else(|| MegaError::with_message(format!("Change list not found for link: {}", link)))?;
    // let repo_path: PathBuf = cl_model.path.into();

    let user = req.extensions().get::<LoginUser>().cloned();

    let _username = match user {
        Some(user) => user.username,
        None => "reader".to_string(),
    };
    let username = "reader".to_string(); // For testing purpose only

    // let policy_path = repo_path.join("cedar/policies.cedar");
    // let policy_content = get_blob_string(&state, &policy_path).await?;
    let policy_content = POLICY_CONTENT.to_string();

    let entity_store = EntityStore::from_ref(&state);
    let c = CedarContext::from(entity_store, &policy_content)?;

    authorize(&c, &username, &action.to_string())
        .await
        .map_err(|e| {
            tracing::debug!(
                "Authorization failed for {}: {}",
                &username,
                &action.to_string()
            );
            ApiError::with_status(
                StatusCode::UNAUTHORIZED,
                MegaError::with_message(format!("Guard Authorization failed: {}", e)),
            )
        })?;
    let response = next.run(req).await;

    if response.status().is_client_error() {
        tracing::error!(
            status = %response.status(),
            path = %request_path,
            "Downstream returned a 4xx error"
        );
    }

    Ok(response)
}

async fn authorize(
    cedar_context: &CedarContext,
    user_id: &str,
    // repo_id: &str,
    action: &str,
) -> Result<(), MegaError> {
    let user_entity = EntityId::from_str(user_id)?;
    let action_entity = EntityId::from_str(action)?;

    let role_entity = EntityTypeName::from_str("User")?;
    let actiontype_entity = EntityTypeName::from_str("Action")?;

    let principal = SaturnEUid::from(EntityUid::from_type_name_and_id(role_entity, user_entity));
    let action = SaturnEUid::from(EntityUid::from_type_name_and_id(
        actiontype_entity,
        action_entity,
    ));
    // TODO: repository is currently hardcoded to "0", need to change it to actual repo id
    let resource = SaturnEUid::from(EntityUid::from_type_name_and_id(
        EntityTypeName::from_str("Repository")?,
        EntityId::from_str("0")?,
    ));

    let context = Context::empty();

    cedar_context
        .is_authorized(&principal, &action, &resource, context)
        .map_err(|e| MegaError::with_message(format!("Authorization failed: {}", e)))?;

    Ok(())
}

#[allow(dead_code)]
async fn get_blob_string(state: &MonoApiServiceState, path: &Path) -> Result<String, ApiError> {
    // Use main as default branch
    let refs = None;
    let data = state
        .api_handler(path.as_ref())
        .await?
        .get_blob_as_string(path.into(), refs)
        .await?;

    match data {
        Some(content) => Ok(content),
        None => {
            Err(MegaError::with_message(format!(
                "Blob not found at path: {}",
                path.display()
            )))
        }?,
    }
}
