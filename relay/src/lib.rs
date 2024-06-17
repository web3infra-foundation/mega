use crate::vault::{read_secret, write_secret};

pub mod pki;
pub mod vault;
pub mod nostr;

pub fn init() {
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
    println!("Nostr ID: {:?}", id.unwrap().data.unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
    }
}