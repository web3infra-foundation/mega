use serde::{Deserialize, Serialize};

use db_entity::db_enums::RefType;

///
/// Represent the references(all branches and tags) in protocol transfer
///
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Reference {
    pub ref_name: String,
    pub ref_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandType {
    Create,
    Delete,
    Update,
}

/// Reference Update Request
#[derive(Debug, Clone, PartialEq)]
pub struct RefCommand {
    pub ref_name: String,
    pub old_id: String,
    pub new_id: String,
    pub status: String,
    pub error_msg: String,
    pub command_type: CommandType,
    pub ref_type: RefType,
}
