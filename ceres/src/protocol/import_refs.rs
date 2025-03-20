use serde::{Deserialize, Serialize};

use callisto::{import_refs, mega_refs, sea_orm_active_enums::RefTypeEnum};
use common::utils::{generate_id, MEGA_BRANCH_NAME, ZERO_ID};

///
/// Represent the references(all branches and tags) in protocol transfer
///
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Refs {
    pub id: i64,
    pub ref_name: String,
    pub ref_hash: String,
    pub default_branch: bool,
}

impl From<import_refs::Model> for Refs {
    fn from(value: import_refs::Model) -> Self {
        Self {
            id: value.id,
            ref_name: value.ref_name,
            ref_hash: value.ref_git_id,
            default_branch: value.default_branch,
        }
    }
}

impl From<mega_refs::Model> for Refs {
    fn from(value: mega_refs::Model) -> Self {
        Self {
            id: value.id,
            ref_name: value.ref_name.clone(),
            ref_hash: value.ref_commit_hash,
            default_branch: value.ref_name == MEGA_BRANCH_NAME,
        }
    }
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
    pub ref_type: RefTypeEnum,
    pub default_branch: bool,
}

pub const SP: char = ' ';

impl RefCommand {
    const OK_STATUS: &'static str = "ok";

    const FAILED_STATUS: &'static str = "ng";

    pub fn new(old_id: String, new_id: String, ref_name: String) -> Self {
        let command_type = if ZERO_ID == old_id {
            CommandType::Create
        } else if ZERO_ID == new_id {
            CommandType::Delete
        } else {
            CommandType::Update
        };
        RefCommand {
            ref_name: ref_name.clone(),
            old_id,
            new_id,
            status: RefCommand::OK_STATUS.to_owned(),
            error_msg: "".to_owned(),
            command_type,
            ref_type: if ref_name.starts_with("refs/tags") {
                RefTypeEnum::Tag
            } else {
                RefTypeEnum::Branch
            },
            default_branch: false,
        }
    }

    pub fn get_status(&self) -> String {
        if RefCommand::OK_STATUS == self.status {
            format!("{}{}{}", self.status, SP, self.ref_name,)
        } else {
            format!(
                "{}{}{}{}{}",
                self.status,
                SP,
                self.ref_name,
                SP,
                self.error_msg.clone()
            )
        }
    }

    pub fn failed(&mut self, msg: String) {
        RefCommand::FAILED_STATUS.clone_into(&mut self.status);
        self.error_msg = msg;
    }
}

impl From<RefCommand> for import_refs::Model {
    fn from(value: RefCommand) -> Self {
        import_refs::Model {
            id: generate_id(),
            repo_id: 0,
            ref_name: value.ref_name,
            ref_git_id: value.new_id,
            ref_type: value.ref_type,
            default_branch: value.default_branch,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<RefCommand> for mega_refs::Model {
    fn from(value: RefCommand) -> Self {
        mega_refs::Model {
            id: generate_id(),
            path: String::new(),
            ref_name: value.ref_name,
            ref_commit_hash: value.new_id,
            ref_tree_hash: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
