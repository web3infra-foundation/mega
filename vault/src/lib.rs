use secp256k1::{Keypair, Secp256k1};

use crate::vault::{read_secret, write_secret};

pub mod nostr;
pub mod pki;
pub mod vault;

/// Initialize the Nostr ID if it's not found.
/// - return: `(Nostr ID, secret_key)`
/// - You can get `Public Key` by just `base58::decode(nostr)`
pub fn init() -> (String, String) {
    let mut id = read_secret("id").unwrap();
    if id.is_none() {
        println!("Nostr ID not found, generating new one...");
        let (nostr, (secret_key, _)) = nostr::generate_nostr_id();
        let data = serde_json::json!({
            "nostr": nostr,
            "secret_key": secret_key.display_secret().to_string(),
        })
        .as_object()
        .unwrap()
        .clone();
        write_secret("id", Some(data)).unwrap_or_else(|e| {
            panic!("Failed to write Nostr ID: {:?}", e);
        });
        id = read_secret("id").unwrap();
    }
    let id_data = id.unwrap().data.unwrap();
    (
        id_data["nostr"].as_str().unwrap().to_string(),
        id_data["secret_key"].as_str().unwrap().to_string(),
    )
}

pub fn get_peerid() -> String {
    let (id, _sk) = init();
    id
}

pub fn get_keypair() -> Keypair {
    let (_, sk) = init();
    let secp = Secp256k1::new();
    secp256k1::Keypair::from_seckey_str(&secp, &sk).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let id = init();
        println!("Nostr ID: {:?}", id.0);
        println!("Secret Key: {:?}", id.1); // private key
    }
}
