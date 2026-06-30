use std::str::FromStr;

use common::config::MonoConfig;
use git_internal::{hash::ObjectHash, internal::object::commit::Commit};

use super::MegaModelConverter;

#[test]
pub fn test_init_mega_dir() {
    let mut mono_config = MonoConfig::default();
    if !mono_config.root_dirs.iter().any(|d| d == "toolchains") {
        mono_config.root_dirs.push("toolchains".to_string());
    }
    let converter = MegaModelConverter::init(&mono_config);
    let mega_trees = converter.mega_trees.borrow().clone();
    let mega_blobs = converter.mega_blobs.borrow().clone();
    let dir_nums = mono_config.root_dirs.len();
    assert_eq!(mega_trees.len(), dir_nums + 2);
    assert_eq!(mega_blobs.len(), dir_nums + 5);
}

#[test]
pub fn test_init_commit() {
    let commit = Commit::from_tree_id(
        ObjectHash::from_str("bd4a28f2d8b2efc371f557c3b80d320466ed83f3").unwrap(),
        vec![],
        "\nInit Mega Directory",
    );
    println!("{commit}");
}
