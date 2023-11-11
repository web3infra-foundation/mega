# Craft - Git Extension of Mega

Craft is a Git plugin for Mega, a Large File Storage (LFS) client, encryption and decryption of code, and generation of AI/LLM training data or model data, among other functionalities. As an integral part of the Mega service, it is installed locally in the developers' environment and incorporated with the server to enhance developer experiences.

## Quick Started for developing and testing craft on MacOS

1. Install Rust on your MacOS.

   ```bash
   $ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone mega repository and build it.

   ```bash
   $ git clone https://github.com/web3infra-foundation/mega.git
   $ cd mega/craft
   $ cargo build --release
   ```

3. Copy `git-craft` to $PATH.

   ```bash
   $ cp target/release/git-craft /usr/local/bin
   ```

4. Edit `.git/config` to add `craft` filter in your repository.

   ```bash
   $ vim .git/config
   ```

   ```ini
   [filter "craft"]
       smudge = git-craft decrypt /User/eli/.mega/craft/key_files/sec.asc
       clean = git-craft encrypt /User/eli/.mega/craft/key_files/pub.asc
   ```

5. Edit `.gitattributes` to add `craft` filter for files you want to encrypt.

   ```bash
   $ vim .gitattributes
   ```

   ```ini
   *.rs filter=craft -text
   ```

## Encryption and Decryption of Source Code



### Prepare
1. cd mega/craft
2. modify all the key file path, KEY_FILE_PATH, MSG_FILE_NAME and filter to match your project
3. cargo build --release

### Usage

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

### About Filter
  
1. edit .git/config "../craft/key_files/sec.asc" is a default key, you can use another key.
- [filter "crypt"]
	      smudge = ../craft/target/release/git-craft decrypt ../craft/key_files/sec.asc
        clean = ../craft/target/release/git-craft encrypt ../craft/key_files/pub.asc
2. edit .gitattributes
- file_need_crypted filter=crypt -text
- *.txt filter=crypt -text (it will use filter crypt at all txt file in this git dir)
3. must be used arguments
  1. Two commands below are used when you dont need crypt. 
   - git -c filter.crypt.smudge=noop <option>
   - git -c filter.crypt.clean=noop <option>

## WARNING

1. do not upload your key file to github, it must be saved at local file. I already add it in .gitignore.	
 

