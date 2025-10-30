use cedar_policy::{
    Authorizer, CedarSchemaError, Context, Decision, Diagnostics, ParseErrors, PolicySet,
    PolicySetError, Request, Schema, SchemaError, ValidationMode, Validator,
};
use itertools::Itertools;
use thiserror::Error;

use crate::{entitystore::EntityStore, util::EntityUid};

pub struct CedarContext {
    pub entities: EntityStore,
    authorizer: Authorizer,
    policies: PolicySet,
    schema: Schema,
}

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ContextError {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("Error Parsing Json Schema: {0}")]
    JsonSchema(#[from] SchemaError),
    #[error("Error Parsing Human-readable Schema: {0}")]
    CedarSchema(#[from] CedarSchemaError),
    #[error("Error Parsing PolicySet: {0}")]
    Policy(#[from] ParseErrors),
    #[error("Error Processing PolicySet: {0}")]
    PolicySet(#[from] PolicySetError),
    #[error("Validation Failed: {0}")]
    Validation(String),
    #[error("Error Deserializing Json: {0}")]
    Json(#[from] serde_json::Error),
}

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("Authorization Denied")]
    AuthDenied(Diagnostics),
    #[error("Error constructing authorization request: {0}")]
    Request(String),
}

#[allow(clippy::result_large_err)]
impl CedarContext {
    pub fn from(
        entities: EntityStore,
        schema_content: &str,
        policy_content: &str,
    ) -> Result<Self, ContextError> {
        let (schema, _) = Schema::from_cedarschema_str(schema_content)?;
        let policies = policy_content.parse()?;
        let validator = Validator::new(schema.clone());
        let output = validator.validate(&policies, ValidationMode::default());

        if output.validation_passed() {
            tracing::info!("All policy validation passed!");
            let authorizer = Authorizer::new();
            let c = Self {
                entities,
                authorizer,
                policies,
                schema,
            };

            Ok(c)
        } else {
            let error_string = output
                .validation_errors()
                .map(|err| format!("{err}"))
                .join("\n");
            Err(ContextError::Validation(error_string))
        }
    }

    pub fn new(entities: EntityStore) -> Result<Self, ContextError> {
        let schema_content = include_str!("../mega.cedarschema");
        let policy_content = include_str!("../mega_policies.cedar");
        let (schema, _) = Schema::from_cedarschema_str(schema_content).unwrap();
        let policies = policy_content.parse()?;
        let validator = Validator::new(schema.clone());
        let output = validator.validate(&policies, ValidationMode::default());

        if output.validation_passed() {
            tracing::info!("All policy validation passed!");
            let authorizer = Authorizer::new();
            let c = Self {
                entities,
                authorizer,
                policies,
                schema,
            };

            Ok(c)
        } else {
            let error_string = output
                .validation_errors()
                .map(|err| format!("{err}"))
                .join("\n");
            Err(ContextError::Validation(error_string))
        }
    }

    pub fn is_authorized(
        &self,
        principal: impl AsRef<EntityUid>,
        action: impl AsRef<EntityUid>,
        resource: impl AsRef<EntityUid>,
        context: Context,
    ) -> Result<(), Error> {
        let es = self.entities.as_entities(&self.schema);
        let q = Request::new(
            principal.as_ref().clone().into(),
            action.as_ref().clone().into(),
            resource.as_ref().clone().into(),
            context,
            Some(&self.schema),
        )
        .map_err(|e| Error::Request(e.to_string()))?;
        tracing::info!(
            "is_authorized request: principal: {}, action: {}, resource: {}",
            principal.as_ref(),
            action.as_ref(),
            resource.as_ref()
        );
        let response = self.authorizer.is_authorized(&q, &self.policies, &es);
        tracing::info!("Auth response: {:?}", response);
        match response.decision() {
            Decision::Allow => Ok(()),
            Decision::Deny => Err(Error::AuthDenied(response.diagnostics().clone())),
        }
    }
}
