use std::{
    io::Read,
    sync::{Arc, RwLock},
};

use anyhow::Ok;
use pgp_key::{decrypt_message, encrypt_message, generate_key_pair};
use rusty_vault::{
    core::Core,
    logical::{Operation, Request},
};
use serde_json::json;

use crate::pgp_key::{self, delete_key, list_keys, KeyPair};

// the trait and impl for KeyPair is a preparation for crate Tongsuo.
// a trait for key
pub trait Key {
    // type
    type PublicKey;
    type PrivateKey;

    // function
    // generate default key
    fn generate_key(core: Arc<RwLock<Core>>);
    // generate key with primary id
    fn generate_key_full(primary_id: &str, key_name: &str, core: Arc<RwLock<Core>>);
    // encrypt with public key
    fn encrypt(public_key_file_path: &str, core: Arc<RwLock<Core>>);
    // decrypt with private key
    fn decrypt(private_key_file_path: &str, core: Arc<RwLock<Core>>);
    // list keys
    fn list_keys(key_path: &str, core: Arc<RwLock<Core>>);
    // delete key
    fn delete_key(key_name: &str, core: Arc<RwLock<Core>>);
}

// OpenPGP Key
impl Key for KeyPair {
    type PublicKey = pgp::SignedPublicKey;
    type PrivateKey = pgp::SignedSecretKey;

    fn generate_key(core: Arc<RwLock<Core>>) {
        let _ = generate_key(core);
    }

    fn generate_key_full(primary_id: &str, key_name: &str, core: Arc<RwLock<Core>>) {
        let _ = generate_key_full(primary_id, key_name, core);
    }

    fn encrypt(public_key_file_path: &str, core: Arc<RwLock<Core>>) {
        let _ = encrypt_blob(public_key_file_path, core);
    }

    fn decrypt(private_key_file_path: &str, core: Arc<RwLock<Core>>) {
        let _ = decrypt_blob(private_key_file_path, core);
    }

    fn list_keys(key_path: &str, core: Arc<RwLock<Core>>) {
        let _ = list_keys(key_path, core);
    }

    fn delete_key(key_name: &str, core: Arc<RwLock<Core>>) {
        let _ = delete_key(key_name, core);
    }
}
// Generate default public key and secret key at /craft/key_files/
pub fn generate_key(core: Arc<RwLock<Core>>) -> Result<(), anyhow::Error> {
    let core = core.read().unwrap();
    println!("Creating key pair, this will take a few seconds...");
    // deafult key pair
    let key_pair =
        generate_key_pair("User <craft@craft.com>").expect("Failed to generate key pair");
    // Generate a public key
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
    let mut req = Request::new("secret/craft");
    req.operation = Operation::Write;
    req.body = Some(kv_data);
    core.handle_request(&mut req).unwrap();
    Ok(())
}

// Generate full key with pubkey, seckey, primary id.
// Arguments: primary_id, as &str, it should be written as "User <example@example.com>"; key_name, git-craft will keep ur key file as key_namepub.asc
pub fn generate_key_full(
    primary_id: &str,
    key_name: &str,
    core: Arc<RwLock<Core>>,
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

    let mut req = Request::new(&format!("secret/{}", key_name));
    req.operation = Operation::Write;
    req.body = Some(kv_data);
    core.handle_request(&mut req).unwrap();

    Ok(key_pair)
}
// A blob encrypt function,it can encrypt blob.data
// Argument: public_key_file_path, public key's file path; I set a default path now.
pub fn encrypt_blob(key_file_path: &str, core: Arc<RwLock<Core>>) -> Result<(), anyhow::Error> {
    // Read blob data from standard input stream
    let mut blob_data = Vec::new();
    std::io::stdin().read_to_end(&mut blob_data).unwrap();
    // Get blob.data as msg to encrypt
    let msg = std::str::from_utf8(&blob_data).expect("Invalid UTF-8 sequence");

    let core = core.read().unwrap();
    let mut req = Request::new(key_file_path);
    req.operation = Operation::Read;
    let resp = core.handle_request(&mut req);
    let body = resp.unwrap().unwrap().body.unwrap();
    let pub_key = body["pub"].as_str().unwrap();

    // Encrypt the contents with the given public key
    let encrypted = encrypt_message(msg, pub_key).expect("Failed to encrypt message");
    // Print it, git will get encrypted data
    print!("{}", &encrypted);
    Ok(())
}

// A blob decrypt function,it can decrypt blob.data encrypted by encrypted_blob()
// Arguments: secret_key_file_path; I set a default one now.
pub fn decrypt_blob(key_file_path: &str, core: Arc<RwLock<Core>>) -> Result<(), anyhow::Error> {
    // Read blob.data from standard input stream
    let mut blob_data = Vec::new();
    std::io::stdin().read_to_end(&mut blob_data).unwrap();
    // Set a encrypt_msg to get &str
    let encrypted_msg = std::str::from_utf8(&blob_data).expect("Invalid UTF-8 sequence");

    let core = core.read().unwrap();
    let mut req = Request::new(key_file_path);
    req.operation = Operation::Read;
    let resp = core.handle_request(&mut req);
    let body = resp.unwrap().unwrap().body.unwrap();
    let sec_key = body["sec"].as_str().unwrap();

    // Decrypt contents with the given secret key
    let decrypted_msg = decrypt_message(encrypted_msg, sec_key).expect("Failed to decrypt message");
    // Print decrypted contents, then git will write decrypted contents to origin file
    print!("{}", &decrypted_msg);
    Ok(())
}
