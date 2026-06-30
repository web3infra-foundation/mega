use std::sync::Arc;

use async_trait::async_trait;
use common::{errors::MegaError, utils::ZERO_ID};
use jupiter::{model::cl_dto::ClInfoDto, storage::Storage};
use serde::Deserialize;

use crate::merge_checker::{CheckResult, CheckType, Checker, ConditionResult};

pub struct ClSyncChecker {
    pub storage: Arc<Storage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClSyncParams {
    cl_from: String,
    current: String,
}

impl ClSyncParams {
    fn from_value(v: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(v.clone())?)
    }
}

#[async_trait]
impl Checker for ClSyncChecker {
    async fn run(&self, params: &serde_json::Value) -> CheckResult {
        let params = ClSyncParams::from_value(params).expect("parse params err");
        let mut res = CheckResult {
            check_type_code: CheckType::ClSync,
            status: ConditionResult::FAILED,
            message: String::new(),
        };
        if params.cl_from == params.current {
            res.status = ConditionResult::PASSED;
        } else {
            res.message =
                String::from("The pull request must not have any unresolved merge conflicts");
        }
        res
    }

    async fn build_params(&self, cl_info: &ClInfoDto) -> Result<serde_json::Value, MegaError> {
        let current = match self
            .storage
            .mono_storage()
            .get_main_ref(&cl_info.path)
            .await?
        {
            Some(refs) => refs.ref_commit_hash,
            None if cl_info.from_hash == ZERO_ID => ZERO_ID.to_string(),
            None => {
                return Err(MegaError::Other(format!(
                    "Main ref not found for CL path {}",
                    cl_info.path
                )));
            }
        };
        Ok(serde_json::json!({
            "cl_from": cl_info.from_hash,
            "current": current
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use jupiter::storage::Storage;
    use serde_json::json;

    use super::ClSyncChecker;
    use crate::merge_checker::{CheckType, Checker, ConditionResult};

    #[tokio::test]
    async fn cl_sync_checker_passes_when_hashes_match() {
        let checker = ClSyncChecker {
            storage: Arc::new(Storage::mock()),
        };
        let result = checker
            .run(&json!({"cl_from": "abc123", "current": "abc123"}))
            .await;
        assert_eq!(result.check_type_code, CheckType::ClSync);
        assert_eq!(result.status, ConditionResult::PASSED);
    }

    #[tokio::test]
    async fn cl_sync_checker_fails_when_hashes_differ() {
        let checker = ClSyncChecker {
            storage: Arc::new(Storage::mock()),
        };
        let result = checker
            .run(&json!({"cl_from": "abc123", "current": "def456"}))
            .await;
        assert_eq!(result.check_type_code, CheckType::ClSync);
        assert_eq!(result.status, ConditionResult::FAILED);
        assert!(!result.message.is_empty());
    }
}
