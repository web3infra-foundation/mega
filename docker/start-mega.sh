#!/bin/sh

export MEGA_BASE_DIR="/opt/mega"
CONFIG_FILE="$MEGA_BASE_DIR/etc/config.toml"

if [ -f "$CONFIG_FILE" ]; then
    echo "Using config file: $CONFIG_FILE"
    exec /usr/local/bin/mega -c "$CONFIG_FILE" service multi http ssh --host 0.0.0.0 --bootstrap-node http://aries-engine:8001
else
    exec /usr/local/bin/mega service multi http ssh --host 0.0.0.0 --bootstrap-node aries-engine:8001
fi