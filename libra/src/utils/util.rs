use std::env;
use std::path::PathBuf;

pub const ROOT_DIR: &str = ".libra";
pub const DATABASE: &str = "libra.db";

#[allow(dead_code)]
pub fn cur_dir() -> PathBuf {
    env::current_dir().unwrap()
}