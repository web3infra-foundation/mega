#!/bin/bash

# Check if both required arguments are provided
if [ $# -ne 2 ]; then
    echo "Error: Both directory path and config file path are required"
    echo "Usage: $0 <directory_path> <config_file_path>"
    exit 1
fi

# Get the directory path argument
base_dir="$1"

# Get the config file path
config_file="$2"

# Check if the config file exists
if [ ! -f "$config_file" ]; then
    echo "Error: Config file not found at $config_file"
    exit 1
fi

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if the mono directory already exists
mono_dir="$base_dir/mono"
if [ -d "$mono_dir" ]; then
    read -p "The directory $mono_dir already exists. Do you want to delete it and continue? (y/N): " confirm
    if [[ $confirm == [yY] || $confirm == [yY][eE][sS] ]]; then
        echo "Deleting existing mono directory. Root password may be required."
        if command_exists sudo; then
            sudo rm -rf "$mono_dir"
        else
            echo "Error: sudo is not available. Please run this script with appropriate permissions to delete the directory."
            exit 1
        fi
    else
        echo "Operation cancelled. Exiting..."
        exit 0
    fi
fi

# Create the base directory (if it doesn't exist)
mkdir -p "$base_dir"

# Create the mono directory
mkdir -p "$mono_dir"

# Create mono-data and pg-data directories under the mono directory
mono_data_dir="$mono_dir/mono-data"
pg_data_dir="$mono_dir/pg-data"
mkdir -p "$mono_data_dir"
mkdir -p "$pg_data_dir"

# Create subdirectories in mono-data
mkdir -p "$mono_data_dir/etc"
mkdir -p "$mono_data_dir/cache"
mkdir -p "$mono_data_dir/lfs"
mkdir -p "$mono_data_dir/logs"
mkdir -p "$mono_data_dir/objects"

# Create ssh and https directories under etc
ssh_dir="$mono_data_dir/etc/ssh"
mkdir -p "$ssh_dir"
mkdir -p "$mono_data_dir/etc/https"

# Generate SSH key for sshd (non-interactive)
ssh_key_file="$ssh_dir/ssh_host_rsa_key"
ssh-keygen -t rsa -b 4096 -f "$ssh_key_file" -N "" -C "sshd host key" >/dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "SSH host key generated at $ssh_key_file"
else
    echo "Warning: Failed to generate SSH host key"
fi

# Copy config file
cp "$config_file" "$mono_data_dir/etc/config.toml"
echo "Config file copied to $mono_data_dir/etc/config.toml"

echo "Directory structure has been successfully created:"
echo "$base_dir"
echo "└── mono"
echo "    ├── mono-data"
echo "    │   ├── etc"
echo "    │   │   ├── ssh"
echo "    │   │   │   ├── ssh_host_rsa_key"
echo "    │   │   │   └── ssh_host_rsa_key.pub"
echo "    │   │   ├── https"
echo "    │   │   └── config.toml"
echo "    │   ├── cache"
echo "    │   ├── lfs"
echo "    │   ├── logs"
echo "    │   └── objects"
echo "    └── pg-data"

echo "Note: Please review and set appropriate permissions if needed."