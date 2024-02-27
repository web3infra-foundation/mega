use serde::{Deserialize, Serialize};

use crate::hash::SHA1;
use crate::internal::pack::header::EntryHeader;

///
/// One Pre loading Git object in memory
///
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Entry {
    pub header: EntryHeader,
    pub offset: usize,
    pub data: Vec<u8>,
    pub hash: Option<SHA1>,
}
