use crate::merge_checker::{CheckResult, CheckType, Checker, ConditionResult};
use async_trait::async_trait;
use common::errors::MegaError;
use jupiter::model::mr_dto::MrInfoDto;
use jupiter::storage::Storage;
use pgp::{Deserializable, SignedPublicKey, StandaloneSignature};
use regex::Regex;
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
        let (commit_msg, signature) = extract_from_commit_content(commit_content);
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
        let (signature, _) = StandaloneSignature::from_string(&normalize_signature_block(signature))?;
        let (keys, _) = SignedPublicKey::from_armor_many(public_key.as_bytes())?;
        let header = format!("commit {}\0", message.len());
        let msg_bytes = [header.as_bytes(), message.as_bytes()].concat();

        if keys.into_iter().any(|key_res| {
            key_res
                .ok()
                .map(|k| signature.verify(&k, &msg_bytes).is_ok())
                .unwrap_or(false)
        }) {
            Ok(())
        } else {
            Err(MegaError::with_message("no key verifies the commit"))
        }
    }
}

fn normalize_signature_block(sig_block: &str) -> String {
    let mut lines = Vec::new();
    for (i, line) in sig_block.lines().enumerate() {
        if i == 0 {
            lines.push(line.trim_start_matches("gpgsig ").to_string());
        } else {
            lines.push(line.trim_start().to_string());
        }
    }
    lines.join("\n") + "\n"
}

fn extract_from_commit_content(msg_gpg: &str) -> (&str, Option<&str>) {
    const SIG_PATTERN: &str = r"gpgsig (-----BEGIN (?:PGP|SSH) SIGNATURE-----[\s\S]*?-----END (?:PGP|SSH) SIGNATURE-----)";
    let sig_regex = Regex::new(SIG_PATTERN).unwrap();

    if let Some(caps) = sig_regex.captures(msg_gpg) {
        let signature = caps.get(1).unwrap().as_str();

        let end = caps.get(0).unwrap().end();

        let msg = msg_gpg[end..].trim_start();
        (msg, Some(signature))
    } else {
        (msg_gpg.trim_start(), None)
    }
}

#[test]
fn test_extract_signature_from_commits() {
    let cm= r#"tree 341e54913a3a43069f2927cc0f703e5a9f730df1
    author benjamin.747 <benjamin.747@outlook.com> 1757467768 +0800
    committer benjamin.747 <benjamin.747@outlook.com> 1757491219 +0800
    gpgsig -----BEGIN PGP SIGNATURE-----
     
     iQJNBAABCAA3FiEEs4MaYUV7JcjxsVMPyqxGczTZ6K4FAmjBMC4ZHGJlbmphbWlu
     Ljc0N0BvdXRsb29rLmNvbQAKCRDKrEZzNNnorj73EADNpsyLAHsB3NgoeH+uy9Vq
     G2+LRtlvqv3QMK7vbQUadXHlQYWk25SIk+WJ1kG1AnUy5fqOrLSDTA1ny+qwpH8O
     +2sKCF/S1wlzqGWjCcRH5/ir9srsGIn9HbNqBjmU22NJ6Dt2jnqoUvtWfPwyqwWg
     VpjYlj390cFdXTpH5hMvtlmUQB+zCSKtWQW2Ur64h/UsGtllARlACi+KHQQmA2/p
     FLWNddvfJQpPM597DkGohQTD68g0PqOBhUkOHduHq7VHy68DVW+07bPNXK8JhJ8S
     4dyV1sZwcVcov0GcKl0wUbEqzy4gf+zV7DQhkfrSRQMBdo5vCWahYj1AbgaTiu8a
     hscshYDuWWqpxBU/+nCxOPskV29uUG1sRyXp3DqmKJZpnO9CVdw3QaVrqnMEeh2S
     t/wYRI9aI1A+Mi/DETom5ifTVygMkK+3m1h7pAMOlblFEdZx2sDXPRG2IEUcatr4
     Jb2+7PUJQXxUQnwHC7xHHxRh6a2h8TfEJfSoEyrgzxZ0CRxJ6XMJaJu0UwZ2xMsx
     Lgmeu6miB/imwxz5R5RL2yVHbgllSlO5l12AIeBaPoarKXYPSALigQnKCXu5OM3x
     Jq5qsSGtxdr6S1VgLyYHR4o69bQjzBp9K47J3IXqvrpo/ZiO/6Mspk2ZRWhGj82q
     e3qERPp5b7+hA+M7jKPyJg==
     =UeLf
     -----END PGP SIGNATURE-----
    
    test parse commit from bytes
    "#;

    let (msg, sig) = extract_from_commit_content(cm);
    let sig = sig.expect("unable to parse");
    print!("{msg}\n{sig}");

    assert_eq!(
        msg,
        "test parse commit from bytes\n    "
    );

    assert_eq!(
        sig,
        r#"-----BEGIN PGP SIGNATURE-----
     
     iQJNBAABCAA3FiEEs4MaYUV7JcjxsVMPyqxGczTZ6K4FAmjBMC4ZHGJlbmphbWlu
     Ljc0N0BvdXRsb29rLmNvbQAKCRDKrEZzNNnorj73EADNpsyLAHsB3NgoeH+uy9Vq
     G2+LRtlvqv3QMK7vbQUadXHlQYWk25SIk+WJ1kG1AnUy5fqOrLSDTA1ny+qwpH8O
     +2sKCF/S1wlzqGWjCcRH5/ir9srsGIn9HbNqBjmU22NJ6Dt2jnqoUvtWfPwyqwWg
     VpjYlj390cFdXTpH5hMvtlmUQB+zCSKtWQW2Ur64h/UsGtllARlACi+KHQQmA2/p
     FLWNddvfJQpPM597DkGohQTD68g0PqOBhUkOHduHq7VHy68DVW+07bPNXK8JhJ8S
     4dyV1sZwcVcov0GcKl0wUbEqzy4gf+zV7DQhkfrSRQMBdo5vCWahYj1AbgaTiu8a
     hscshYDuWWqpxBU/+nCxOPskV29uUG1sRyXp3DqmKJZpnO9CVdw3QaVrqnMEeh2S
     t/wYRI9aI1A+Mi/DETom5ifTVygMkK+3m1h7pAMOlblFEdZx2sDXPRG2IEUcatr4
     Jb2+7PUJQXxUQnwHC7xHHxRh6a2h8TfEJfSoEyrgzxZ0CRxJ6XMJaJu0UwZ2xMsx
     Lgmeu6miB/imwxz5R5RL2yVHbgllSlO5l12AIeBaPoarKXYPSALigQnKCXu5OM3x
     Jq5qsSGtxdr6S1VgLyYHR4o69bQjzBp9K47J3IXqvrpo/ZiO/6Mspk2ZRWhGj82q
     e3qERPp5b7+hA+M7jKPyJg==
     =UeLf
     -----END PGP SIGNATURE-----"#
    );
}