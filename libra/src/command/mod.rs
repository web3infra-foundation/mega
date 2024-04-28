pub mod add;
pub mod branch;
pub mod commit;
pub mod init;
pub mod log;
pub mod remove;
pub mod restore;
pub mod status;

use crate::utils::util;
use venus::{hash::SHA1, internal::object::ObjectTrait};

// impl load for all objects
fn load_object<T>(hash: &SHA1) -> Result<T, venus::errors::GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = storage.get(hash).unwrap();
    T::from_bytes(data.to_vec(), *hash)
}

// impl save for all objects
fn save_object<T>(object: &T) -> Result<SHA1, venus::errors::GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = object.to_data()?;
    let hash = SHA1::from_type_and_data(object.get_type(), &data);
    storage.put(&hash, &data, object.get_type()).unwrap();
    Ok(hash)
}
