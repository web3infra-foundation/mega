use callisto::refs;
use common::utils::generate_id;

use crate::internal::pack::reference::RefCommand;

impl From<RefCommand> for refs::Model {
    fn from(value: RefCommand) -> Self {
        refs::Model {
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
