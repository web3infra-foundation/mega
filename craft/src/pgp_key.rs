use anyhow::{Context, Result, Ok};
//use git::internal::object::blob::Blob;
use pgp::{composed, composed::signed_key::*, crypto::{self, SymmetricKeyAlgorithm}, types::SecretKeyTrait, Deserializable, Message};
use rand::prelude::*;
use smallvec::*;
use std::io::Cursor;
//use directories::ProjectDirs;
//use color_eyre::eyre::Result;

// While the keys used in this example are unique for each "person", the key password is the same for both
#[allow(unused)]
const PUBLIC_KEY_FILE: &str= "../craft/key_files/pub.asc";
#[allow(unused)]
const SECRET_KEY_FILE: &str= "../craft/key_files/sec.asc";
const MSG_FILE_NAME:  &str= "../craft/src/encrypted_message.txt";
#[allow(unused)]
const SECRET_MSG:  &str= "../craft/src/message.txt";


pub struct KeyPair {
    pub secret_key: pgp::SignedSecretKey,
    pub public_key: pgp::SignedPublicKey,
}

pub fn generate_key_pair() -> Result<KeyPair, anyhow::Error> {
   // let password = &args[2];

    let mut key_params = composed::key::SecretKeyParamsBuilder::default();
    key_params
        .key_type(composed::KeyType::Rsa(2048))
        //.passphrase(Some(password.clone()))
        .can_create_certificates(false)
        .can_sign(true)
        .primary_user_id("User <phyknife@phyknife.com>".into())
        .preferred_symmetric_algorithms(smallvec![crypto::sym::SymmetricKeyAlgorithm::AES256]);

    let secret_key_params = key_params
        .build()
        .expect("Must be able to create secret key params");

    let secret_key = secret_key_params
        .generate()
        .expect("Failed to generate a plain key.");

    let passwd_fn = String::new;
    let signed_secret_key = secret_key
        .sign(passwd_fn)
        .expect("Secret Key must be able to sign its own metadata");

    let public_key = signed_secret_key.public_key();
    let signed_public_key = public_key
        .sign(&signed_secret_key, passwd_fn)
        .expect("Public key must be able to sign its own metadata");

    let key_pair = KeyPair {
        secret_key: signed_secret_key,
        public_key: signed_public_key,
    };

    Ok(key_pair)
}

pub fn encrypt(msg: &str, pubkey_str: &str) -> Result<String, anyhow::Error> {
    let (pubkey, _) = SignedPublicKey::from_string(pubkey_str)?;
    // Requires a file name as the first arg, in this case I pass "none", as it's not used
    let msg = composed::message::Message::new_literal("none", msg);

    let mut rng = StdRng::from_entropy();
    let new_msg = msg.encrypt_to_keys(
        &mut rng,
        crypto::sym::SymmetricKeyAlgorithm::AES128,
        &[&pubkey],
    )?;
    Ok(new_msg.to_armored_string(None)?)
}

pub fn decrypt(armored: &str, seckey: &SignedSecretKey) -> Result<String, anyhow::Error> {
    let buf = Cursor::new(armored);
    let (msg, _) = composed::message::Message::from_armor_single(buf)
        .context("Failed to convert &str to armored message")?;
    let (decryptor, _) = msg
        .decrypt(|| String::from(""), || String::from(""), &[seckey])
        .context("Decrypting the message")?;

    for msg in decryptor {
        let bytes = msg?.get_content()?.unwrap();
        let clear_text = String::from_utf8(bytes)?;
        if String::len(&clear_text) > 0 {
            return Ok(clear_text);
        }
    }

    Err(anyhow::Error::msg("Failed to find message"))
}

#[allow(unused)]
/*pub fn save_key(keys: (String,String)) -> Result<String,anyhow::Error>{
    //let proj_dirs=ProjectDirs::from("mega","database").unwarp();
    
}*/
#[allow(unused)]
pub fn encrypt_message(msg: &str, pubkey_file: &str) -> Result<String> {
    let pubkey = std::fs::read_to_string(pubkey_file)
        .context("Trying to load public key for Person Two from file")?;
    let (pubkey, _) = SignedPublicKey::from_string(pubkey.as_str())?;

    // Requires a file name as the first arg, in this case I pass "none", as it's not used typically, it's just meta data
    let msg = pgp::Message::new_literal("none", msg);

    let armored = generate_armored_string(msg, pubkey)?;
    std::fs::write(MSG_FILE_NAME, &armored).context("Writing encrypted message to file")?;

    Ok(armored)
}


#[allow(unused)]
pub fn generate_armored_string(msg: Message, pk: SignedPublicKey) -> Result<String> {
    let mut rng = StdRng::from_entropy();
    let new_msg = msg.encrypt_to_keys(&mut rng, SymmetricKeyAlgorithm::AES128, &[&pk])?;
    Ok(new_msg.to_armored_string(None)?)
}
#[allow(unused)]
pub fn decrypt_message(armored: &str, seckey_file: &str) -> Result<String> {
    let seckey = std::fs::read_to_string(seckey_file)?;
    let (seckey, _) = SignedSecretKey::from_string(seckey.as_str())?;

    let buf = Cursor::new(armored);
    let (msg, _) = Message::from_armor_single(buf)?;
    let (decryptor, _) = msg
        .decrypt(|| String::from(""), || String::from(""), &[&seckey])
        .context("Decrypting the message")?;

    for msg in decryptor {
        let bytes = msg?.get_content()?.unwrap();
        let clear = String::from_utf8(bytes)?;
        if String::len(&clear) > 0 {
            return Ok(clear);
        }
    }

    Err(anyhow::Error::msg("Failed to find message"))
}

#[allow(unused)]
pub fn list_keys(public_key_file:&str,secret_key_file:&str)->Result<String>{
    //Convert key to string and print it
    //TODO: 
    let pubkey = std::fs::read_to_string(public_key_file)
        .context("Trying to load public key for Person Two from file")?;
    let (pubkey, _) = SignedPublicKey::from_string(pubkey.as_str())?;
    let seckey = std::fs::read_to_string(secret_key_file)
    .context("Trying to load secret key for Person Two from file")?;
    let (seckey, _) = SignedSecretKey::from_string(seckey.as_str())?;
    // Format the public key and secret key information as a string
    let output = format!(
        "Public key: {:?}\nSecret key: {:?}",
        pubkey, seckey
    );
    println!("{}",output);
    // Return the output as an Ok result
    Ok(output)
}
#[allow(unused)]
pub fn delete_key(fingerprint: &str)-> Result<(),anyhow::Error>{
   /* TODO: Parse the fingerprint as a KeyId
    let key_id = KeyId::from_hex(fingerprint)?;
    // Delete the key from the keyring
    delete_key(key_id, "../craft/key_files/pub.asc", "../craft/key_files/sec.asc")?;*/
   println!("Key {} deleted successfully", fingerprint);
   Ok(())
}