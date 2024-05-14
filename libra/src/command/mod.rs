pub mod add;
pub mod branch;
pub mod clone;
pub mod commit;
pub mod fetch;
pub mod index_pack;
pub mod init;
pub mod log;
pub mod merge;
pub mod push;
pub mod remove;
pub mod restore;
pub mod status;
pub mod switch;

use std::io;
use std::io::Write;
use rpassword::read_password;
use crate::utils::util;
use mercury::{errors::GitError, hash::SHA1, internal::object::ObjectTrait};

// impl load for all objects
fn load_object<T>(hash: &SHA1) -> Result<T, GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = storage.get(hash)?;
    T::from_bytes(data.to_vec(), *hash)
}

// impl save for all objects
fn save_object<T>(object: &T, ojb_id: &SHA1) -> Result<(), GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = object.to_data()?;
    storage.put(ojb_id, &data, object.get_type())?;
    Ok(())
}

/// Ask for username and password (CLI interaction)
pub fn ask_username_password() -> (String, String) {
    print!("username: ");
    // Normally your OS will buffer output by line when it's connected to a terminal,
    // which is why it usually flushes when a newline is written to stdout.
    io::stdout().flush().unwrap(); // ensure the prompt is shown
    let mut username = String::new();
    io::stdin().read_line(&mut username).unwrap();
    username = username.trim().to_string();

    print!("password: ");
    io::stdout().flush().unwrap();
    let password = read_password().unwrap(); // hide password
    (username, password)
}

#[cfg(test)]
mod test {
    use mercury::internal::object::commit::Commit;

    use super::*;
    use crate::utils::test;
    #[tokio::test]
    async fn test_save_load_object() {
        test::setup_with_new_libra().await;
        let object = Commit::from_tree_id(
            SHA1::new(&vec![1; 20]),
            vec![],
            "Commit_1",
        );
        save_object(&object, &object.id).unwrap();
        let _ = load_object::<Commit>(&object.id).unwrap();
    }
}
