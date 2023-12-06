//! 
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!
pub mod decode;
pub mod wrapper;
pub mod utils;


///
/// 
/// 
#[allow(unused)]
pub struct Pack {
    pub number: usize,
}

#[cfg(test)]
mod tests {
}