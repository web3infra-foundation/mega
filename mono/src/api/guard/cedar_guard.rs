use std::{path::Path, str::FromStr};

use axum::{
    extract::{FromRef, Request, State},
    middleware::Next,
    response::Response,
};
use cedar_policy::{Context, EntityId, EntityTypeName, EntityUid};
use common::errors::MegaError;
use http::StatusCode;
use saturn::{ActionEnum, util::SaturnEUid};
use saturn::{context::CedarContext, entitystore::EntityStore};
use std::collections::HashMap;

use crate::api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser};

// TODO: All users are temporary allowed during development stage
const CL_PATH_PREFIX: &str = "/cl";
const POLICY_CONTENT: &str = r#"
permit (
    principal,  
    action,
    resource
);"#;

pub async fn resolve_cl_action(req_path: &str) -> Result<(ActionEnum, String), MegaError> {
    let endpoints_config_dict = include_str!("guarded_endpoints.json");

    // Avoid parsing request of non-CL endpoints
    if !req_path.starts_with(CL_PATH_PREFIX) {
        return Ok((ActionEnum::UnprotectedRequest, String::new()));
    }
    let path = req_path.trim_start_matches(CL_PATH_PREFIX);

    let api_config: HashMap<String, HashMap<String, String>> =
        serde_json::from_str(endpoints_config_dict).map_err(|e| {
            MegaError::Other(format!("Failed to parse guarded_endpoints.json: {}", e))
        })?;

    let cl_config = api_config.get(CL_PATH_PREFIX).ok_or_else(|| {
        MegaError::Other("No CL config found in guarded_endpoints.json".to_string())
    })?;

    let Some((action, mr_link)) = match_operation(path, cl_config) else {
        tracing::warn!("No matching CL action for path: {}", req_path);
        return Ok((ActionEnum::UnprotectedRequest, String::new()));
    };

    Ok((action, mr_link))
}

//TODO: Only match cl api paths for now, extend when need in the future
/// return (ActionEnum, mr_link)
fn match_operation(
    suffix: &str,
    patterns: &HashMap<String, String>,
) -> Option<(ActionEnum, String)> {
    for (pattern, action) in patterns {
        let op = pattern.trim_start_matches("/{link}/");
        if suffix.ends_with(op) {
            println!("op matched: {}", op);
            println!("suffix is : {}", suffix);
            let mr_link = suffix
                .trim_end_matches(op)
                .trim_end_matches('/')
                .trim_start_matches('/')
                .to_string();
            return Some((ActionEnum::from(action.to_string()), mr_link));
        }
    }
    None
}

pub async fn cedar_guard(
    State(state): State<MonoApiServiceState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let request_path = req.uri().path().to_owned();
    tracing::debug!("Processing request: {}", request_path);

    let (action, link) = resolve_cl_action(&request_path).await.map_err(|e| {
        tracing::error!("Failed to resolve CL action: {}", e);
        ApiError::with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            MegaError::Other("Failed to resolve CL action".to_string()),
        )
    })?;
    tracing::debug!("Resolved action: {:?}, link: {}", action, link);

    // Skip authorization for unprotected requests
    if action.eq(&ActionEnum::UnprotectedRequest) {
        tracing::debug!("Unprotected request for path: {}", request_path);
        return Ok(next.run(req).await);
    }

    // TODO: Fetch repo path from CL model
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
                MegaError::Other(format!("Guard Authorization failed: {}", e)),
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
        .map_err(|e| MegaError::Other(format!("Authorization failed: {}", e)))?;

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
            Err(MegaError::Other(format!(
                "Blob not found at path: {}",
                path.display()
            )))
        }?,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_match_operation() {
        let patterns: HashMap<String, String> = HashMap::from([
            (
                "/{link}/approve".to_string(),
                "approveMergeRequest".to_string(),
            ),
            ("/{link}/close".to_string(), "editMergeRequest".to_string()),
        ]);

        let suffix = "/my-cl-link/approve";
        let result = match_operation(suffix, &patterns);
        assert_eq!(
            result,
            Some((ActionEnum::ApproveMergeRequest, "my-cl-link".to_string()))
        );

        let suffix = "/another-cl-link/close";
        let result = match_operation(suffix, &patterns);
        assert_eq!(
            result,
            Some((ActionEnum::EditMergeRequest, "another-cl-link".to_string()))
        );

        let suffix = "/no-match-link/delete";
        let result = match_operation(suffix, &patterns);
        assert_eq!(result, None);
    }
}
