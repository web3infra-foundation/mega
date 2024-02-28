#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::ffi::CString;
use crate::lfs::errors::get_locale_error::OSGetLocaleError;
use crate::lfs::tools::constant_table::{get_locale_prompt_message,osget_locale_error};
#[cfg(target_os = "windows")]
pub fn get_locale() -> Result<String, OSGetLocaleError> {
    extern crate winapi;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use winapi::um::winnls::GetUserDefaultLocaleName;
    use winapi::shared::ntdef::WCHAR;

    const LOCALE_NAME_MAX_SIZE: usize = 85;
    let mut buffer: [WCHAR; LOCALE_NAME_MAX_SIZE] = [0; LOCALE_NAME_MAX_SIZE];

    let len =  unsafe {
        GetUserDefaultLocaleName(buffer.as_mut_ptr(),LOCALE_NAME_MAX_SIZE as i32)
    };
    if len == 0 {
        print!("{}",get_locale_prompt_message::GetLocalePromptMsgCharacters::get(
            get_locale_prompt_message::GetLocalePromptMsg::FAIL
        ))
    }

    let os_string = OsString::from_wide(&buffer[..(len as usize -1)]);
    os_string.into_string()
        .map_err(|_| OSGetLocaleError::new(osget_locale_error::OSGetLocaleErrorMsgCharacters::get(
            osget_locale_error::OSGetLocaleErrorMsg::ERROE
        ))
        )

}
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn get_locale() -> Result<CString, OSGetLocaleError> {
    use std::env;
    let lang = env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
    CString::new(lang).map_err(OSGetLocaleError::from)
}