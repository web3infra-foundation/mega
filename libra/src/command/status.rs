use std::path::PathBuf;
use crate::internal::head::Head;
use crate::internal::index::Index;
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
pub async fn changes_to_be_committed() -> Changes {
    let mut changes = Changes::default();
    let index = Index::load().unwrap();
    let head_commit = Head::current_commit().await;
    let tracked_files = index.tracked_files();
    if head_commit.is_none() {
        changes.new = tracked_files;
        return changes;
    }
    let head_commit = head_commit.unwrap();

    !todo!()
}

/// Compare the difference between `index` and the `workdir`
pub fn changes_to_be_staged() -> Changes {
    !todo!()
}