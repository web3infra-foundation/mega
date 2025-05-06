#!/bin/sh
#!/bin/bash

gateway_ip=$(ip route | grep default | awk '{print $3}')

if [ -z "$gateway_ip" ]; then
    echo "Unable to find the default gateway."
    exit 1
fi

mkdir -pv /opt/megadir/mount
mkdir -pv /opt/megadir/store

# TODO: pass config file as argument

# delete the old config
if [ -d ~/.cargo/bin/scorpio.toml ]; then
    rm -rf ~/.cargo/bin/scorpio.toml
fi
if [ -d ~/.cargo/bin/config.toml ]; then
    rm -rf ~/.cargo/bin/config.toml
fi

cat <<EOF > ~/.cargo/bin/scorpio.toml
git_author = "MEGA"
git_email = "admin@mega.org"
# monorepo path
workspace = "/opt/megadir/mount"
# local storage path
store_path = "/opt/megadir/store"
base_url = "http://${gateway}:8000"
config_file = "config.toml"
file_blob_endpoint = "http://$gateway_ip:8000/api/v1/file/blob"
lfs_url = "http://$gateway_ip:8000"
EOF

cat <<EOF > ~/.cargo/bin/config.toml
works = []
EOF

echo "Configuration files created successfully with the gateway IP: $gateway_ip"

exec /root/.cargo/bin/scorpio