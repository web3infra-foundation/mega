use std::{
    io::Read,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use pgp_key::{decrypt_message, encrypt_message};
use rusty_vault::{
    core::Core,
    errors::RvError,
    logical::{Operation, Request, Response},
};
use serde_json::{json, Map, Value};

use crate::vault::{pgp_key, RustyVault};

// the trait and impl for KeyPair is a preparation for crate Tongsuo.
// a trait for key
pub trait KeyOperation {
    /// Generates a key based on the provided parameters.
    ///
    /// # Arguments
    ///
    /// * `key_name` - A string representing the name of the key.
    /// * `key_type` - A string indicating the type of key to generate.
    /// * `key_bits` - The number of bits for the generated key.
    ///
    /// # Returns
    ///
    /// A `Result` containing a map of key-related data on success, or an `anyhow::Error` on failure.
    fn generate_key(
        &self,
        key_name: &str,
        key_type: &str,
        key_bits: u32,
    ) -> Result<Map<String, Value>, anyhow::Error>;

    /// Encrypts data using a specified key.
    ///
    /// # Arguments
    ///
    /// * `key_name` - A string representing the name of the key used for encryption.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an error (`anyhow::Error`) if encryption fails.
    fn encrypt(&self, key_name: &str) -> Result<(), anyhow::Error>;

    // A blob decrypt function,it can decrypt blob.data
    fn decrypt(&self, key_name: &str) -> Result<(), anyhow::Error>;

    // List keys and show their fingerprint, key id
    // Argument: key_path, key file path.
    // Return: public key and its name, secret key and its name
    fn list_keys(&self, key_prefix: &str) -> Result<Vec<String>>;

    // Delete key function, it list keys first, then delete keys you input,
    // Considering the public key and secret key should be used  together, it will be deleted together
    // Arguments: key_path, default one is "/mega/craft/key_files"; key_name, key's name you want delete
    fn delete_key(&self, key_name: &str) -> Result<(), anyhow::Error>;
}

impl KeyOperation for RustyVault {
    fn generate_key(
        &self,
        key_name: &str,
        key_type: &str,
        key_bits: u32,
    ) -> Result<Map<String, Value>, anyhow::Error> {
        tracing::info!("Creating key pair, this will take a few seconds...");

        let req_data = json!({
            "key_name": key_name.to_string(),
            "key_type": key_type.to_string(),
            "key_bits": key_bits,
        })
        .as_object()
        .unwrap()
        .clone();
        tracing::debug!("generate req_data: {:?}", req_data);

        let resp = self.write_request(
            "pki/keys/generate/exported",
            Some(req_data),
        );

        let resp_body = resp.unwrap();
        assert!(resp_body.is_some());
        let data = resp_body.unwrap().data;
        assert!(data.is_some());
        let key_data = data.unwrap();
        Ok(key_data)
    }

    fn encrypt(&self, key_name: &str) -> Result<(), anyhow::Error> {
        // Read blob data from standard input stream
        let mut blob_data = Vec::new();
        std::io::stdin().read_to_end(&mut blob_data).unwrap();
        // Get blob.data as msg to encrypt
        let data = std::str::from_utf8(&blob_data)
            .expect("Invalid UTF-8 sequence")
            .as_bytes();
        let origin_data = hex::encode(data);
        let req_data = json!({
            "key_name": key_name.to_string(),
            "data": origin_data.clone(),
        })
        .as_object()
        .unwrap()
        .clone();
        let resp = self.write_request("pki/keys/encrypt", Some(req_data));

        let resp_body = resp.unwrap();
        assert!(resp_body.is_some());
        let resp_raw_data = resp_body.unwrap().data;
        assert!(resp_raw_data.is_some());
        let resp_data = resp_raw_data.unwrap();
        tracing::debug!("encrypt resp_data: {:?}", resp_data);
        let encrypted_data = resp_data["result"].as_str().unwrap();
        // Print it, git will get encrypted data
        print!("{}", &encrypted_data);
        Ok(())
    }

    fn decrypt(&self, key_name: &str) -> Result<(), anyhow::Error> {
        // Read blob.data from standard input stream
        let mut blob_data = Vec::new();
        std::io::stdin().read_to_end(&mut blob_data).unwrap();
        // Set a encrypt_msg to get &str
        let encrypted_data = std::str::from_utf8(&blob_data).expect("Invalid UTF-8 sequence");

        let req_data = json!({
            "key_name": key_name.to_string(),
            "data": encrypted_data,
        })
        .as_object()
        .unwrap()
        .clone();
        let resp = self.write_request("pki/keys/decrypt", Some(req_data));
        let resp_body = resp.unwrap();
        assert!(resp_body.is_some());
        let resp_raw_data = resp_body.unwrap().data;
        assert!(resp_raw_data.is_some());
        let resp_data = resp_raw_data.unwrap();
        tracing::debug!("decrypt resp_data: {:?}", resp_data);
        let data = hex::decode(resp_data["result"].as_str().unwrap()).unwrap();
        // Print decrypted contents, then git will write decrypted contents to origin file
        print!("{}", String::from_utf8(data).unwrap());
        Ok(())
    }

    fn list_keys(&self, key_prefix: &str) -> Result<Vec<String>> {
        let core = self.core.read().unwrap();
        let mut req = Request::new(key_prefix);
        req.operation = Operation::List;
        req.client_token = self.token.to_string();
        let resp = core.handle_request(&mut req);
        println!("resp:{:?}", resp);
        if let Ok(resp) = resp {
            assert!(resp.is_some());
            let body = resp.unwrap().data.unwrap();
            let keys = body["keys"].as_array().unwrap();
            let keys = keys.iter().map(|x| x.to_string()).collect::<Vec<String>>();
            println!("{:?}", keys);
            Ok(keys)
        } else {
            panic!("list key failed: {}", key_prefix)
        }
    }

    fn delete_key(&self, key_name: &str) -> Result<(), anyhow::Error> {
        let core = self.core.write().unwrap();
        let mut req = Request::new(key_name);
        req.operation = Operation::Delete;
        req.client_token = self.token.to_string();
        assert!(core.handle_request(&mut req).is_ok());
        Ok(())
    }
}

const CA_CERT_PEM: &str = r#"
-----BEGIN CERTIFICATE-----
MIIC/DCCAeSgAwIBAgIBAjANBgkqhkiG9w0BAQsFADASMRAwDgYDVQQDDAdSb290
IENBMCAXDTIwMTIxMjIwMTY1MFoYDzIxMjAxMjEzMjAxNjUwWjANMQswCQYDVQQD
DAJDQTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAJadpD0ASxxfxsvd
j9IxsogVzMSGLFziaYuE9KejU9+R479RifvwfBANO62sNWJ19X//9G5UjwWmkiOz
n1k50DkYsBBA3mJzik6wjt/c58lBIlSEgAgpvDU8ht8w3t20JP9+YqXAeugqFj/W
l9rFQtsvaWSRywjXVlp5fxuEQelNnXcJEKhsKTNExsBUZebo4/J1BWpklWzA9P0l
YW5INvDAAwcF1nzlEf0Y6Eot03IMNyg2MTE4hehxjdgCSci8GYnFirE/ojXqqpAc
ZGh7r2dqWgZUD1Dh+bT2vjrUzj8eTH3GdzI+oljt29102JIUaqj3yzRYkah8FLF9
CLNNsUcCAwEAAaNgMF4wDwYDVR0TAQH/BAUwAwEB/zALBgNVHQ8EBAMCAQYwHQYD
VR0OBBYEFLQRM/HX4l73U54gIhBPhga/H8leMB8GA1UdIwQYMBaAFI71Ja8em2uE
PXyAmslTnE1y96NSMA0GCSqGSIb3DQEBCwUAA4IBAQDacg5HHo+yaApPb6mk/SP8
J3CjQWhRzv91kwsGLnhPgZI4HcspdJgTaznrstiiA1VRjkQ/kwzd29Sftb1kBio0
pAyblmravufRdojfTgkMnFyRSaj4FHuOQq8lnX3gwlKn5hBtEF6Qd+U79MkpMALa
cxPdyJs2tgDOpP1jweubOawqsKlxhAjwgdeX0Qp8iUj4BrY0zg4Q5im0mEKo4hij
49dQQqoWakCejH4QP2+T1urJsRGn9rXk/nkW9daNYaQDyoAPlnhr5oU+pP3+hSec
Ol83n08VZ8BizTSPkG0J66sZGC5jvsf5rX8YHURv0jNxHcG8QVEmyCwPqfDTI4fz
-----END CERTIFICATE-----"#;

const CA_KEY_PEM: &str = r#"
-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCWnaQ9AEscX8bL
3Y/SMbKIFczEhixc4mmLhPSno1PfkeO/UYn78HwQDTutrDVidfV///RuVI8FppIj
s59ZOdA5GLAQQN5ic4pOsI7f3OfJQSJUhIAIKbw1PIbfMN7dtCT/fmKlwHroKhY/
1pfaxULbL2lkkcsI11ZaeX8bhEHpTZ13CRCobCkzRMbAVGXm6OPydQVqZJVswPT9
JWFuSDbwwAMHBdZ85RH9GOhKLdNyDDcoNjExOIXocY3YAknIvBmJxYqxP6I16qqQ
HGRoe69naloGVA9Q4fm09r461M4/Hkx9xncyPqJY7dvddNiSFGqo98s0WJGofBSx
fQizTbFHAgMBAAECggEABdXHpiFbx5aiUgWca81HGGSX0UlNcK/I3QHipJf8SN4T
D7dt/Be+BrUsibbxPoZJY5Mb+iZGgDaK1N1BoChQO9YMBCUvOGs3gYLvlhat2Csw
1Etp1mcfhoR4yS7Qg5BWGpvf4IILgPEYeZKrwWsBAxLcJ2xKjGYjT1ADr6I5F3u+
FYN+bvlXxr07GccfS+UHt04oT0dHwxQzFaJj+yqKWGo2IFtPqtr6Sgoh9a+yFYIi
8a9MigTTt+IyJ55OuC/FHRf1PofprftADFts78k43qxWtrxSrQVdlNXp1lpZOtuR
7gvB/r3a2byDYxCxYVu98tQuOfW909TdDgPmEJjcAQKBgQDHcTYi+zcGKooN3tfK
Oc6hnFXAYTNpYp074NfIYB8i10CwbvWta1FDoi3iRqlQFwg+pu12UefZsj21F+aF
v2eGP33kQ6yiXJQ3j7jam7dY+tZ6xb0dthm+X/INuHp/HbSb1qKFmSO2rmMDQg+e
Crqts9+t5Xk04ewTgpySLZjvRwKBgQDBU85Ls3s8osre5EmVBRd5qBt6ILnjtdoa
UxrrrWopRx2q3HsI41VhKFx0PGs6ia0c6+9GFR6wX/Qevj85DADbzHDA5XEZq98q
8yH4lme2Uj2gOlWqyhDeC/g4S+MsbNoIaUOZbMGg/phyAe20HvtvD7MUhZ/2rkta
U5UjFpouAQKBgQC/+vU+tQ0hTV94vJKBoiWKIX/V4HrprbhmxCdSRVyTYBpv+09X
8J7X+MwsLRKb+p/AF1UreOox/sYxhOEsy7MuYf2f9Zi+7VjrJtis7gmOiF5e7er+
J6UeQSMyG+smY4TQIcptyZy8I59Bqpx36CIMRMJClUqYIgTqPubSOzwkzwKBgENB
9LNBbc5alFmW8kJ10wTwBx8l44Xk7kvaPbNgUV6q7xdSPTuKW1nBwOhvXJ6w5xj4
u/WVw2d4+mT3qucd1e6h4Vg6em6D7M/0Zg0lxk8XrXjg0ozoX5XgdCqhvBboh7IF
bQ8jVvm7mS2QnjHb1X196L9q/YvEd1KlYW0jn+ABAoGBAKwArjjmr3zRhJurujA5
x/+V28hUf8m8P2NxP5ALaDZagdaMfzjGZo3O3wDv33Cds0P5GMGQYnRXDxcZN/2L
/453f0uUObRwFepuv9HzuvPgkTRGpcLFiIHCThiKdyBgPKoq39qjbAyWQcfmW8+S
2k24wuH7oUtLlvf05p4cqfEx
-----END PRIVATE KEY-----"#;

impl RustyVault {
    pub fn mount_pki(&self) {
        // mount pki backend to path: pki/
        let mount_data = json!({
            "type": "pki",
        })
        .as_object()
        .unwrap()
        .clone();

        let resp = self.write_request("sys/mounts/pki/", Some(mount_data));
        assert!(resp.is_ok());

        let ca_pem_bundle = format!("{}{}", CA_CERT_PEM, CA_KEY_PEM);

        let ca_data = json!({
            "pem_bundle": ca_pem_bundle,
        })
        .as_object()
        .unwrap()
        .clone();

        // config ca
        let resp = self.write_request("pki/config/ca", Some(ca_data));
        assert!(resp.is_ok());
    }

    pub fn write_request(
        &self,
        path: &str,
        data: Option<Map<String, Value>>,
    ) -> Result<Option<Response>, RvError> {
        let mut req = Request::new(path);
        req.operation = Operation::Write;
        req.client_token = self.token.to_string();
        req.body = data;

        let core = self.core.read().unwrap();
        let resp = core.handle_request(&mut req);
        tracing::debug!("path: {}, resp: {:?}", path, resp);
        resp
    }

    fn _read_request(&self, path: &str) -> Result<Option<Response>, RvError> {
        let mut req = Request::new(path);
        req.operation = Operation::Read;
        req.client_token = self.token.to_string();
        let core = self.core.read().unwrap();
        let resp = core.handle_request(&mut req);
        tracing::debug!("path: {}, resp: {:?}", path, resp);
        resp
    }
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
