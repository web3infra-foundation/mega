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

use crate::utils::util;
use venus::{hash::SHA1, internal::object::ObjectTrait};

// impl load for all objects
fn load_object<T>(hash: &SHA1) -> Result<T, venus::errors::GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = storage.get(hash)?;
    T::from_bytes(data.to_vec(), *hash)
}

// impl save for all objects
fn save_object<T>(object: &T, ojb_id: &SHA1) -> Result<(), venus::errors::GitError>
where
    T: ObjectTrait,
{
    let storage = util::objects_storage();
    let data = object.to_data()?;
    storage.put(ojb_id, &data, object.get_type())?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::test;
    #[tokio::test]
    async fn test_save_load_object() {
        test::setup_with_new_libra().await;
        let object = venus::internal::object::commit::Commit::from_tree_id(
            venus::hash::SHA1::new(&vec![1; 20]),
            vec![],
            "Commit_1",
        );
        save_object(&object, &object.id).unwrap();
        let _ = load_object::<venus::internal::object::commit::Commit>(&object.id).unwrap();
    }
}
