use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::Serialize;
use utoipa::ToSchema;

use callisto::{check_result, sea_orm_active_enums::CheckTypeEnum};
use common::errors::MegaError;
use jupiter::{model::mr_dto::MrInfoDto, storage::Storage};

use crate::merge_checker::mr_sync_checker::MrSyncChecker;

pub mod mr_sync_checker;

#[async_trait]
pub trait Checker: Send + Sync {
    async fn run(&self, params: &serde_json::Value) -> CheckResult;

    async fn build_params(&self, mr_info: &MrInfoDto) -> Result<serde_json::Value, MegaError>;
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, ToSchema)]
pub enum CheckType {
    GpgSignature,
    BranchProtection,
    CommitMessage,
    MrSync,
    MergeConflict,
    CiStatus,
    CodeReview,
}

impl CheckType {
    pub fn display_name(&self) -> &'static str {
        match self {
            CheckType::GpgSignature => "Gpg signature",
            CheckType::BranchProtection => "Branch protection",
            CheckType::CommitMessage => "Commit message",
            CheckType::MrSync => "Mr sync",
            CheckType::MergeConflict => "Merge conflict",
            CheckType::CiStatus => "Ci status",
            CheckType::CodeReview => "Code review",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CheckType::GpgSignature => "Verify whether the commit has a valid GPG signature and the key is trusted",
            CheckType::BranchProtection => "Ensure the merge target complies with branch protection policies, such as no direct merges to main and requiring squash or rebase",
            CheckType::CommitMessage => "Verify whether the commit message follows Conventional Commits or the internal agreed-upon format",
            CheckType::MrSync => "Ensure the MR is based on the latest commit of the target branch and determine whether a rebase is required",
            CheckType::MergeConflict => "The pull request must not have any unresolved merge conflicts",
            CheckType::CiStatus => "Verify that all required continuous integration pipelines have passed",
            CheckType::CodeReview => "Ensure the required reviewers have approved the merge request",
        }
    }
}

impl From<CheckTypeEnum> for CheckType {
    fn from(value: CheckTypeEnum) -> Self {
        match value {
            CheckTypeEnum::GpgSignature => CheckType::GpgSignature,
            CheckTypeEnum::BranchProtection => CheckType::BranchProtection,
            CheckTypeEnum::CommitMessage => CheckType::CommitMessage,
            CheckTypeEnum::MrSync => CheckType::MrSync,
            CheckTypeEnum::MergeConflict => CheckType::MergeConflict,
            CheckTypeEnum::CiStatus => CheckType::CiStatus,
            CheckTypeEnum::CodeReview => CheckType::CodeReview,
        }
    }
}

impl From<CheckType> for CheckTypeEnum {
    fn from(value: CheckType) -> Self {
        match value {
            CheckType::GpgSignature => CheckTypeEnum::GpgSignature,
            CheckType::BranchProtection => CheckTypeEnum::BranchProtection,
            CheckType::CommitMessage => CheckTypeEnum::CommitMessage,
            CheckType::MrSync => CheckTypeEnum::MrSync,
            CheckType::MergeConflict => CheckTypeEnum::MergeConflict,
            CheckType::CiStatus => CheckTypeEnum::CiStatus,
            CheckType::CodeReview => CheckTypeEnum::CodeReview,
        }
    }
}

#[derive(Debug)]
pub struct CheckResult {
    pub check_type_code: CheckType,
    pub status: String,
    pub message: String,
}

pub struct CheckerRegistry {
    checkers: HashMap<CheckType, Box<dyn Checker>>,
    storage: Arc<Storage>,
    #[allow(dead_code)]
    username: String,
}

impl CheckerRegistry {
    pub fn new(storage: Arc<Storage>, username: String) -> Self {
        let mut r = CheckerRegistry {
            checkers: HashMap::new(),
            storage: storage.clone(),
            username,
        };
        r.register(CheckType::MrSync, Box::new(MrSyncChecker { storage }));
        r
    }

    pub fn register(&mut self, check_type: CheckType, checker: Box<dyn Checker>) {
        self.checkers.insert(check_type, checker);
    }

    pub async fn run_checks(&self, mr_info: MrInfoDto) -> Result<(), MegaError> {
        let check_configs = self
            .storage
            .mr_storage()
            .get_checks_config_by_path(&mr_info.path)
            .await?;
        let mut save_models = vec![];

        for c_config in check_configs {
            if let Some(checker) = self.checkers.get(&c_config.check_type_code.into()) {
                let params = checker.build_params(&mr_info).await?;
                let res = checker.run(&params).await;
                let model = check_result::Model::new(
                    &mr_info.path,
                    &mr_info.link,
                    &mr_info.to_hash,
                    res.check_type_code.into(),
                    &res.status,
                    &res.message,
                );
                save_models.push(model);
            }
        }
        self.storage
            .mr_storage()
            .save_check_results(save_models)
            .await?;
        Ok(())
    }
}
