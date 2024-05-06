
use idgenerator::IdInstance;

pub const ZERO_ID: &str = match std::str::from_utf8(&[b'0'; 40]) {
    Ok(s) => s,
    Err(_) => panic!("can't get ZERO_ID"),
};

pub fn generate_id() -> i64 {
    let mut new_id: i64 = 0;
    let mut times = 100;
    while times > 0 {
        // Call `next_id` to generate a new unique id.
        new_id = IdInstance::next_id();
        times -= 1;
    }
    new_id
}

pub const MEGA_BRANCH_NAME: &str= "refs/heads/main";
