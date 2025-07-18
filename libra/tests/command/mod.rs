use common::utils::format_commit_msg;
use libra::command::branch::execute;
use libra::command::branch::BranchArgs;
use libra::command::get_target_commit;
use libra::command::init::init;
use libra::command::init::InitArgs;
use libra::command::log::{get_reachable_commits, LogArgs};
use libra::command::save_object;
use libra::command::status::changes_to_be_staged;
use libra::command::switch::{self, check_status, SwitchArgs};
use libra::command::{
    add::{self, AddArgs},
    load_object,
};
use libra::internal::branch::Branch;
use libra::internal::head::Head;
use libra::{
    command::commit::{self, CommitArgs},
    utils::test::{self, ChangeDirGuard},
};
use mercury::hash::SHA1;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;
use serial_test::serial;
use std::path::Path;
use tempfile::tempdir;
mod add_test;
mod branch_test;
mod checkout_test;
mod clone_test;
mod commit_test;
mod config_test;
mod diff_test;
mod fetch_test;
mod index_pack_test;
mod init_test;
mod lfs_test;
mod log_test;
mod merge_test;
mod pull_test;
mod push_test;
mod remote_test;
mod remove_test;
mod restore_test;
mod status_test;
mod switch_test;
