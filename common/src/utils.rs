use idgenerator::IdInstance;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub const ZERO_ID: &str = match std::str::from_utf8(&[b'0'; 40]) {
    Ok(s) => s,
    Err(_) => panic!("can't get ZERO_ID"),
};

pub fn generate_id() -> i64 {
    // Call `next_id` to generate a new unique id.
    IdInstance::next_id()
}

pub fn generate_link() -> String {
    let str: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    str.to_uppercase()
}

pub const MEGA_BRANCH_NAME: &str = "refs/heads/main";
