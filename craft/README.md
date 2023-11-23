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

4. Init RustyVault Core and Generate a default key
   ```bash
   $ git-craft vault init
   ```

5. Edit `.git/config` to add `craft` filter in your repository.

   ```bash
   $ vim .git/config
   ```

   ```ini
   [filter "craft"]
       smudge = git-craft vault decrypt -k secret/craft
       clean = git-craft vault encrypt -k secret/craft
   ```

6. Edit `.gitattributes` to add `craft` filter for files you want to encrypt.

   ```bash
   $ vim .gitattributes
   ```

   ```ini
   *.rs filter=craft -text
   ```

## Encryption and Decryption of Source Code


### Usage

1. `git-craft vault new-key [primary_id] [key_path]`
  - git-craft will generate key with primary id and key name you entered to default file path
2. `git-craft vault encrypt [key_path]`
  - git-craft will get the file content and encrypt it, it should be used without public key path now, because I set a default key path
3. `git-craft vault decrypt [key_path]`
  - git-craft will decrypt blob data read from git's standard input stream, it should be used without secret key path now, because I set a default key path
4. `git-craft vault list`
  - git-craft will list keys name, key's fingerprint and id, it should be used without key path now, because I set a default key path
5. `git-craft vault delete [key_path]`
  - git-craft will show you what keys you have now, then remove keys by key name you entered, it should be used without key path now, because I set a default key path

