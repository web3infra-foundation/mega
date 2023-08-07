use std::{io::Cursor, fs};

use anyhow::{Context, Ok};
use pgp_key::{generate_key_pair,encrypt_message, decrypt_message};
use git::{internal::{object::{blob::Blob, ObjectT}, zlib::stream::inflate::ReadBoxed, ObjectType}, utils};

use crate::pgp_key;

//A basic generate-key function,it can make a public key and a secret key, not finished but it can be used now
//TODO:generate-key -full, it should can generate keys with pubkey, seckey, fingerprint and password
pub fn generate_key(){
    println!("Creating key pair, this will take a few seconds...");
            let key_pair = generate_key_pair().expect("Failed to generate key pair");
            //Generate a public key
            let pub_key = key_pair
                .public_key
                .to_armored_string(None)
                .expect("Failed to convert public key to armored ASCII string");
            //Write public key to pub.asc,it will replace the old public key
            _=std::fs::write( "../craft/key_files/pub.asc",pub_key).context("Writing public key to file");
            //Generate a secret key
            let sec_key = key_pair
                .secret_key
                .to_armored_string(None)
                .expect("Failed to convert secret key to armored ASCII string");
            //Write secret key and save it to sec.asc,same as public key
            _=std::fs::write( "../craft/key_files/sec.asc",sec_key).context("Writing secret key to file");
}

//A blob encrypt function,it can encrypt blob.data
pub fn encrypt_blob(blob_path:&str)-> Result<(),anyhow::Error>{
            //Create blob object to get blob
            let content = fs::read_to_string(blob_path)?;
            let t_test = Cursor::new(utils::compress_zlib(content.as_bytes()).unwrap());
            let mut deco = ReadBoxed::new(t_test, ObjectType::Blob, content.len());
            //Set a mut blob  to encrpyt it
            let mut blob = Blob::new_from_read(&mut deco, content.len());
            //let data = blob.data;
            //Get blob.data as msg to encrypt
            let msg = std::str::from_utf8(&blob.data).expect("Invalid UTF-8 sequence");
            //println!("message:{}",msg);
            // Encrypt the contents with the public key
            let encrypted = encrypt_message(msg, "../craft/key_files/pub.asc").expect("Failed to encrypt message");
            //Print it to check whether it was encrypted
            //println!("Encrypted: {}", encrypted);
            //Make encrypted message to blob.data and save it to original blob
            let encrypted_data = encrypted.as_bytes().to_vec();
            blob.data = encrypted_data;
            //Write encrypted blob
            std::fs::write(blob_path,&blob.data).unwrap_or_else(|e| {
                panic!("Write failed: {}", e);
            });
            Ok(())
}

//A blob decrypt function,it can decrypt blob.data encrypted by encrypted_blob()
pub fn decrypt_blob(blob_path:&str) -> Result<(),anyhow::Error>{
            // Get the encrypted file contents and the secret key from file
            let content = std::fs::read_to_string(blob_path)?;
            let t_test = Cursor::new(utils::compress_zlib(content.as_bytes()).unwrap());
            let mut deco = ReadBoxed::new(t_test, ObjectType::Blob, content.len());
            //Set a mut blob to encrypt it
            let mut blob = Blob::new_from_read(&mut deco, content.len());
            //Get blob.data as msg to encrypt
            let encrypted_msg = std::str::from_utf8(&blob.data).expect("Invalid UTF-8 sequence");
            //println!("encrypted_message:{}",encrypted_msg);
            // Decrypt the message with the secret key
            let decrypted_msg = decrypt_message(encrypted_msg,"../craft/key_files/sec.asc" ).expect("Failed to decrypt message");
            //Print decrypted message
            //println!("Decrypted: {}", &decrypted_msg);
            //Make decrypted_data to blob.data;
            let decrypted_data = decrypted_msg.as_bytes().to_vec();
            blob.data = decrypted_data;
            //Write decrypted file 
            std::fs::write(blob_path,&blob.data).unwrap_or_else(|e| {
                panic!("Write failed: {}", e);
            });
            Ok(())
}