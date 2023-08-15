use anyhow::{Context, Result, Ok};
//use git::internal::object::blob::Blob;
use pgp::{composed, composed::signed_key::*, crypto::{self, sym::SymmetricKeyAlgorithm}, types::{SecretKeyTrait, KeyTrait}, Deserializable, Message};
use rand::prelude::*;
use smallvec::*;
use std::{io::Cursor, path::Path};
//use directories::ProjectDirs;
//use color_eyre::eyre::Result;

// Set some default file paths
// Default public key file path
#[allow(unused)]
const PUBLIC_KEY_FILE: &str= "../craft/key_files/pub.asc";
// Default secret key file path
#[allow(unused)]
const SECRET_KEY_FILE: &str= "../craft/key_files/sec.asc";
// Encrypt function use this file to save encrypted message
const MSG_FILE_NAME:  &str= "../mega/craft/src/encrypted_message.txt";



pub struct KeyPair {
    pub secret_key: pgp::SignedSecretKey,
    pub public_key: pgp::SignedPublicKey,
}

// Generate key pair function
// Arguments: primary_user_id, it should input as "User <example@example.com>"
// Return: KeyPair, it has a signed secret key and a signed public key
pub fn generate_key_pair(primary_user_id: &str) -> Result<KeyPair, anyhow::Error> {

    // Set key_params with primary user id, Rsa with 2048 bites, AES with 256 bit key
    let mut key_params = composed::key::SecretKeyParamsBuilder::default();
    key_params
        .key_type(composed::KeyType::Rsa(2048))
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

// Encrypt function
// Arguments: msg, contents need to encrypt; pubkey_str, public key as &str
// Return: encrypted contents
pub fn encrypt(msg: &str, pubkey_str: &str) -> Result<String, anyhow::Error> {
    let (pubkey, _) = SignedPublicKey::from_string(pubkey_str)?;
    // Requires a file name as the first arg, in this case I pass "none", as it's not used
    let msg = composed::message::Message::new_literal("none", msg);
    // Encrypt
    let mut rng = StdRng::from_entropy();
    let new_msg = msg.encrypt_to_keys(
        &mut rng,
        crypto::sym::SymmetricKeyAlgorithm::AES128,
        &[&pubkey],
    )?;
    Ok(new_msg.to_armored_string(None)?)
}

// Decrypt encrypted contents
// Arguments: armored, encrypted contents; seckey, secket key
pub fn decrypt(armored: &str, seckey: &SignedSecretKey) -> Result<String, anyhow::Error> {
    // Get encrypted contents
    let buf = Cursor::new(armored);
    let (msg, _) = composed::message::Message::from_armor_single(buf)
        .context("Failed to convert &str to armored message")?;
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
        .decrypt(|| String::from(""), &[&seckey])
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

// List keys and show their fingerprint, key id
// Argument: key_path, key file path, I use a default file path in main.rs 
// Return: public key and its name, secret key and its name
#[allow(unused)]
pub fn list_keys(key_path: &str)->Result<String>{
    // Create a vector to store and output strings 
    let mut output =Vec::new();
    // Get an itreator over the key files in key file directory
    let files = std::fs::read_dir(key_path).context("Trying to read key file directory")?;
    for file in files{
        // Get file path
        let file_path = file.context("Trying to get the key file path")?.path();
        // Get file name
        let file_name = file_path.file_name().context("Trying to get the key file name")?.to_string_lossy();
        // If the file name ends with .asc, get a key file
        if file_name.ends_with(".asc") {
            // Read the key file content as a string 
            let key = std::fs::read_to_string(&file_path).context(format!("Trying to load key from {}", file_name))?;
            // Check this key is a public key or a secret key
            if key.contains("PUBLIC KEY"){
                // Parse it as a public key
                let (pubkey, _) = SignedPublicKey::from_string(key.as_str())?;
                // Get the fingerprint bytes of the pubkey
                let fingerprint = pubkey.fingerprint();
                // Get the key id of the pubkey
                let  key_id= pubkey.key_id();
                // Print the fingerprint and key id of public key
                println!("The {} public key's fingerprint is: {:?}", file_name, fingerprint);
                println!("The {} public key's key_id is: {:?}", file_name, key_id);
                // Format public key information as a string and push it to output vector
                output.push(format!("Public key: {:?}\nFile name: {}\n", pubkey, file_name));
            }
            else if key.contains("PRIVATE KEY") {
                // Parse it as a secret key
                let (seckey,_) = SignedSecretKey::from_string(key.as_str())?;
                // Get fingerprint of the seckey
                let fingerprint =seckey.fingerprint();
                // Get the key id of seckey
                let key_id =seckey.key_id();
                // Print the fingerprint and key id of secret key
                println!("The {} secret key's fingerprint is: {:?}", file_name, fingerprint);
                println!("The {} secret key's key id is: {:?}", file_name, key_id);
                // format secret key information  as a string and push it to output vector
                output.push(format!("Secret key: {:?}\nFile name: {}\n", seckey, file_name))
            }
            else {
                // The file is not a vaild key file, skip it continue
            }
        }
        else {
            // The file is not a .asc file, skip it continue
        } 

    }
    // Return the output as Ok result
    Ok(output.join("\n"))
}

// Delete key function, it list keys first, then delete keys you input, 
// Considering the public key and secret key should be used  together, it will be deleted together 
// Arguments: key_path, default one is "/mega/craft/key_files"; key_name, key's name you want delete
#[allow(unused)]
pub fn delete_key(key_name: &str, key_path: &str)-> Result<(),anyhow::Error>{
   // ############################################WARNING############################################# 
   // git-craft v0.1.0 use crate pgp, and it will change crypt crate to crate Tongsuo at next version,
   // However, I dont know about Tongsuo at all, so I just set default OpenPGP key as files,
   // file path: "/mega/craft/key_files/pub.asc" "/mega/craft/key_files/sec.asc"
   // If they are not exist, u can run basic generate-key to generate them, 
   // or u can use generate-key-full to generate at other key file.
   // ##############################################OVER##############################################  
    list_keys(key_path);
    let key_file_path = Path::new(key_path);
    let pubkey_file = key_file_path.join(format!("{}pub.asc", key_name));
    std::fs::remove_file(pubkey_file).expect("Unable to remove public key file");
    println!("Public key {} deleted successfully", key_name);
    let seckey_file = key_file_path.join(format!("{}sec.asc", key_name));
    std::fs::remove_file(seckey_file).expect("Unable to remove secret key file");
    println!("Secret key {} deleted successfully", key_name);
    Ok(())
}
