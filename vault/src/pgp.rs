/// This module provides functions for generating, loading, saving, and deleting PGP key pairs.
///
/// It uses the `pgp` crate for key generation and management, and stores the keys in a vault
/// using asynchronous operations.
use pgp::composed::{Deserializable, SignedPublicKey, SignedSecretKey};
use pgp::types::SecretKeyTrait;
use pgp::SecretKeyParams;
use secp256k1::rand::{CryptoRng, Rng};

use crate::vault::{read_secret, write_secret};

const VAULT_KEY: &str = "pgp-signed-secret";

/// Generates a PGP key pair (public and secret) based on the provided parameters.
///
/// # Arguments
///
/// *   `rng`: A random number generator. Must implement `Rng` and `CryptoRng` traits.
/// *   `params`: The parameters for the secret key, such as key type, size, and usage flags.
/// *   `passwd`: An optional passphrase to encrypt the secret key. If `None`, the key is not encrypted.
///
/// # Returns
///
/// A tuple containing the armored string representations of the public and secret keys.
/// The first element is the public key, and the second is the secret key.
pub fn gen_pgp_keys<R: Rng + CryptoRng>(
    mut rng: R,
    params: SecretKeyParams,
    passwd: Option<String>,
) -> (String, String) {
    let key = params
        .generate(&mut rng)
        .expect("failed to generate secret key, encrypted");

    let signed_key = key
        .sign(&mut rng, || {
            if let Some(passwd) = passwd.clone() {
                passwd
            } else {
                "".into()
            }
        })
        .expect("failed to sign key");

    let sec_armored = signed_key
        .to_armored_string(None.into())
        .expect("failed to serialize key");

    let pub_key = signed_key.public_key();
    let pub_signed = pub_key
        .sign(rng, &signed_key, || {
            if let Some(passwd) = passwd {
                passwd
            } else {
                "".into()
            }
        })
        .expect("failed to sign key");
    let pub_armored = pub_signed
        .to_armored_string(None.into())
        .expect("failed to serialize key");

    (pub_armored, sec_armored)
}

/// Loads the public key from the vault.
///
/// # Returns
///
/// An `Option` containing the `SignedPublicKey` if the key is found in the vault, otherwise `None`.
pub async fn load_pub_key() -> Option<SignedPublicKey> {
    let key = read_secret(VAULT_KEY).await.unwrap();
    if let Some(key) = key {
        let data = key.data.unwrap();
        let key = data["pub_key"].as_str().unwrap();
        let (key, _headers) = SignedPublicKey::from_string(key).expect("failed to parse key");
        key.verify().expect("invalid key");
        Some(key)
    } else {
        None
    }
}

/// Loads the public key from the vault.
///
/// # Returns
///
/// An `Option` containing the `SignedPublicKey` if the key is found in the vault, otherwise `None`.
pub async fn load_sec_key() -> Option<SignedSecretKey> {
    let key = read_secret(VAULT_KEY).await.unwrap();
    if let Some(key) = key {
        let data = key.data.unwrap();
        let key = data["sec_key"].as_str().unwrap();
        let (key, _headers) = SignedSecretKey::from_string(key).expect("failed to parse key");
        key.verify().expect("invalid key");
        Some(key)
    } else {
        None
    }
}

/// Saves the public and secret keys to the vault.
///
/// # Arguments
///
/// *   `pub_key`: The armored string representation of the public key.
/// *   `sec_key`: The armored string representation of the secret key.
pub async fn save_keys(pub_key: String, sec_key: String) {
    let data = serde_json::json!({
        "pub_key": pub_key,
        "sec_key": sec_key,
    })
    .as_object()
    .unwrap()
    .clone();
    write_secret(VAULT_KEY, Some(data))
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to write PGP keys: {:?}", e);
        });
}

/// Deletes the key pair from the vault.
pub async fn delete_keys() {
    write_secret(VAULT_KEY, None).await.unwrap_or_else(|e| {
        panic!("Failed to delete PGP keys: {:?}", e);
    });
}

#[cfg(test)]
mod tests {
    use pgp::{crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm}, types::{CompressionAlgorithm, KeyVersion}, KeyType, SecretKeyParamsBuilder, SubkeyParamsBuilder};
    use smallvec::smallvec;

    use super::*;

    fn get_params(passwd: Option<String>) -> SecretKeyParams {
        let version = KeyVersion::V6;

        let mut key_params = SecretKeyParamsBuilder::default();
        key_params
            .version(version)
            .key_type(KeyType::Rsa(2048))
            .can_certify(true)
            .can_sign(true)
            .primary_user_id("Me <me@mail.com>".into())
            .preferred_symmetric_algorithms(smallvec![
                SymmetricKeyAlgorithm::AES256,
                SymmetricKeyAlgorithm::AES192,
                SymmetricKeyAlgorithm::AES128,
            ])
            .preferred_hash_algorithms(smallvec![
                HashAlgorithm::SHA2_256,
                HashAlgorithm::SHA2_384,
                HashAlgorithm::SHA2_512,
                HashAlgorithm::SHA2_224,
                HashAlgorithm::SHA1,
            ])
            .preferred_compression_algorithms(smallvec![
                CompressionAlgorithm::ZLIB,
                CompressionAlgorithm::ZIP,
            ]);

        key_params
            .clone()
            .passphrase(passwd)
            .subkey(
                SubkeyParamsBuilder::default()
                    .version(version)
                    .key_type(KeyType::Rsa(2048))
                    .passphrase(Some("hello".into()))
                    .can_encrypt(true)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    #[test]
    fn test_gen_pgp_keypair() {
        const PASSWD: &str = "hello";
        let rng = secp256k1::rand::rngs::OsRng;
        let params = get_params(Some(PASSWD.into()));
        let (pk, sk) = gen_pgp_keys(rng, params, Some(PASSWD.into()));

        let (pk, _) = SignedPublicKey::from_string(&pk).expect("failed to parse key");
        assert!(pk.verify().is_ok());

        let (sk, _) = SignedSecretKey::from_string(&sk).expect("failed to parse key");
        assert!(sk.verify().is_ok());
    }

    #[tokio::test]
    async fn test_save_load_delete_keys() {
        const PASSWD: &str = "hello";
        let rng = secp256k1::rand::rngs::OsRng;
        let params = get_params(Some(PASSWD.into()));
        let (pk, sk) = gen_pgp_keys(rng, params, Some(PASSWD.into()));

        save_keys(pk.clone(), sk.clone()).await;

        let loaded_pub_key = load_pub_key().await;
        assert!(loaded_pub_key.is_some());
        let loaded_pub_key = loaded_pub_key.unwrap();
        assert!(loaded_pub_key.verify().is_ok());
        assert_eq!(
            loaded_pub_key.to_armored_string(None.into()).unwrap(),
            SignedPublicKey::from_string(&pk)
                .unwrap()
                .0
                .to_armored_string(None.into())
                .unwrap()
        );

        let loaded_sec_key = load_sec_key().await;
        assert!(loaded_sec_key.is_some());
        let loaded_sec_key = loaded_sec_key.unwrap();
        assert!(loaded_sec_key.verify().is_ok());
        assert_eq!(
            loaded_sec_key.to_armored_string(None.into()).unwrap(),
            SignedSecretKey::from_string(&sk)
                .unwrap()
                .0
                .to_armored_string(None.into())
                .unwrap()
        );

        delete_keys().await;

        let loaded_pub_key = load_pub_key().await;
        assert!(loaded_pub_key.is_none());

        let loaded_sec_key = load_sec_key().await;
        assert!(loaded_sec_key.is_none());
    }
}
