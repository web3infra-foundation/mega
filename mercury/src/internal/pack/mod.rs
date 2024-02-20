//! 
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!
pub mod decode;
pub mod encode;
pub mod wrapper;
pub mod utils;
pub mod cache;


use venus::hash::SHA1;

///
/// 
/// 
#[allow(unused)]
pub struct Pack {
    pub number: usize,
    pub signature: SHA1,
}

#[cfg(test)]
mod tests {
}