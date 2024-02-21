use common::utils::generate_id;
use db_entity::git_refs;

use crate::internal::pack::reference::RefCommand;



impl From<RefCommand> for git_refs::Model {
    fn from(value: RefCommand) -> Self {
        git_refs::Model {
            id: generate_id(),
            repo_id: 0,
            ref_name: value.ref_name,
            ref_git_id: String::new(),
            ref_type: value.ref_type,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}