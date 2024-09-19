# in .cargo/config
[target.x86_64-unknown-linux-gnu]
runner = 'sudo -E'

# in /etc/fuse.conf
enable user_allow_other

# dependencies
sudo apt install libfuse-dev
sudo apt install librust-openssl-dev