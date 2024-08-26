extern crate libc;

use std::{
    ffi::{CString, NulError, OsStr},
    iter::once,
};

#[cfg(target_os = "windows")]
use winapi::um::winnls::{SetThreadLocale,GetThreadLocale};
#[cfg(target_os = "windows")]
use winapi::um::winnls::LocaleNameToLCID;
#[cfg(target_os = "windows")]
use winapi::um::winnt::LCID;
#[cfg(target_os = "windows")]
use std::{ os::windows::ffi::OsStrExt};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use libc::{setlocale, LC_ALL};

use crate::lfs::errors::get_locale_error::OSGetLocaleError;
use crate::lfs::tools::constant_table::{get_locale_prompt_message,osget_locale_error};

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn with_locale<F>(locale_result: Result<CString, OSGetLocaleError>, f: F)
    where
        F: FnOnce(),
{
    unsafe {
        match locale_result {
            Ok(ref locale) => {
                let previous_locale = setlocale(LC_ALL, locale.as_ptr());
                if !previous_locale.is_null() {
                    f();

                    setlocale(LC_ALL, previous_locale);
                } else {
                    print!("{}", get_locale_prompt_message::GetLocalePromptMsgCharacters::get(
                        get_locale_prompt_message::GetLocalePromptMsg::SetThreadLocaleError
                    ));
                    f();
                }
            }
            Err(_) => {
                print!("{}",get_locale_prompt_message::GetLocalePromptMsgCharacters::get(
                    get_locale_prompt_message::GetLocalePromptMsg::FAIL_ENV_LANG
                ));
                f();
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn get_LCID(local:Result<String, OSGetLocaleError>) -> Result<LCID,OSGetLocaleError> {
   match local {
       Ok(local_name) => {
           let wide : Vec<u16> = OsStr::new(&local_name).encode_wide().chain(once(0)).collect();
           let lcid = unsafe {
               LocaleNameToLCID(wide.as_ptr(),0)
           };
           if lcid != 0 {
               Ok(lcid)
           } else {
               Err(OSGetLocaleError::new(osget_locale_error::OSGetLocaleErrorMsgCharacters::get(
                   osget_locale_error::OSGetLocaleErrorMsg::LCIDError
               )))
           }
       },
       Err(e) => Err(e)
   }
}

#[cfg(target_os = "windows")]
pub fn with_locale<F>(locale_result: Result<String, OSGetLocaleError>, f: F)
    where
        F: FnOnce(),
{
    match get_LCID(locale_result) {
        Ok(lcid) => {
            unsafe {
                let old_locale = GetThreadLocale();
                if SetThreadLocale(lcid) == 0 {
                    print!("{}", get_locale_prompt_message::GetLocalePromptMsgCharacters::get(
                        get_locale_prompt_message::GetLocalePromptMsg::SetThreadLocaleError
                    ));
                }
                f();

                if SetThreadLocale(old_locale) == 0 {
                    print!("{}",get_locale_prompt_message::GetLocalePromptMsgCharacters::get(
                        get_locale_prompt_message::GetLocalePromptMsg::RestoreThreadLocaleError
                    ));
                }
            }

        },
        Err(e) => eprintln!("Error getting LCID: {:?}", e)
    }
}
