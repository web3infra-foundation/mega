#!/bin/sh

export MEGA_BASE_DIR="/opt/mega"
CONFIG_FILE="$MEGA_BASE_DIR/etc/config.toml"

# check if config file exists
if [ -f "$CONFIG_FILE" ]; then
    echo "Using config file: $CONFIG_FILE"
    exec /usr/local/bin/mono -c "$CONFIG_FILE"  service multi http ssh --host 0.0.0.0 --port 8000 --ssh-port 9000
else
    exec /usr/local/bin/mono service multi http ssh --host 0.0.0.0 --ssh-port 22
fi