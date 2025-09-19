use crate::merge_checker::{CheckResult, CheckType, Checker, ConditionResult};
use async_trait::async_trait;
use common::errors::MegaError;
use jupiter::model::mr_dto::MrInfoDto;
use jupiter::storage::Storage;
use pgp::composed::{Deserializable, SignedPublicKey, StandaloneSignature};
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

        let sig = signature.unwrap();

        let keys = self.storage.gpg_storage().list_user_gpg(assignee).await?;

        for key in keys {
            let verified = self
                .verify_signature_with_key(&key.public_key, &sig, &commit_msg)
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
        let pub_key = SignedPublicKey::from_string(public_key)
            .map_err(|e| MegaError::with_message(format!("Failed to parse public key: {e}")))?
            .0;
        let sig = StandaloneSignature::from_string(signature)
            .map_err(|e| MegaError::with_message(format!("Failed to parse signature: {e}")))?
            .0;
        let bytes = message.as_bytes();
        sig.signature
            .verify(&pub_key, &bytes[..])
            .map_err(|e| MegaError::with_message(format!("Signature verification failed: {e}")))?;

        Ok(())
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

fn extract_from_commit_content(msg_gpg: &str) -> (String, Option<String>) {
    const SIG_PATTERN: &str =
        r"gpgsig (-----BEGIN (?:PGP|SSH) SIGNATURE-----[\s\S]*?-----END (?:PGP|SSH) SIGNATURE-----)";
    let sig_regex = Regex::new(SIG_PATTERN).unwrap();

    if let Some(caps) = sig_regex.captures(msg_gpg) {
        let signature = caps.get(1).unwrap().as_str().to_string();
        let signature = format!("{signature}\n");
        let start = caps.get(0).unwrap().start();
        let end = caps.get(0).unwrap().end();

        let mut commit = String::new();
        commit.push_str(&msg_gpg[..start-1]); 
        commit.push_str(&msg_gpg[end..]);   

        while commit.starts_with('\n') {
            commit = commit[1..].to_string();
        }
        // Add a trailing newline if not present
        let commit = if commit.ends_with('\n') {
            commit
        } else {
            format!("{commit}\n")
        };

        (commit, Some(normalize_signature_block(&signature)))
    } else {
        (msg_gpg.to_string(), None)
    }
}

#[test]
fn test_extract_signature_from_commits() {
    let cm = r#"tree 52a266a58f2c028ad7de4dfd3a72fdf76b0d4e24
author AidCheng <cn.aiden.cheng@gmail.com> 1758211153 +0100
committer AidCheng <cn.aiden.cheng@gmail.com> 1758211153 +0100
gpgsig -----BEGIN PGP SIGNATURE-----
 
 iQGzBAABCAAdFiEExZ7S27JTGFCk8+gpQv9AeDZzXb0FAmjMLFEACgkQQv9AeDZz
 Xb3zKQwAoZ+CpV7x8SyF2ZWm3MJZmqa12G51s1/N1/ND6VRiyO6cIS71w5nA0RY/
 vNwQlxjD2gDq8MFr8eFIwQCBrHVFx4QrmAnLNhtbnf9fGws//nPEPrzm3bHaRmt4
 tnjstUEJDE2sKztTcics740FZJnXW13MGQZV6HEODYo00zldOUYWqNflTYt8oVmQ
 LrPWncxOVXBKnhs1X+Zh8aJIj5Gnqrl0A8PMRlqSOKMEQzZD0Erd2/Fj+uzemGwl
 EexiTwtuoMBSjAWCTholW8HzvHOoSvSj3fV5bKD7XbWtBaB62rCtqNvqJK2QX8aM
 fSNM3KnloWz+sDFGYnmGacNsn+uqxF517DT/mqJeNMrI2MWGMzuQolHqeNTRSCiq
 Mvslf2i50W0P8npM+U+JJBIaNGReslK0zlsSwc4X50ReDXJxxi5QvlS0WLEmJIOl
 f58UBUCXm/LEYABTW5xKdEFoSxmpGcZ09G7/O6CvqPVau0gGcqwjl4LP3/496ifz
 jjI4Ah4p
 =QXLc
 -----END PGP SIGNATURE-----

test

Signed-off-by: AidCheng <cn.aiden.cheng@gmail.com>"#;
    let (msg, sig) = extract_from_commit_content(cm);
    let sig = sig.expect("unable to parse");
    println!("{msg}\n{sig}");

    assert_eq!(msg, r#"tree 52a266a58f2c028ad7de4dfd3a72fdf76b0d4e24
author AidCheng <cn.aiden.cheng@gmail.com> 1758211153 +0100
committer AidCheng <cn.aiden.cheng@gmail.com> 1758211153 +0100

test

Signed-off-by: AidCheng <cn.aiden.cheng@gmail.com>
"#);

    assert_eq!(
        sig,
        r#"-----BEGIN PGP SIGNATURE-----

iQGzBAABCAAdFiEExZ7S27JTGFCk8+gpQv9AeDZzXb0FAmjMLFEACgkQQv9AeDZz
Xb3zKQwAoZ+CpV7x8SyF2ZWm3MJZmqa12G51s1/N1/ND6VRiyO6cIS71w5nA0RY/
vNwQlxjD2gDq8MFr8eFIwQCBrHVFx4QrmAnLNhtbnf9fGws//nPEPrzm3bHaRmt4
tnjstUEJDE2sKztTcics740FZJnXW13MGQZV6HEODYo00zldOUYWqNflTYt8oVmQ
LrPWncxOVXBKnhs1X+Zh8aJIj5Gnqrl0A8PMRlqSOKMEQzZD0Erd2/Fj+uzemGwl
EexiTwtuoMBSjAWCTholW8HzvHOoSvSj3fV5bKD7XbWtBaB62rCtqNvqJK2QX8aM
fSNM3KnloWz+sDFGYnmGacNsn+uqxF517DT/mqJeNMrI2MWGMzuQolHqeNTRSCiq
Mvslf2i50W0P8npM+U+JJBIaNGReslK0zlsSwc4X50ReDXJxxi5QvlS0WLEmJIOl
f58UBUCXm/LEYABTW5xKdEFoSxmpGcZ09G7/O6CvqPVau0gGcqwjl4LP3/496ifz
jjI4Ah4p
=QXLc
-----END PGP SIGNATURE-----
"#
    );

    let pk = r#"-----BEGIN PGP PUBLIC KEY BLOCK-----

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
F5MtAwnDBeT2Qg==
=Q/C5
-----END PGP PUBLIC KEY BLOCK-----
"#;
    let pub_key = SignedPublicKey::from_string(&pk)
        .expect("unable to parse key")
        .0;
    let sig = StandaloneSignature::from_string(&sig)
        .expect("unable to parse sig")
        .0;
    let bytes = msg.as_bytes();
    sig.signature
        .verify(&pub_key, &bytes[..])
        .expect("unable to verify");

    assert!(true);
}

// #[cfg(test)]
// mod tests {
//     use std::fs;

//     use super::*;
//     #[tokio::test]
//     async fn test_verify_key() {
//         let pub_key_file = "pk.key";
//         let sig = fs::read_to_string("sig.txt").unwrap();
//         let sig = StandaloneSignature::from_string(&sig).unwrap().0;

//         let data = fs::read_to_string("commit.txt").unwrap().as_bytes().to_vec();

//         // Verify the signature using the public key
//         let key_string = fs::read_to_string(pub_key_file).expect("Failed to load public key");
//         let public_key = SignedPublicKey::from_string(&key_string).unwrap().0;

//         sig.signature.verify(&public_key, &data[..]).unwrap();    
//     }
// }