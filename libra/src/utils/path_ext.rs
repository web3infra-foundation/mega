use std::path::PathBuf;
use crate::utils::util;

pub trait PathExt {
    fn to_workdir(&self) -> PathBuf;
    fn to_string_or_panic(&self) -> String;
}

impl PathExt for PathBuf {
    fn to_workdir(&self) -> PathBuf {
        util::to_workdir_path(self)
    }

    /// `PathBuf` to `String`, may panic
    /// - aka: `into_os_string().into_string().unwrap()`
    fn to_string_or_panic(&self) -> String {
        self.clone().into_os_string().into_string().unwrap()
    }
}