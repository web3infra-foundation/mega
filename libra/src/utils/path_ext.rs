use crate::utils::util;
use std::path::{Path, PathBuf};

pub trait PathExt {
    fn to_workdir(&self) -> PathBuf;
    fn to_string_or_panic(&self) -> String;
    fn workdir_to_absolute(&self) -> PathBuf;
    #[allow(dead_code)]
    fn workdir_to_current(&self) -> PathBuf;
    #[allow(dead_code)]
    fn sub_of(&self, parent: &Path) -> bool;
    fn sub_of_paths<P, U>(&self, paths: U) -> bool
    where
        P: AsRef<Path>,
        U: IntoIterator<Item = P>;
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

    fn workdir_to_absolute(&self) -> PathBuf {
        util::workdir_to_absolute(self)
    }

    fn workdir_to_current(&self) -> PathBuf {
        util::workdir_to_current(self)
    }

    /// Check if `self` is a sub path (child) of `parent`<br>
    /// Simply convert to absolute path (to current dir) and call `starts_with`
    /// - aka: "src/main.rs" is a sub path of "src/"
    fn sub_of(&self, parent: &Path) -> bool {
        util::is_sub_path(self, parent)
    }

    fn sub_of_paths<P, U>(&self, paths: U) -> bool
    where
        P: AsRef<Path>,
        U: IntoIterator<Item = P>,
    {
        // TODO 接口都改成 to workdir好了
        util::is_sub_of_paths(self, paths)
    }
}
