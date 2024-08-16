use std::{
    error::Error,
    ffi::NulError,
    fmt,
    io,
};

define_error!(OSGetLocaleError);

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl From <std::ffi::NulError> for OSGetLocaleError {
    fn from(value: NulError) -> OSGetLocaleError {
        OSGetLocaleError::new("ad")
    }
}

