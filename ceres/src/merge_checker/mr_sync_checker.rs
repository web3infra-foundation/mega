use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use common::errors::MegaError;
use jupiter::{model::mr_dto::MrInfoDto, storage::Storage};

use crate::merge_checker::{CheckResult, CheckType, Checker};

pub struct MrSyncChecker {
    pub storage: Arc<Storage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MrSyncParams {
    mr_from: String,
    current: String,
}

impl MrSyncParams {
    fn from_value(v: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(v.clone())?)
    }
}

#[async_trait]
impl Checker for MrSyncChecker {
    async fn run(&self, params: &serde_json::Value) -> CheckResult {
        let params = MrSyncParams::from_value(params).expect("parse params err");
        let mut res = CheckResult {
            check_type_code: CheckType::MrSync,
            status: String::from("PENDING"),
            message: String::new(),
        };
        if params.mr_from == params.current {
            res.status = String::from("PASSED");
        } else {
            res.status = String::from("FAILED");
            res.message =
                String::from("The pull request must not have any unresolved merge conflicts");
        }
        res
    }

    async fn build_params(&self, mr_info: &MrInfoDto) -> Result<serde_json::Value, MegaError> {
        let refs = self
            .storage
            .mono_storage()
            .get_ref(&mr_info.path)
            .await?
            .expect("Err: MR Related Refs Not Found");
        Ok(serde_json::json!({
            "mr_from": mr_info.from_hash,
            "current": refs.ref_commit_hash
        }))
    }
}
