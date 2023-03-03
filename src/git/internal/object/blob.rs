//!
//!
//!
//!
//!

use crate::git::internal::object::meta::Meta;

#[allow(unused)]
pub struct Blob {
    pub meta: Meta,
}

impl Blob {
    #[allow(unused)]
    pub fn new_from_meta(meta: Meta) -> Self {
        Self { meta }
    }
}

mod tests {

}