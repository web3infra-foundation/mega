use secp256k1::{rand, PublicKey, Secp256k1, SecretKey};

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
    use secp256k1::Message;

    #[test]
    fn test_generate_nostr_id() {
        let (nostr, keypair) = generate_nostr_id();
        println!("nostr: {:?}", nostr);
        println!("keypair: {:?}", keypair);
        let secret_key = keypair.0;
        let public_key = keypair.1;

        let nostr_decode = bs58::decode(&nostr).into_vec().unwrap();
        assert_eq!(nostr_decode, public_key.serialize().to_vec());
        assert_eq!(PublicKey::from_slice(&nostr_decode).unwrap(), public_key);
        // verify
        let secp = Secp256k1::new();
        let message = Message::from_digest_slice(&[0xab; 32]).expect("32 bytes");
        let sig = secp.sign_ecdsa(&message, &secret_key);
        assert_eq!(secp.verify_ecdsa(&message, &sig, &public_key), Ok(()));
    }
}
