pub mod add;
pub mod branch;
pub mod commit;
pub mod init;
pub mod status;
pub mod remove;

use std::str::FromStr;

use storage::driver::file_storage::FileStorage;
use venus::{hash::SHA1, internal::object::ObjectTrait};
// impl load for all objects
async fn load_object<T>(
    hash: &str,
    storage: &impl FileStorage,
) -> Result<T, venus::errors::GitError>
where
    T: ObjectTrait,
{
    let data = storage.get(hash).await.unwrap();
    T::from_bytes(data.to_vec(), SHA1::from_str(hash).unwrap())
}
