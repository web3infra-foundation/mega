# Git-craft v0.1.0

Git-craft is a extension for git, it can encrypt the content when submitting code in Git, and rewrite the content of the Blob object and code decryption when reading the Blob content, with filter.

## Prepare
1. cd mega/craft
2. modify the key file path to match your project
3. cargo build --release

## Usage

1. git-craft generate-key
  - git-craft will default generate a public key to /key_files/pub.asc and a secret key to /key_files/sec.asc
2. git-craft generate-key-full [primary_id] [key_name]
  - git-craft will generate key with primary id and key name you entered to default file path
3. git-craft encrypt [public_key_path]
  - git-craft will get the file content and encrypt it, it should be used without public key path now, because I set a default key path
4. git-craft decrypt [secret_key_path]
  - git-craft will decrypt blob data read from git's standard input stream, it should be used without secret key path now, because I set a default key path
5. git-craft list-keys [Option<key_path>]
  - git-craft will list keys name, key's fingerprint and id, it should be used without key path now, because I set a default key path
6. git-craft delete-key [key_name] [Option<key_path>]
  - git-craft will show you what keys you have now, then remove keys by key name you entered, it should be used without key path now, because I set a default key path      
 


## About Filter
  
1. edit .git/config "../craft/key_files/sec.asc" is a default key, you can use another key.
- [filter "crypt"]
	      smudge = /root/mega/target/release/git-craft decrypt ../craft/key_files/sec.asc
        clean = /root/mega/target/release/git-craft encrypt ../craft/key_files/pub.asc
2. edit .gitattributes
- file_need_crypted filter=crypt -text
	1. *.txt filter=crypt -text (it will use filter crypt at all txt file in this git dir)
3. must be used arguments
  1. Two commands below are used when you dont need crypt. 
   - git -c filter.crypt.smudge=noop <option>
   - git -c filter.crypt.clean=noop <option>

## WARNING

1. do not upload your key file to github, it must be saved at local file. I already add it in .gitignore.	
 

