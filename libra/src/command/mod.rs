pub mod add;
pub mod branch;
pub mod clone;
pub mod commit;
pub mod fetch;
pub mod index_pack;
pub mod init;
pub mod log;
pub mod merge;
pub mod pull;
pub mod push;
pub mod remote;
pub mod remove;
pub mod restore;
pub mod status;
pub mod switch;
pub mod lfs;
pub mod diff;

use crate::internal::protocol::https_client::BasicAuth;
use crate::utils::util;
use mercury::{errors::GitError, hash::SHA1, internal::object::ObjectTrait};
use rpassword::read_password;
use std::io;
use std::io::Write;
use std::path::Path;
use mercury::internal::object::blob::Blob;
use crate::utils;
use crate::utils::object_ext::BlobExt;

// impl load for all objects
fn load_object<T>(hash: &SHA1) -> Result<T, GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = storage.get(hash)?;
    T::from_bytes(&data.to_vec(), *hash)
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
fn ask_username_password() -> (String, String) {
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

/// same as ask_username_password, but return BasicAuth
pub fn ask_basic_auth() -> BasicAuth {
    let (username, password) = ask_username_password();
    BasicAuth { username, password }
}

/// Format commit message with GPG signature<br>
/// There must be a `blank line`(\n) before `message`, or remote unpack failed.<br>
/// If there is `GPG signature`,
/// `blank line` should be placed between `signature` and `message`
pub fn format_commit_msg(msg: &str, gpg_sig: Option<&str>) -> String {
    match gpg_sig {
        None => {
            format!("\n{}", msg)
        }
        Some(gpg) => {
            format!("{}\n\n{}", gpg, msg)
        }
    }
}
/// parse commit message
pub fn parse_commit_msg(msg_gpg: &str) -> (String, Option<String>) {
    const GPG_SIG_START: &str = "gpgsig -----BEGIN PGP SIGNATURE-----";
    const GPG_SIG_END: &str = "-----END PGP SIGNATURE-----";
    let gpg_start = msg_gpg.find(GPG_SIG_START);
    let gpg_end = msg_gpg.find(GPG_SIG_END).map(|end| end + GPG_SIG_END.len());
    let gpg_sig = match (gpg_start, gpg_end) {
        (Some(start), Some(end)) => {
            if start < end {
                Some(msg_gpg[start..end].to_string())
            } else {
                None
            }
        }
        _ => None,
    };
    match gpg_sig {
        Some(gpg) => {
            // skip the leading '\n\n' (blank line)
            let msg = msg_gpg[gpg_end.unwrap()..].to_string();
            assert!(msg.starts_with("\n\n"), "commit message format error");
            let msg = msg[2..].to_string();
            (msg, Some(gpg))
        }
        None => {
            assert!(msg_gpg.starts_with('\n'), "commit message format error");
            let msg = msg_gpg[1..].to_string(); // skip the leading '\n' (blank line)
            (msg, None)
        }
    }
}

/// Calculate the hash of a file blob
/// - for `lfs` file: calculate hash of the pointer data
pub fn calc_file_blob_hash(path: impl AsRef<Path>) -> io::Result<SHA1> {
    let blob =  if utils::lfs::is_lfs_tracked(&path) {
        let (pointer, _) = utils::lfs::generate_pointer_file(&path);
        Blob::from_content(&pointer)
    } else {
        Blob::from_file(&path)
    };
    Ok(blob.id)
}

#[cfg(test)]
mod test {
    use mercury::internal::object::commit::Commit;

    use super::*;
    use crate::utils::test;
    #[tokio::test]
    async fn test_save_load_object() {
        test::setup_with_new_libra().await;
        let object = Commit::from_tree_id(SHA1::new(&vec![1; 20]), vec![], "Commit_1");
        save_object(&object, &object.id).unwrap();
        let _ = load_object::<Commit>(&object.id).unwrap();
    }

    #[test]
    fn test_format_and_parse_commit_msg() {
        let msg = "commit message";
        let gpg_sig = "gpgsig -----BEGIN PGP SIGNATURE-----\ncontent\n-----END PGP SIGNATURE-----";
        let msg_gpg = format_commit_msg(msg, Some(gpg_sig));
        let (msg_, gpg_sig_) = parse_commit_msg(&msg_gpg);
        assert_eq!(msg, msg_);
        assert_eq!(gpg_sig, gpg_sig_.unwrap());

        let msg_gpg = format_commit_msg(msg, None);
        let (msg_, gpg_sig_) = parse_commit_msg(&msg_gpg);
        assert_eq!(msg, msg_);
        assert_eq!(None, gpg_sig_);
    }
}
