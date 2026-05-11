use pgp::composed::{Deserializable, EncryptionCaps};
pub use pgp::composed::{
    KeyType, SecretKeyParams, SecretKeyParamsBuilder, SignedPublicKey, SignedSecretKey,
    SubkeyParamsBuilder,
};
use rand::thread_rng;
/// This module provides functions for generating, loading, saving, and deleting PGP key pairs.
///
/// It uses the `pgp` crate for key generation and management, and stores the keys in a vault
/// using asynchronous operations.
use smallvec::smallvec;

use crate::integration::vault_core::{VaultCore, VaultCoreInterface};

const VAULT_KEY: &str = "pgp-signed-secret";

impl VaultCore {
    /// Generates a PGP key pair (public and secret) based on the provided parameters.
    ///
    /// # Arguments
    ///
    /// *   `params`: The parameters for the secret key, such as key type, size, and usage flags.
    /// *   `passwd`: An optional passphrase to encrypt the secret key. If `None`, the key is not encrypted.
    ///
    /// # Returns
    ///
    /// A tuple containing the armored string representations of the public and secret keys.
    /// The first element is the public key, and the second is the secret key.
    pub fn gen_pgp_keypair(
        &self,
        params: SecretKeyParams,
        _passwd: Option<String>,
    ) -> (SignedPublicKey, SignedSecretKey) {
        let mut rng = thread_rng();
        let signed_key = params
            .generate(&mut rng)
            .expect("failed to generate secret key, encrypted");
        let signed_pub = signed_key.to_public_key();

        (signed_pub, signed_key)
    }

    /// Loads the public key from the vault.
    ///
    /// # Returns
    ///
    /// An `Option` containing the `SignedPublicKey` if the key is found in the vault, otherwise `None`.
    pub async fn load_pub_key(&self) -> Option<SignedPublicKey> {
        let key = self.read_secret(VAULT_KEY).await.unwrap();
        if let Some(data) = key {
            let key = data["pub_key"].as_str().unwrap();
            let (key, _headers) = SignedPublicKey::from_string(key).expect("failed to parse key");
            key.verify_bindings().expect("invalid key");
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
    pub async fn load_sec_key(&self) -> Option<SignedSecretKey> {
        let key = self.read_secret(VAULT_KEY).await.unwrap();
        if let Some(data) = key {
            let key = data["sec_key"].as_str().unwrap();
            let (key, _headers) = SignedSecretKey::from_string(key).expect("failed to parse key");
            key.verify_bindings().expect("invalid key");
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
    ///
    /// # Panics
    ///
    /// When input is invalid.
    pub async fn save_keys(&self, pub_key: SignedPublicKey, sec_key: SignedSecretKey) {
        let pub_key = pub_key.to_armored_string(None.into()).unwrap();
        let sec_key = sec_key.to_armored_string(None.into()).unwrap();
        let data = serde_json::json!({
            "pub_key": pub_key,
            "sec_key": sec_key,
        })
        .as_object()
        .unwrap()
        .clone();
        self.write_secret(VAULT_KEY, Some(data))
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to write PGP keys: {e:?}");
            });
    }

    /// Deletes the key pair from the vault.
    pub async fn delete_keys(&self) {
        self.delete_secret(VAULT_KEY).await.unwrap_or_else(|e| {
            panic!("Failed to delete PGP keys: {e:?}");
        });
    }

    /// Creates a set of parameters for generating a PGP secret key.
    ///
    /// This function simplifies the creation of `SecretKeyParams` by pre-configuring several options
    /// such as key version, key type, capabilities (certify and sign), preferred algorithms, and subkeys.
    ///
    /// # Arguments
    ///
    /// *   `key_type`: The type of key to generate (e.g., RSA, ECDSA).
    /// *   `passwd`: An optional passphrase to encrypt the secret key. If `None`, the key is not encrypted.
    /// *   `uid`: The user ID associated with the key. This is typically an email address or name.
    ///
    /// # Returns
    ///
    /// A `SecretKeyParams` object configured with the specified parameters, ready for key generation.
    pub fn params(key_type: KeyType, passwd: Option<String>, uid: &str) -> SecretKeyParams {
        let version = pgp::types::KeyVersion::V6;

        let mut key_params = SecretKeyParamsBuilder::default();
        key_params
            .version(version)
            .key_type(key_type.clone())
            .can_certify(true)
            .can_sign(true)
            .primary_user_id(uid.into())
            .preferred_symmetric_algorithms(smallvec![
                pgp::crypto::sym::SymmetricKeyAlgorithm::AES256,
                pgp::crypto::sym::SymmetricKeyAlgorithm::AES192,
                pgp::crypto::sym::SymmetricKeyAlgorithm::AES128,
            ])
            .preferred_hash_algorithms(smallvec![
                pgp::crypto::hash::HashAlgorithm::Sha256,
                pgp::crypto::hash::HashAlgorithm::Sha384,
                pgp::crypto::hash::HashAlgorithm::Sha512,
                pgp::crypto::hash::HashAlgorithm::Sha224,
                pgp::crypto::hash::HashAlgorithm::Sha1,
            ])
            .preferred_compression_algorithms(smallvec![
                pgp::types::CompressionAlgorithm::ZLIB,
                pgp::types::CompressionAlgorithm::ZIP,
            ])
            .passphrase(passwd.clone())
            .subkey(
                SubkeyParamsBuilder::default()
                    .version(version)
                    .key_type(key_type)
                    .passphrase(passwd)
                    .can_encrypt(EncryptionCaps::All)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }
}

// TODO use mock core to test
#[cfg(test)]
mod tests {
    // use pgp::KeyType;

    // use super::*;

    // #[test]
    // fn test_gen_pgp_keypair() {
    //     const PASSWD: &str = "hello";
    //     const KEY_TYPE: KeyType = KeyType::Rsa(2048);
    //     const UID: &str = "test";
    //     let params = params(KEY_TYPE, Some(PASSWD.into()), UID);
    //     let (pk, sk) = gen_pgp_keypair(params, Some(PASSWD.into()));

    //     assert!(pk.verify().is_ok());
    //     assert!(sk.verify().is_ok());
    // }

    // #[tokio::test]
    // async fn test_save_load_delete_keys() {
    //     const PASSWD: &str = "hello";
    //     const KEY_TYPE: KeyType = KeyType::Rsa(2048);
    //     const UID: &str = "test";
    //     let params = params(KEY_TYPE, Some(PASSWD.into()), UID);
    //     let (pk, sk) = gen_pgp_keypair(params, Some(PASSWD.into()));

    //     save_keys(pk.clone(), sk.clone()).await;

    //     let loaded_pub_key = load_pub_key().await;
    //     assert!(loaded_pub_key.is_some());
    //     let loaded_pub_key = loaded_pub_key.unwrap();
    //     assert!(loaded_pub_key.verify().is_ok());
    //     assert_eq!(
    //         loaded_pub_key.to_armored_string(None.into()).unwrap(),
    //         pk.to_armored_string(None.into()).unwrap()
    //     );

    //     let loaded_sec_key = load_sec_key().await;
    //     assert!(loaded_sec_key.is_some());
    //     let loaded_sec_key = loaded_sec_key.unwrap();
    //     assert!(loaded_sec_key.verify().is_ok());
    //     assert_eq!(
    //         loaded_sec_key.to_armored_string(None.into()).unwrap(),
    //         sk.to_armored_string(None.into()).unwrap()
    //     );

    //     delete_keys().await;

    //     let loaded_pub_key = load_pub_key().await;
    //     assert!(loaded_pub_key.is_none());

    //     let loaded_sec_key = load_sec_key().await;
    //     assert!(loaded_sec_key.is_none());
    // }
}
