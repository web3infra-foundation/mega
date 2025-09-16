use crate::merge_checker::{CheckResult, CheckType, Checker, ConditionResult};
use async_trait::async_trait;
use common::errors::MegaError;
use common::utils::parse_commit_msg;
use jupiter::model::mr_dto::MrInfoDto;
use jupiter::storage::Storage;
use pgp::{Deserializable, SignedPublicKey, StandaloneSignature};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

pub struct GpgSignatureChecker {
    pub storage: Arc<Storage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GpgSignatureParams {
    mr_to: String,
    committer: String,
}

impl GpgSignatureParams {
    fn from_value(v: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(v.clone())?)
    }
}

#[async_trait]
impl Checker for GpgSignatureChecker {
    async fn run(&self, params: &serde_json::Value) -> crate::merge_checker::CheckResult {
        let params = GpgSignatureParams::from_value(params).expect("parse params err");
        let mut res = CheckResult {
            check_type_code: CheckType::GpgSignature,
            status: ConditionResult::FAILED,
            message: String::new(),
        };

        let is_verified = self.verify_mr(&params.mr_to, params.committer).await;
        match is_verified {
            Ok(_) => {
                res.status = ConditionResult::PASSED;
                res.message = String::from("PASSED");
            }

            Err(e) => {
                res.status = ConditionResult::FAILED;
                res.message = format!("Error during GPG signature verification: {e}");
            }
        };

        res
    }

    async fn build_params(&self, mr_info: &MrInfoDto) -> Result<Value, MegaError> {
        Ok(serde_json::json!({
            "mr_to": mr_info.to_hash,
            "committer": mr_info.username,
        }))
    }
}

impl GpgSignatureChecker {
    async fn verify_mr(&self, mr_to: &str, assignee: String) -> Result<(), MegaError> {
        let commit = self
            .storage
            .mono_storage()
            .get_commit_by_hash(mr_to)
            .await?
            .ok_or_else(|| MegaError::with_message("Commit not found"))?;

        let content = commit.content.clone().unwrap_or_default();
        self.verify_commit_gpg_signature(&content, assignee).await?;

        Ok(())
    }

    async fn verify_commit_gpg_signature(
        &self,
        commit_content: &str,
        assignee: String,
    ) -> Result<(), MegaError> {
        let (commit_msg, signature) = parse_commit_msg(commit_content);
        if signature.is_none() {
            return Err(MegaError::with_message(format!(
                "No GPG signature found for user {assignee}"
            )));
        }

        let sig_str = signature.unwrap();

        // Remove "gpgsig " prefix if present
        let sig = sig_str
            .strip_prefix("gpgsig ")
            .map(|s| s.trim())
            .unwrap_or(sig_str);

        let keys = self.storage.gpg_storage().list_user_gpg(assignee).await?;

        for key in keys {
            let verified = self
                .verify_signature_with_key(&key.public_key, sig, commit_msg)
                .await;

            if verified.is_ok() {
                return Ok(());
            }
        }

        Err(MegaError::with_message(
            "No valid GPG key found to verify the signature",
        ))
    }

    async fn verify_signature_with_key(
        &self,
        public_key: &str,
        signature: &str,
        message: &str,
    ) -> Result<(), MegaError> {
        let (public_key, _) = SignedPublicKey::from_string(public_key)?;
        let (signature, _) = StandaloneSignature::from_string(signature)?;

        signature
            .verify(&public_key, message.as_bytes())
            .map_err(|e| MegaError::with_message(format!("Signature verification failed: {e}")))?;

        Ok(())
    }
}
