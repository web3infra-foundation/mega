use anyhow::{Context, Result, Ok};
//use git::internal::object::blob::Blob;
use pgp::{composed, composed::signed_key::*, crypto::{self, SymmetricKeyAlgorithm}, types::{SecretKeyTrait, KeyTrait}, Deserializable, Message};
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
const MSG_FILE_NAME:  &str= "/root/mega/craft/src/encrypted_message.txt";
#[allow(unused)]
const SECRET_MSG:  &str= "../craft/src/message.txt";


pub struct KeyPair {
    pub secret_key: pgp::SignedSecretKey,
    pub public_key: pgp::SignedPublicKey,
}

pub fn generate_key_pair(primary_user_id: &str) -> Result<KeyPair, anyhow::Error> {
   // let password = &args[2];

    let mut key_params = composed::key::SecretKeyParamsBuilder::default();
    key_params
        .key_type(composed::KeyType::Rsa(2048))
        //.passphrase(Some(password.clone()))
        .can_create_certificates(false)
        .can_sign(true)
        .primary_user_id(primary_user_id.into())
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

// Encrypt message from file, using two arguments, message and public key file path
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
pub fn decrypt_message(armored: &str, seckey_file: &str) -> Result<String,anyhow::Error> {
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

// List keys and show their fingerprint
#[allow(unused)]
pub fn list_keys(public_key_file:&str,secret_key_file:&str)->Result<String>{
    //Convert key to string and print it
    //TODO: 
    let pubkey = std::fs::read_to_string(public_key_file)
        .context("Trying to load public key from file")?;
    let (pubkey, _) = SignedPublicKey::from_string(pubkey.as_str())?;
    // Get the fingerprint bytes of the pubkey
    let fingerprint = pubkey.fingerprint();
    // Get the key id of the pubkey
    let  key_id= pubkey.key_id();
    // Print the fingerprint and key id of public key
    println!("The public key's fingerprint is: {:?}", fingerprint);
    println!("The public key's key_id is: {:?}",key_id);
    
    let seckey = std::fs::read_to_string(secret_key_file)
    .context("Trying to load secret key from file")?;
    let (seckey, _) = SignedSecretKey::from_string(seckey.as_str())?;
    // Get fingerprint of the seckey
    let fingerprint =seckey.fingerprint();
    // Get the key id of seckey
    let key_id =seckey.key_id();
    // Print the fingerprint and key id of secret key
    println!("The secret key's fingerprint is: {:?}", fingerprint);
    println!("The secret key's key id is: {:?}", key_id);

    // Format the public key and secret key information as a string
    let output = format!(
        "Public key: {:?}\nSecret key: {:?}",
        pubkey, seckey
    );
    // Return the output as an Ok result
    Ok(output)
}

//This function is directly delete file, it should be updated with another idea.
#[allow(unused)]
pub fn delete_key(public_key_file:&str, secret_key_file:&str)-> Result<String,anyhow::Error>{
   // TODO: I suppose the function should set three option arguments(for delete key with only key id or only fingerprint),
   // arguments: key_type to find public key/secret key to delete, key id and fingerprint to delete keys from keyring, 
   // or other better and easier data structure to impl, then saving the keyring
   // but git-craft v0.1.0 will change crypt crate from crate pgp to another crate Tongsuo at next version
   // and I dont know about Tongsuo at all, so I just set default OpenPGP key as files,
   // their file path is "craft/key_files/pub.asc" "craf/key_files/sec.asc"
   // If they are not exist, u can run basic generate-key to generate them, 
   // or u can use generate-key full to generate at other file path, but please remember the file path u writed.  
    list_keys(public_key_file, secret_key_file);
    let pubkey = std::fs::read_to_string(public_key_file)
        .context("Trying to load public key from file")?;
    let (pubkey, _) = SignedPublicKey::from_string(pubkey.as_str())?;
    // Get the fingerprint bytes of the pubkey
    let fingerprint = pubkey.fingerprint();
    // Get the key id of the pubkey
    let  pubkey_id= pubkey.key_id();
    std::fs::write(public_key_file, "").context("Delete public key from file");
    println!("Key {:?} deleted successfully",pubkey_id);
    let seckey =std::fs::read_to_string(secret_key_file)
        .context("Trying to load secret key from file")?;
    let (seckey,_)=SignedSecretKey::from_string(seckey.as_str())?;
    let seckey_id =seckey.key_id();
    std::fs::write(secret_key_file, "").context("Delete seccret key from file");
    println!("Key {:?} deleted successfully",seckey_id); 
    let output =format!(
        "Key {:?} and {:?} deleted successfully", pubkey_id, seckey_id
    );
    Ok(output)
}
