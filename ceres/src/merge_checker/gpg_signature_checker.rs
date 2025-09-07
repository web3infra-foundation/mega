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

        let is_verified = self
            .verify_mr(&params.mr_to, params.committer)
            .await
            .expect("cannot verify commits");
        if is_verified {
            res.status = ConditionResult::PASSED;
        } else {
            res.message = String::from("The commit GPG signature verification failed");
        }

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
    async fn verify_mr(&self, mr_to: &str, assignee: String) -> Result<bool, MegaError> {
        let commit = self
            .storage
            .mono_storage()
            .get_commit_by_hash(mr_to)
            .await?
            .ok_or_else(|| MegaError::with_message("Commit not found"))?;

        let content = commit.content.clone().unwrap_or_default();
        let verified = self.verify_commit_gpg_signature(&content, assignee).await?;

        Ok(verified)
    }

    async fn verify_commit_gpg_signature(
        &self,
        commit_content: &str,
        assignee: String,
    ) -> Result<bool, MegaError> {
        let (commit_msg, signature) = parse_commit_msg(commit_content);
        if signature.is_none() {
            return Ok(false); // No signature to verify
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
                .await?;
            if verified {
                return Ok(true); // Signature verified successfully
            }
        }

        Ok(false) // No key could verify the signature
    }

    async fn verify_signature_with_key(
        &self,
        public_key: &str,
        signature: &str,
        message: &str,
    ) -> Result<bool, MegaError> {
        let (public_key, _) = SignedPublicKey::from_string(public_key)?;
        let (signature, _) = StandaloneSignature::from_string(signature)?;

        Ok(signature.verify(&public_key, message.as_bytes()).is_ok())
    }
}
