use std::{io::{Cursor, Read}, fs, path::Path};

use anyhow::{Context, Ok};
use pgp_key::{generate_key_pair,encrypt_message, decrypt_message};
use git::{internal::{object::{blob::Blob, ObjectT}, zlib::stream::inflate::ReadBoxed, ObjectType}, utils};

use crate::pgp_key::{self, KeyPair};

// Default key file path 
const KEY_FILE_PATH:  &str= "../mega/craft/key_files";

// Generate default public key and secret key at /craft/key_files/ 
pub fn generate_key() -> Result<(), anyhow::Error>{
    println!("Creating key pair, this will take a few seconds...");
    // Create a dir if it is not exist.
    std::fs::create_dir_all(KEY_FILE_PATH)?; 
            let key_pair = generate_key_pair("User <phyknife@phyknife.com>").expect("Failed to generate key pair");
            // Generate a public key
            let pub_key = key_pair
                .public_key
                .to_armored_string(None)
                .expect("Failed to convert public key to armored ASCII string");
            // Write public key to pub.asc,it will replace the old public key
            _=std::fs::write( "../mega/craft/key_files/pub.asc",pub_key).context("Writing public key to file");
            // Generate a secret key
            let sec_key = key_pair
                .secret_key
                .to_armored_string(None)
                .expect("Failed to convert secret key to armored ASCII string");
            // Write secret key to sec.asc, it will replace the old secret key
            _=std::fs::write( "../mega/craft/key_files/sec.asc",sec_key).context("Writing secret key to file");
            Ok(())
}

// Generate full key with pubkey, seckey, primary id.
// Arguments: primary_id, as &str, it should be written as "User <example@example.com>"; key_name, git-craft will keep ur key file as key_name_pub.asc 
pub fn generate_key_full(primary_id:&str, key_name:&str)-> Result<KeyPair, anyhow::Error> {
    println!("Creating key pair, this will take a few seconds...");
    let key_file_path= Path::new("../mega/craft/key_files");
    // Create a dir if it is not exist.
    std::fs::create_dir_all(key_file_path)?; 
        // Give primary id to generate_key_pair to generate key with a non-default key id
        let key_pair=generate_key_pair(primary_id).expect("Failed to generate full key pair");
        // Generate a public key with primary id
        let pub_key = key_pair
            .public_key
            .to_armored_string(None)
            .expect("Failed to convert public key to armored ASCII string");
        // Add key_name_pub.asc to key file path
        let pub_key_file_path =key_file_path.join(format!("{}pub.asc", key_name));
        // Write public key to file,it will replace the old same name's public key 
        _=std::fs::write( pub_key_file_path,pub_key).context("Writing public key to file");
        // Generate a secret key
        let sec_key = key_pair
            .secret_key
            .to_armored_string(None)
            .expect("Failed to convert secret key to armored ASCII string");
        // Add key_name_sec.asc to key file path
        let sec_key_file_path = key_file_path.join(format!("{}sec.asc", key_name)); 
        // Write secret key to file, it will replace the old same name's secret key.
        _=std::fs::write( sec_key_file_path,sec_key).context("Writing secret key to file");    
            
    Ok(key_pair)
}
// A blob encrypt function,it can encrypt blob.data
// Argument: blob_path, contents file path; public_key_file_path, public key's file path; I set a default path now.  
pub fn encrypt_blob(blob_path:&str, public_key_file_path: &str)-> Result<(),anyhow::Error>{
            // Create blob object to get blob
            // Read from content path
            let content = fs::read_to_string(blob_path)?;
            let t_test = Cursor::new(utils::compress_zlib(content.as_bytes()).unwrap());
            let mut deco = ReadBoxed::new(t_test, ObjectType::Blob, content.len());
            // Set a mut blob to encrpyt it
            let mut blob = Blob::new_from_read(&mut deco, content.len());
            // Get blob.data as msg to encrypt
            let msg = std::str::from_utf8(&blob.data).expect("Invalid UTF-8 sequence");
            // Encrypt the contents with the public key 
            let encrypted = encrypt_message(msg, public_key_file_path).expect("Failed to encrypt message");
            // Make encrypted message to blob.data and save it to original blob
            let encrypted_data = encrypted.as_bytes().to_vec();
            blob.data = encrypted_data;
            // Write encrypted blob to file
            std::fs::write(blob_path,&blob.data).unwrap_or_else(|e| {
                panic!("Write failed: {}", e);
            });
            Ok(())
}

// A blob decrypt function,it can decrypt blob.data encrypted by encrypted_blob()
// Arguments: secret_key_file_path; I set a default one now. 
pub fn decrypt_blob(secret_key_file_path:&str) -> Result<(),anyhow::Error>{
            // Read blob.data from standard input stream
            let mut blob_data = Vec::new();
            std::io::stdin().read_to_end(&mut blob_data).unwrap();
            // Set a encrypt_msg to get &str 
            let encrypted_msg = std::str::from_utf8(&blob_data).expect("Invalid UTF-8 sequence");
            // Decrypt contents with the secret key
            let decrypted_msg = decrypt_message(encrypted_msg, secret_key_file_path).expect("Failed to decrypt message");
            // Print decrypted contents, then git will write decrypted contents to origin file
            print!("{}", &decrypted_msg);
            Ok(())
}
