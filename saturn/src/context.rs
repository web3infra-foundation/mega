use cedar_policy::{
    Authorizer, HumanSchemaError, ParseErrors, PolicySet, PolicySetError, Schema, SchemaError,
    ValidationMode, Validator,
};
use itertools::Itertools;
use std::path::PathBuf;
use thiserror::Error;

#[allow(dead_code)]
pub struct AppContext {
    // entities: EntityStore,
    authorizer: Authorizer,
    policies: PolicySet,
    schema: Schema,
}

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("Error Parsing Json Schema: {0}")]
    JsonSchema(#[from] SchemaError),
    #[error("Error Parsing Human-readable Schema: {0}")]
    CedarSchema(#[from] HumanSchemaError),
    #[error("Error Parsing PolicySet: {0}")]
    Policy(#[from] ParseErrors),
    #[error("Error Processing PolicySet: {0}")]
    PolicySet(#[from] PolicySetError),
    #[error("Validation Failed: {0}")]
    Validation(String),
    #[error("Error Deserializing Json: {0}")]
    Json(#[from] serde_json::Error),
}

impl AppContext {
    pub fn new(
        _: impl Into<PathBuf>,
        schema_path: impl Into<PathBuf>,
        policies_path: impl Into<PathBuf>,
    ) -> Result<Self, ContextError> {
        let schema_path = schema_path.into();
        let policies_path = policies_path.into();

        let schema_file = std::fs::File::open(schema_path)?;
        let (schema, _) = Schema::from_file_natural(schema_file).unwrap();
        // let entities_file = std::fs::File::open(entities_path.into())?;
        // let entities = serde_json::from_reader(entities_file)?;
        let policy_src = std::fs::read_to_string(policies_path)?;
        let policies = policy_src.parse()?;
        let validator = Validator::new(schema.clone());
        let output = validator.validate(&policies, ValidationMode::default());

        if output.validation_passed() {
            tracing::info!("All policy validation passed!");
            let authorizer = Authorizer::new();
            let c = Self {
                // entities,
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
}
