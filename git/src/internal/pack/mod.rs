//!
//!
//!
//!
//!

use std::path::PathBuf;

use crate::hash::Hash;

/// ### PackFile Structure<br>
///  `head`: always = "PACK" <br>
/// `version`: version code <br>
/// `number_of_objects` : Total mount of objects <br>
/// `signature`:Hash <br>
/// `result`: decoded cache,
#[allow(unused)]
#[derive(Default)]
pub struct Pack {
    head: [u8; 4],
    version: u32,
    number_of_objects: usize,
    pub signature: Hash,
    // pub result: Arc<PackObjectCache>,
    pack_file: PathBuf,
}

#[cfg(test)]
mod tests {}
