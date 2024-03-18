use common::utils::{MEGA_BRANCH_NAME, ZERO_ID};
use serde::{Deserialize, Serialize};

use callisto::{db_enums::RefType, mega_refs, refs};

///
/// Represent the references(all branches and tags) in protocol transfer
///
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Refs {
    pub ref_name: String,
    pub ref_hash: String,
}


impl From<refs::Model> for Refs {
    fn from(value: refs::Model) -> Self {
        Self {
            ref_name: value.ref_name,
            ref_hash: value.ref_git_id,
        }
    }
}

impl From<mega_refs::Model> for Refs {
    fn from(value: mega_refs::Model) -> Self {
        Self {
            ref_name: MEGA_BRANCH_NAME.to_owned(),
            ref_hash: value.ref_git_id,
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
    pub ref_type: RefType,
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
                RefType::Tag
            } else {
                RefType::Branch
            },
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
        self.status = RefCommand::FAILED_STATUS.to_owned();
        self.error_msg = msg;
    }
}

