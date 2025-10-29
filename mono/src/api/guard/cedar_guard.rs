use std::{path::Path, str::FromStr};

use axum::{extract::{FromRef, Request, State}, middleware::{Next}};
use cedar_policy::{Context, EntityId, EntityTypeName, EntityUid};
use common::errors::MegaError;
use saturn::{context::CedarContext, entitystore::EntityStore};
use once_cell::sync::Lazy;
use saturn::ActionEnum;
use serde_json::json;
use std::collections::HashMap;

use crate::api::{error::ApiError, oauth::model::LoginUser, MonoApiServiceState};



/// Maps mutating CL router handlers to the Saturn action they require.
pub static CL_ROUTER_ACTIONS: Lazy<HashMap<&'static str, ActionEnum>> = Lazy::new(|| {
    HashMap::from([
        ("reopen_cl", ActionEnum::EditMergeRequest),
        ("close_cl", ActionEnum::EditMergeRequest),
        ("merge", ActionEnum::ApproveMergeRequest),
        ("save_comment", ActionEnum::EditMergeRequest),
        ("edit_title", ActionEnum::EditMergeRequest),
        ("labels", ActionEnum::EditMergeRequest),
        ("assignees", ActionEnum::EditMergeRequest),
        ("add_reviewers", ActionEnum::EditMergeRequest),
        ("remove_reviewers", ActionEnum::EditMergeRequest),
        ("reviewer_approve", ActionEnum::ApproveMergeRequest),
        ("review_resolve", ActionEnum::EditMergeRequest),
    ])
});



pub async fn cedar_guard(
    State(state): State<MonoApiServiceState>,
    req: Request,
    _next: Next,
) -> Result<(), ApiError> {
    let method = req.uri().path();
    let action = CL_ROUTER_ACTIONS.get(method).ok_or_else(|| {
        MegaError::with_message(format!("No action mapping found for method: {}", method))
    })?;
    
    let user = req
        .extensions()
        .get::<LoginUser>()
        .cloned() 
        .ok_or_else(|| {
            MegaError::with_message("Missing LoginUser injection")
        })?;

    let mr_path = req.uri().path().to_string();
    let schema_path = Path::new(&mr_path).join("cedar/.cedarschema");
    let policy_path = Path::new(&mr_path).join("cedar/policies.cedar");

    let schema_content= get_blob_string(&state, &schema_path).await?;
    let policy_content= get_blob_string(&state, &policy_path).await?;
    let entity_store = EntityStore::from_ref(&state);
    let c = CedarContext::from(entity_store, &schema_content, &policy_content)?;

    authorize(&c, &user.campsite_user_id, &action.to_string()).await?;
    Ok(())
}

async fn authorize(
    cedar_context: &CedarContext,
    user_id: &str,
    action: &str,
) -> Result <(), ApiError> {
    let user_entity = EntityId::from_str(user_id)?;
    let action_entity = EntityId::from_str(action)?; 
    
    let role_entity = EntityTypeName::from_str("User")?;    
    let actiontype_entity = EntityTypeName::from_str("Action")?; 

    let principal = saturn::util::EntityUid::from(
        EntityUid::from_type_name_and_id(role_entity,user_entity)
    );
    let action = saturn::util::EntityUid::from(
        EntityUid::from_type_name_and_id(actiontype_entity, action_entity)
    ); 
    let resource = saturn::util::EntityUid::from(
        EntityUid::from_type_name_and_id(
            EntityTypeName::from_str("Repository")?,
            EntityId::from_str("0")?,
        ));

    let context = Context::from_json_value(json!({
        "repo_id": "core",
        "user_role": "maintainer"
    }), None)?;

    cedar_context.is_authorized(
        &principal,
        &action,
        &resource,
        context,
    ).map_err(|e| MegaError::with_message(format!("Authorization failed: {}", e)))?;

    Ok(())
}

async fn get_blob_string(state: &MonoApiServiceState, path: &Path) -> Result<String, ApiError> {
    // Use main as default branch
    let refs = None;
    let data = state
        .api_handler(path.as_ref())
        .await?
        .get_blob_as_string(path.into(), refs)
        .await?;

    match data {
        Some(content) => {
            Ok(content)
        }
        None => {
            Err(MegaError::with_message(format!(
                "Blob not found at path: {}",
                path.display()
            )))
        }?
    }
}