use std::io::Cursor;

use anyhow::{Context, Result};
use pgp::{
    crypto::sym::SymmetricKeyAlgorithm, types::SecretKeyTrait, Deserializable, KeyType, Message,
    SecretKeyParamsBuilder, SignedPublicKey, SignedSecretKey,
};
use rand::prelude::*;
use smallvec::*;

pub struct KeyPair {
    pub secret_key: pgp::SignedSecretKey,
    pub public_key: pgp::SignedPublicKey,
}

// Generate key pair function
// Arguments: primary_user_id, it should input as "User <example@example.com>"
// Return: KeyPair, it has a signed secret key and a signed public key
pub fn generate_key_pair(primary_user_id: &str) -> Result<KeyPair, anyhow::Error> {
    // Set key_params with primary user id, Rsa with 2048 bites, symmetric algorithms key prefer to use is AES with 256 bit
    let mut key_params = SecretKeyParamsBuilder::default();
    key_params
        .key_type(KeyType::Rsa(2048))
        .can_certify(false)
        .can_sign(true)
        .primary_user_id(primary_user_id.into())
        .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256]);

    // build a new SecretKeyParams
    let secret_key_params = key_params
        .build()
        .expect("Must be able to create secret key params");

    // generate a secret key
    let secret_key = secret_key_params
        .generate()
        .expect("Failed to generate a plain key.");

    // new a password to sign the secret key
    let passwd_fn = String::new;
    let signed_secret_key = secret_key
        .sign(passwd_fn)
        .expect("Secret Key must be able to sign its own metadata");

    // generate a public key by the signed secret key
    let public_key = signed_secret_key.public_key();
    // sign public key
    let signed_public_key = public_key
        .sign(&signed_secret_key, passwd_fn)
        .expect("Public key must be able to sign its own metadata");

    let key_pair = KeyPair {
        secret_key: signed_secret_key,
        public_key: signed_public_key,
    };

    Ok(key_pair)
}

// Encrypt function
// Arguments: msg, contents need to encrypt; pubkey_str, public key as &str
// Return: encrypted contents
pub fn encrypt(msg: &str, pubkey_str: &str) -> Result<String, anyhow::Error> {
    let (pubkey, _) = SignedPublicKey::from_string(pubkey_str)?;
    // Requires a file name as the first arg, in this case I pass "none", as it's not used
    let msg = Message::new_literal("none", msg);
    // Encrypt
    let mut rng = StdRng::from_entropy();

    let new_msg = msg.encrypt_to_keys(&mut rng, SymmetricKeyAlgorithm::AES128, &[&pubkey])?;
    Ok(new_msg.to_armored_string(None)?)
}

// Decrypt encrypted contents
// Arguments: armored, encrypted contents; seckey, secret key
pub fn decrypt(armored: &str, seckey: &SignedSecretKey) -> Result<String, anyhow::Error> {
    // Get encrypted contents
    let buf = Cursor::new(armored);
    let (msg, _) =
        Message::from_armor_single(buf).context("Failed to convert &str to armored message")?;
    // Set a decryptor
    let (decryptor, _) = msg
        .decrypt(|| String::from(""), &[seckey])
        .context("Decrypting the message")?;
    // Use decryptor to decrypt encrypted contents
    for msg in decryptor {
        let bytes = msg?.get_content()?.unwrap();
        let clear_text = String::from_utf8(bytes)?;
        if String::len(&clear_text) > 0 {
            return Ok(clear_text);
        }
    }

    Err(anyhow::Error::msg("Failed to find message"))
}

// Encrypt message from file, and write it to a MGS_FILE waiting for decrypt
// Arguments: message, read from file; public key file path
pub fn encrypt_message(msg: &str, pub_key: &str) -> Result<String> {
    let (pub_key, _) = SignedPublicKey::from_string(pub_key)?;
    // Requires a file name as the first arg, in this case I pass "none", as it's not used typically, it's just meta data
    let msg = pgp::Message::new_literal("none", msg);
    // convert data from OpenPGP Message to string
    let armored = generate_armored_string(msg, pub_key)?;

    Ok(armored)
}

// Convert data from OpenPGP Message to String
// Arguments: msg, OpenPGP Message; pk, a signed public key
// Return: string
pub fn generate_armored_string(msg: Message, pk: SignedPublicKey) -> Result<String> {
    let mut rng = StdRng::from_entropy();
    // encrypt the message
    let new_msg = msg.encrypt_to_keys(&mut rng, SymmetricKeyAlgorithm::AES128, &[&pk])?;
    // return encrypted message as string
    Ok(new_msg.to_armored_string(None)?)
}

// Decrypt message from file
// Arguments: armored, encrypted message;v seckey_file, secret key file path
pub fn decrypt_message(armored: &str, seckey: &str) -> Result<String, anyhow::Error> {
    let (seckey, _) = SignedSecretKey::from_string(seckey)?;
    // get encrypted message
    let buf = Cursor::new(armored);
    let (msg, _) = Message::from_armor_single(buf)?;
    // return a decryptor, it can decrypt message with a given key
    let (decryptor, _) = msg
        .decrypt(|| String::from(""), &[&seckey])
        .context("Decrypting the message")?;
    // decrypt message
    for msg in decryptor {
        let bytes = msg?.get_content()?.unwrap();
        let clear = String::from_utf8(bytes)?;
        if String::len(&clear) > 0 {
            return Ok(clear);
        }
    }

    Err(anyhow::Error::msg("Failed to find message"))
}
