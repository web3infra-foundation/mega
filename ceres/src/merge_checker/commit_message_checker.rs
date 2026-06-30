use async_trait::async_trait;
use common::{errors::MegaError, utils::check_conventional_commits_message};
use jupiter::model::cl_dto::ClInfoDto;
use serde_json::{Value, json};

use crate::merge_checker::{CheckResult, CheckType, Checker, ConditionResult};

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
            "Commit message does not follow conventional commits. Please make sure your CL title follows the Conventional Commits specification.".to_string()
        };

        CheckResult {
            check_type_code: CheckType::CommitMessage,
            status,
            message,
        }
    }

    async fn build_params(&self, cl_info: &ClInfoDto) -> Result<Value, MegaError> {
        let title = cl_info.title.clone();
        Ok(json!({
            "title": title,
        }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::CommitMessageChecker;
    use crate::merge_checker::{CheckType, Checker, ConditionResult};

    #[tokio::test]
    async fn commit_message_checker_passes_conventional_title() {
        let checker = CommitMessageChecker;
        let result = checker
            .run(&json!({"title": "feat: add webhook support"}))
            .await;
        assert_eq!(result.check_type_code, CheckType::CommitMessage);
        assert_eq!(result.status, ConditionResult::PASSED);
    }

    #[tokio::test]
    async fn commit_message_checker_fails_non_conventional_title() {
        let checker = CommitMessageChecker;
        let result = checker.run(&json!({"title": "add webhook support"})).await;
        assert_eq!(result.check_type_code, CheckType::CommitMessage);
        assert_eq!(result.status, ConditionResult::FAILED);
        assert!(!result.message.is_empty());
    }
}
