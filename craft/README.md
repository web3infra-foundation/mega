How to use git-craft v0.1.0


1.generate-key
git-craft generate-key
By this, git-craft will generate a public key to /key_files/pub.asc and a secret key to /key_files/sec.asc
2.encrypt
git-craft encrypt
By this, git-craft will get the file content from src/message.txt, encrypt it and write encrypted content to src/encrypted_message.txt
3.decrypt
git-craft decrypt
By this, git-craft will get the encrypted file content from src/encrypted_message.txt, decrypt and show it 

