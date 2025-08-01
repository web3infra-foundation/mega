## Vault Module

A secure cryptographic vault module for the Mega project, providing unified interfaces for secret management, cryptographic operations, and certificate handling.

## Overview
The Vault module is a comprehensive security library that integrates multiple cryptographic functionalities including:

- Secret Management : Secure storage and retrieval of sensitive data
- PGP Operations : Key generation, signing, and encryption using OpenPGP
- PKI (Public Key Infrastructure) : Certificate authority operations and X.509 certificate management
- Nostr Identity : Decentralized identity management using Nostr protocol
- Vault Core Integration : Seamless integration with HashiCorp Vault-compatible backends


## Features

### 🔐 Secret Management
- Generic vault interface for storing and retrieving secrets
- Prefix-based key organization
- Type-safe secret operations

### 🔑 PGP Support
- RSA and ECDSA key generation
- Key pair management (public/private)
- Armored key format support
- Configurable key parameters and encryption

### 📜 PKI Operations
- Certificate Authority (CA) initialization
- X.509 certificate issuance
- Certificate verification (time and signature)
- Role-based certificate management

### 🌐 Nostr Identity
- Secp256k1 key pair generation
- Base58-encoded identity management
- Automatic key persistence

### 🏗️ Architecture
- VaultCore : Main vault implementation using RustyVault
- JupiterBackend : Custom storage backend integration
- Trait-based Design : Extensible interfaces for different vault implementations

## Quick Start

### Basic Vault Usage

```Rust
use vault::{Vault, integration::vault_core::VaultCore};
use jupiter::storage::Storage;

// Initialize vault core
let storage = Storage::new(/* your storage config */);
let vault_core = VaultCore::new(storage);

// Define your vault implementation
struct MyVault {
    core: VaultCore,
}

impl Vault for MyVault {
    type Core = VaultCore;
    const VAULT_PREFIX: &'static str = "my_app";
    
    fn core(&self) -> &Self::Core {
        &self.core
    }
}

// Use the vault
let my_vault = MyVault { core: vault_core };
my_vault.save_to_vault("api_key", "secret_value");
let secret = my_vault.get_from_vault("api_key".to_string());
```

### PGP Operations

```Rust
use vault::pgp::KeyType;
use vault::integration::vault_core::VaultCore;

// Generate PGP key pair
let vault_core = VaultCore::new(storage);
let params = VaultCore::params(
    KeyType::Rsa(2048),
    Some("passphrase".to_string()),
    "user@example.com"
);
let (public_key, secret_key) = vault_core.gen_pgp_keypair(params, Some("passphrase".to_string()));

// Save keys to vault
vault_core.save_keys(public_key, secret_key);

// Load keys from vault
let public_key = vault_core.load_pub_key();
let secret_key = vault_core.load_sec_key().await;
```

### PKI Certificate Management

```Rust
use serde_json::json;

// Configure certificate role
vault_core.config_role(json!({
    "ttl": "60d",
    "max_ttl": "365d",
    "key_type": "rsa",
    "key_bits": 4096,
    "country": "US",
    "organization": "My Organization"
}));

// Issue a certificate
let (cert_pem, private_key) = vault_core.issue_cert(json!({
    "common_name": "example.com",
    "alt_names": ["www.example.com", "api.example.com"]
}));

// Verify certificate
let is_valid = vault_core.verify_cert(cert_pem.as_bytes());
```

### Nostr Identity

```Rust
// Load or generate Nostr identity
let (nostr_id, secret_key) = vault_core.load_nostr_pair();
let peer_id = vault_core.load_nostr_peerid();
let keypair = vault_core.load_nostr_secp_pair();

println!("Nostr ID: {}", nostr_id);
```

## Configuration
The vault module uses the following configuration:

- Storage Backend : Integrates with Jupiter storage system
- Vault Directory : ~/.mega/vault/ (configurable)
- Core Key File : core_key.json for vault unsealing
- Seal Configuration : 10 secret shares, 5 threshold
## Dependencies
- rusty_vault : HashiCorp Vault-compatible core
- openssl : PKI and certificate operations
- pgp : OpenPGP implementation
- secp256k1 : Elliptic curve cryptography for Nostr
- serde_json : JSON serialization
- tokio : Async runtime support
## Security Considerations
- All secret keys are encrypted at rest
- Vault core uses AES-GCM encryption
- Configurable key sharing and threshold schemes
- Secure random number generation for all cryptographic operations