#!/bin/sh

# user must set the HUB_HOST
if [ -z "$HUB_HOST" ]; then 
  echo "HUB_HOST is not set"
  exit 1
fi

export MEGA_BASE_DIR="/opt/mega"
CONFIG_FILE="$MEGA_BASE_DIR/etc/config.toml"

if [ -f "$CONFIG_FILE" ]; then
    echo "Using config file: $CONFIG_FILE"
    exec /usr/local/bin/aries -c "$CONFIG_FILE" --host 0.0.0.0 --hub-host $HUB_HOST
else
    exec /usr/local/bin/aries --host 0.0.0.0 --hub-host $HUB_HOST
fi