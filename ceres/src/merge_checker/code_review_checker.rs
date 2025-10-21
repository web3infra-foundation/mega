use crate::merge_checker::{CheckResult, Checker};
use async_trait::async_trait;
use common::errors::MegaError;
use jupiter::model::cl_dto::ClInfoDto;
use jupiter::storage::Storage;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

pub struct CodeReviewChecker {
    pub storage: Arc<Storage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CodeReviewParams {
    cl_link: String,
}

impl CodeReviewParams {
    fn from_value(v: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(v.clone())?)
    }
}

#[async_trait]
impl Checker for CodeReviewChecker {
    async fn run(&self, params: &Value) -> CheckResult {
        let params = CodeReviewParams::from_value(params).expect("parse params err");
        let mut res = CheckResult {
            check_type_code: crate::merge_checker::CheckType::CodeReview,
            status: crate::merge_checker::ConditionResult::FAILED,
            message: String::new(),
        };

        let approved = self.verify_cl(&params.cl_link).await;
        match approved {
            Ok(_) => {
                res.status = crate::merge_checker::ConditionResult::PASSED;
                res.message = String::from("All reviewers have approved the CL.");
            }

            Err(e) => {
                res.status = crate::merge_checker::ConditionResult::FAILED;
                res.message = format!("Code review check failed: {e}");
            }
        }

        res
    }

    async fn build_params(&self, cl_info: &ClInfoDto) -> Result<Value, MegaError> {
        Ok(serde_json::json!({
            "cl_link": cl_info.link,
        }))
    }
}

impl CodeReviewChecker {
    async fn verify_cl(&self, cl_link: &str) -> Result<(), MegaError> {
        let reviewers = self
            .storage
            .reviewer_storage()
            .list_reviewers(cl_link)
            .await?;

        let mut err_message = String::new();
        for reviewer in reviewers {
            if !reviewer.approved {
                let msg = format!("Reviewer {} has not approved the CL.\n", reviewer.id);
                err_message = err_message + &msg;
            }
        }

        if !err_message.is_empty() {
            return Err(MegaError::with_message(err_message));
        }
        Ok(())
    }
}
