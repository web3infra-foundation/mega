use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use common::errors::MegaError;
use jupiter::{model::cl_dto::ClInfoDto, storage::Storage};

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
        let refs = self
            .storage
            .mono_storage()
            .get_main_ref(&cl_info.path)
            .await?
            .expect("Err: CL Related Refs Not Found");
        Ok(serde_json::json!({
            "cl_from": cl_info.from_hash,
            "current": refs.ref_commit_hash
        }))
    }
}
