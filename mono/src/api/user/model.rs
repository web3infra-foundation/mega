use callisto::ssh_keys;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AddSSHKey {
    pub ssh_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSSHKey {
    pub id: i64,
    pub ssh_key: String,
}

impl From<ssh_keys::Model> for ListSSHKey {
    fn from(value: ssh_keys::Model) -> Self {
        Self {
            id: value.id,
            ssh_key: value.ssh_key,
        }
    }
}
