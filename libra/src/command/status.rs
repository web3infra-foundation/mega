use std::path::PathBuf;
use crate::utils::util;

#[derive(Debug, Default, Clone)]
pub struct Changes {
    pub new: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
}
pub fn execute() {
    util::check_repo_exist();
    todo!()
}

/**
 * Compare the difference between `index` and the last `Commit Tree`
 */
pub fn changes_to_be_committed() -> Changes {
    !todo!()
}

/// Compare the difference between `index` and the `workdir`
pub fn changes_to_be_staged() -> Changes {
    !todo!()
}