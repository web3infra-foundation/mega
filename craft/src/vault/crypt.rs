use std::{
    io::Read,
    sync::{Arc, RwLock},
};

use pgp_key::{
    decrypt_message, delete_key, encrypt_message, generate_key_pair, list_keys, KeyPair,
};
use rusty_vault::{
    core::Core,
    logical::{Operation, Request},
};
use serde_json::json;

use crate::vault::pgp_key;

// the trait and impl for KeyPair is a preparation for crate Tongsuo.
// a trait for key
pub trait Key {
    // type
    type PublicKey;
    type PrivateKey;

    // generate key with primary id
    fn generate_key_full(primary_id: &str, key_name: &str, core: Arc<RwLock<Core>>, token: &str);
    // encrypt with public key
    fn encrypt(public_key_file_path: &str, core: Arc<RwLock<Core>>, token: &str);
    // decrypt with private key
    fn decrypt(private_key_file_path: &str, core: Arc<RwLock<Core>>, token: &str);
    // list keys
    fn list_keys(key_path: &str, core: Arc<RwLock<Core>>, token: &str);
    // delete key
    fn delete_key(key_name: &str, core: Arc<RwLock<Core>>, token: &str);
}

// OpenPGP Key
impl Key for KeyPair {
    type PublicKey = pgp::SignedPublicKey;
    type PrivateKey = pgp::SignedSecretKey;

    fn generate_key_full(primary_id: &str, key_name: &str, core: Arc<RwLock<Core>>, token: &str) {
        let _ = generate_key_full(primary_id, key_name, core, token);
    }

    fn encrypt(public_key_file_path: &str, core: Arc<RwLock<Core>>, token: &str) {
        let _ = encrypt_blob(public_key_file_path, core, token);
    }

    fn decrypt(private_key_file_path: &str, core: Arc<RwLock<Core>>, token: &str) {
        let _ = decrypt_blob(private_key_file_path, core, token);
    }

    fn list_keys(key_path: &str, core: Arc<RwLock<Core>>, token: &str) {
        let _ = list_keys(key_path, core, token);
    }

    fn delete_key(key_name: &str, core: Arc<RwLock<Core>>, token: &str) {
        let _ = delete_key(key_name, core, token);
    }
}

// Generate full key with pubkey, seckey, primary id.
// Arguments: primary_id, as &str, it should be written as "User <example@example.com>"; key_name, git-craft will keep ur key file as key_namepub.asc
pub fn generate_key_full(
    primary_id: &str,
    key_path: &str,
    core: Arc<RwLock<Core>>,
    token: &str,
) -> Result<KeyPair, anyhow::Error> {
    let core = core.write().unwrap();
    println!("Creating key pair, this will take a few seconds...");

    // generate_key_pair to generate key with a given non-default key id
    let key_pair = generate_key_pair(primary_id).expect("Failed to generate full key pair");
    // Generate a public key with primary id
    let pub_key = key_pair
        .public_key
        .to_armored_string(None)
        .expect("Failed to convert public key to armored ASCII string");

    // Generate a secret key
    let sec_key = key_pair
        .secret_key
        .to_armored_string(None)
        .expect("Failed to convert secret key to armored ASCII string");

    let kv_data = json!({
        "pub": pub_key,
        "sec": sec_key,
    })
    .as_object()
    .unwrap()
    .clone();

    let mut req = Request::new(key_path);
    req.operation = Operation::Write;
    req.body = Some(kv_data);
    req.client_token = token.to_string();
    core.handle_request(&mut req).unwrap();

    Ok(key_pair)
}
// A blob encrypt function,it can encrypt blob.data
// Argument: public_key_file_path, public key's file path; I set a default path now.
pub fn encrypt_blob(
    key_path: &str,
    core: Arc<RwLock<Core>>,
    token: &str,
) -> Result<(), anyhow::Error> {
    // Read blob data from standard input stream
    let mut blob_data = Vec::new();
    std::io::stdin().read_to_end(&mut blob_data).unwrap();
    // Get blob.data as msg to encrypt
    let msg = std::str::from_utf8(&blob_data).expect("Invalid UTF-8 sequence");

    let core = core.read().unwrap();
    let mut req = Request::new(key_path);
    req.operation = Operation::Read;
    req.client_token = token.to_string();
    let resp = core.handle_request(&mut req);
    let body = resp.unwrap().unwrap().data.unwrap();
    let pub_key = body["pub"].as_str().unwrap();

    // Encrypt the contents with the given public key
    let encrypted = encrypt_message(msg, pub_key).expect("Failed to encrypt message");
    // Print it, git will get encrypted data
    print!("{}", &encrypted);
    Ok(())
}

// A blob decrypt function,it can decrypt blob.data encrypted by encrypted_blob()
// Arguments: secret_key_file_path; I set a default one now.
pub fn decrypt_blob(
    key_path: &str,
    core: Arc<RwLock<Core>>,
    token: &str,
) -> Result<(), anyhow::Error> {
    // Read blob.data from standard input stream
    let mut blob_data = Vec::new();
    std::io::stdin().read_to_end(&mut blob_data).unwrap();
    // Set a encrypt_msg to get &str
    let encrypted_msg = std::str::from_utf8(&blob_data).expect("Invalid UTF-8 sequence");

    let core = core.read().unwrap();
    let mut req = Request::new(key_path);
    req.operation = Operation::Read;
    req.client_token = token.to_string();
    let resp = core.handle_request(&mut req);
    let body = resp.unwrap().unwrap().data.unwrap();
    let sec_key = body["sec"].as_str().unwrap();

    // Decrypt contents with the given secret key
    let decrypted_msg = decrypt_message(encrypted_msg, sec_key).expect("Failed to decrypt message");
    // Print decrypted contents, then git will write decrypted contents to origin file
    print!("{}", &decrypted_msg);
    Ok(())
}
