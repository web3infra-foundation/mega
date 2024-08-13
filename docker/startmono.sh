#!/bin/bash

CONFIG_FILE="/etc/mega"
MEGA_BASE_DIR = "/etc/mega"
# check if config file exists
if [ -f "$CONFIG_FILE" ]; then
    exec /opt/mega/target/debug/mono -c "$CONFIG_FILE"  service http --host 0.0.0.0
else
    exec /opt/mega/target/debug/mono service http --host 0.0.0.0
fi