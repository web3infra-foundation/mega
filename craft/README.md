# Git-craft v0.1.0

Git-craft is a extension for git, it can encrypt the content when submitting code in Git, and rewrite the content of the Blob object and code decryption when reading the Blob content, with filter.

## Prepare
 cd mega/craft
 cargo build --release

## Usage

1. git-craft generate-key
  - git-craft will default generate a public key to /key_files/pub.asc and a secret key to /key_files/sec.asc
2. git-craft generate-key-full [primary_id] [key_name]
  - git-craft will generate key with primary id and key name you entered to default file path
3. git-craft encrypt [file_path] [public_key_path]
  - git-craft will get the file content and encrypt it, it should be used without public key path now, because I set a default key path
4. git-craft decrypt [file_path] [secret_key_path]
  - git-craft will decrypt blob data read from git's standard input stream, it should be used without secret key path now, because I set a default key path
5. git-craft list-keys [key_path]
  - git-craft will list keys name, key's fingerprint and id, it should be used without key path now, because I set a default key path
6. git-craft delete-key [key_name] [key_path]
  - git-craft will show you what keys you have now, then remove keys by key name you entered, it should be used without key path now, because I set a default key path      
 


## About Filter
  
1. edit .git/config
- [filter "crypt"]
	      smudge = /root/mega/target/release/git-craft decrypt 
        clean = /root/mega/target/release/git-craft encrypt "%f"
2. edit .gitattributes
- file_need_crypted filter=crypt -text
3. must be used arguments
  1. Two commands below are used when you dont need crypt. 
   - git -c filter.crypt.smudge=noop <option>
   - git -c filter.crypt.clean=noop <option>
	
 

