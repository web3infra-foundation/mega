pub mod nostr;
pub mod pki;
pub mod vault;

/// Initialize the Nostr ID if it's not found.
/// - return: `(Nostr ID, secret_key)`
/// - You can get `Public Key` by just `base58::decode(nostr)`
pub async fn init() -> (String, String) {
    use crate::vault::{read_secret, write_secret};

    let mut id = read_secret("id").await.unwrap();
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
        write_secret("id", Some(data)).await.unwrap_or_else(|e| {
            panic!("Failed to write Nostr ID: {:?}", e);
        });
        id = read_secret("id").await.unwrap();
    }
    let id_data = id.unwrap().data.unwrap();
    (
        id_data["nostr"].as_str().unwrap().to_string(),
        id_data["secret_key"].as_str().unwrap().to_string(),
    )
}

pub async fn get_peerid() -> String {
    let (id, _sk) = init().await;
    id
}

pub async fn get_keypair() -> secp256k1::Keypair {
    let (_, sk) = init().await;
    let secp = secp256k1::Secp256k1::new();
    secp256k1::Keypair::from_seckey_str(&secp, &sk).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init() {
        let id = init().await;
        println!("Nostr ID: {:?}", id.0);
        println!("Secret Key: {:?}", id.1); // private key
    }
}
