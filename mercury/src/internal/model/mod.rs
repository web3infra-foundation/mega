pub mod blob;
pub mod commit;
pub mod tag;
pub mod tree;
pub mod sea_models;

use idgenerator::IdInstance;

pub(crate) fn generate_id() -> i64 {
    // Call `next_id` to generate a new unique id.
    IdInstance::next_id()
}
