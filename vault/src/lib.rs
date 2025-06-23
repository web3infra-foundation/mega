use crate::integration::vault_core::VaultCoreInterface;

pub mod integration;

pub mod nostr;
pub mod pgp;
pub mod pki;

/// A trait that defines the interface for a vault.
/// It provides methods to save, get, and delete secrets.
/// You can conviniently implement this trait for your structs that need to interact with a vault.
/// It is designed to be used with a vault core implementation, such as `VaultCore`.
///
/// # Example:
/// ```rust
/// use vault::Vault;
/// use vault::integration::vault_core::VaultCore;
/// struct MyVault {
///     core: VaultCore,
/// }
///
/// impl Vault for MyVault {
///    type Core = VaultCore;
///    const VAULT_PREFIX: &'static str = "my_vault_key_prefix";
///    fn core(&self) -> &Self::Core {
///        &self.core
///   }
/// }
/// ```
pub trait Vault {
    type Core: VaultCoreInterface;
    const VAULT_PREFIX: &'static str;

    fn core(&self) -> &Self::Core;

    /// Save a secret to the vault.
    fn save_to_vault(&self, key: impl AsRef<str>, value: impl AsRef<str>) {
        let key_f = format!("{}_{}", Self::VAULT_PREFIX, key.as_ref());
        let kv_data = serde_json::json!({
            "data": value.as_ref(),
        })
        .as_object()
        .unwrap()
        .clone();
        _ = self.core().write_secret(key_f.as_str(), Some(kv_data));
    }

    /// Get a secret from the vault.
    fn get_from_vault(&self, key: String) -> Option<String> {
        let key_f = format!("{}_{}", Self::VAULT_PREFIX, key);
        match self.core().read_secret(key_f.as_str()) {
            Ok(Some(data)) => data.get("data").and_then(|v| v.as_str().map(String::from)),
            Ok(None) | Err(_) => None,
        }
    }

    /// Delete a secret from the vault.
    fn delete_from_vault(&self, key: String) {
        let key_f = format!("{}_{}", Self::VAULT_PREFIX, key);
        _ = self.core().delete_secret(key_f.as_str());
    }
}
