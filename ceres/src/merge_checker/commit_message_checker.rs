use async_trait::async_trait;
use jupiter::model::mr_dto::MrInfoDto;
use serde_json::{json, Value};

use crate::merge_checker::{CheckResult, CheckType, Checker, ConditionResult};
use common::{errors::MegaError, utils::check_conventional_commits_message};

pub struct CommitMessageChecker;

#[async_trait]
impl Checker for CommitMessageChecker {
    async fn run(&self, params: &Value) -> CheckResult {
        let title = params["title"].as_str().unwrap_or_default();
        let status = if check_conventional_commits_message(title) {
            ConditionResult::PASSED
        } else {
            ConditionResult::FAILED
        };
        let message = if status == ConditionResult::PASSED {
            "Commit message follows conventional commits".to_string()
        } else {
            "Commit message does not follow conventional commits. Please make sure your MR title follows the Conventional Commits specification.".to_string()
        };

        CheckResult {
            check_type_code: CheckType::CommitMessage,
            status,
            message,
        }
    }

    async fn build_params(&self, mr_info: &MrInfoDto) -> Result<Value, MegaError> {
        let title = mr_info.title.clone();
        Ok(json!({
            "title": title,
        }))
    }
}
