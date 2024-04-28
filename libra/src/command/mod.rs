pub mod add;
pub mod branch;
pub mod commit;
pub mod init;
pub mod status;
pub mod remove;
pub mod log;
pub mod restore;

use venus::{hash::SHA1, internal::object::ObjectTrait};
use crate::utils::util;

// impl load for all objects
fn load_object<T>(
    hash: &SHA1,
) -> Result<T, venus::errors::GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = storage.get(hash).unwrap();
    T::from_bytes(data.to_vec(), *hash)
}
