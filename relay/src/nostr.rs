use secp256k1::{PublicKey, rand, Secp256k1, SecretKey};

pub fn generate_nostr_id() -> (String, (SecretKey, PublicKey)) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    let public_key = secret_key.public_key(&secp);
    let nostr = bs58::encode(public_key.serialize()).into_string();

    (nostr, (secret_key, public_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_nostr_id() {
        let (nostr, keypair) = generate_nostr_id();
        println!("nostr: {:?}", nostr);
        println!("keypair: {:?}", keypair);
    }
}