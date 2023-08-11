###How to use git-craft v0.1.0

###prepare
 cd mega/craft
 cargo build --release

###1.generate-key
 git-craft generate-key
  By this, git-craft will generate a public key to /key_files/pub.asc and a secret key to /key_files/sec.asc
###2.encrypt
 git-craft encrypt "file_path"
  By this, git-craft will get the file content from src/message.txt, encrypt it and write encrypted content to src/encrypted_message.txt
###3.decrypt
 git-craft decrypt 
  By this, git-craft will get the encrypted file content from src/encrypted_message.txt, decrypt and show it 


###About Filter
  
###1.edit .git/config
[filter "crypt"]
	smudge = /root/mega/target/release/git-craft decrypt 
        clean = /root/mega/target/release/git-craft encrypt "%f"
###2.edit .gitattributes
   file_need_crypted filter=crypt
###3.must be used arguments
   Two commands below are used when you dont need crypt. 
   git -c filter.crypt.smudge=noop <option>
   git -c filter.crypt.clean=noop <option>
	
 

