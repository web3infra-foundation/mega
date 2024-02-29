use std::fs::{create_dir_all, read_dir};
use std::{fs, io};
use std::path::{Path, PathBuf};
use gettextrs::gettext;
use crate::lfs::errors::install_error::ENVINSTALLError;
use crate::lfs::tools::constant_table::env_prompt_message;

#[cfg(target_os = "windows")]
pub mod command_install{
    use std::{env, fs, io, ptr};
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    use std::fs::{create_dir_all, read_dir};
    use std::iter::once;
    use std::path::{Path, PathBuf};

    use winapi::um::winuser::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,SW_SHOW};
    use winapi::shared::minwindef::{WPARAM, LPARAM};
    use winapi::um::shellapi::{SEE_MASK_NOCLOSEPROCESS, ShellExecuteExW, SHELLEXECUTEINFOW};
    use gettextrs::gettext;
    use winapi::um::winnt::{HANDLE, TOKEN_QUERY,TOKEN_ELEVATION, TokenElevation};
    use winapi::shared::ntdef::HANDLE as OtherHANDLE;
    use winapi::um::processthreadsapi::{GetCurrentProcess,OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::handleapi::CloseHandle;
    use winapi::ctypes::c_void;

    use crate::lfs::commands::command_install::{copy_file,path_buf_to_str,copy_dir};
    use crate::lfs::errors::install_error::ENVINSTALLError;
    use crate::lfs::tools::constant_table::{env_utils_table,env_prompt_message};
    use crate::lfs::tools::env_utils::env_utils;
    fn notify_environment_change() {
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0 as WPARAM,
                "Environment\0".encode_utf16().collect::<Vec<u16>>().as_ptr() as LPARAM,
                SMTO_ABORTIFHUNG,
                5000,
                ptr::null_mut(),
            );
        }

    }


    fn elevate_privileges() ->Result<*mut c_void, ENVINSTALLError> {
        let operation = to_wide_chars("runas");
        let file = to_wide_chars("cmd.exe");
        let current_path = env::current_dir()?;
        let current_path_str = path_buf_to_str(&current_path).map_err(|_| ENVINSTALLError::new(
            gettext(
                env_prompt_message::ENVPromptMsgCharacters::get(
                    env_prompt_message::ENVPromptMsg::DIRCODError
                )
            )
        ))?;
        let program_dir = current_path.join(
            &env_utils_table::ENVIRONMENTCharacters::get(
                env_utils_table::ENVIRONMENTEnum::PROGRAMDIR_WIN
            )
        );
        let program_dir_str = path_buf_to_str(&program_dir).map_err(|_| ENVINSTALLError::new(
            gettext(
                env_prompt_message::ENVPromptMsgCharacters::get(
                    env_prompt_message::ENVPromptMsg::DIRCODError
                )
            )
        ))?;
        let parameters = to_wide_chars(&format!(
            "/K cd /d \"{}\" && \"{}\" lfs install",
            current_path_str,
            program_dir_str
        ));
        let mut sei = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            hwnd: ptr::null_mut(),
            lpVerb: operation.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: parameters.as_ptr(),
            lpDirectory:ptr::null_mut(),
            nShow: SW_SHOW,
            hInstApp: ptr::null_mut(),
            lpIDList: ptr::null_mut(),
            lpClass: ptr::null(),
            hkeyClass: ptr::null_mut(),
            dwHotKey: 0,
            hMonitor: ptr::null_mut(),
            hProcess: ptr::null_mut(),
        };

        unsafe {
            if ShellExecuteExW(&mut sei) == 0 {
                Err(ENVINSTALLError::from(io::Error::last_os_error()))
            }else {
                Ok(sei.hProcess as *mut c_void)
            }
        }
    }
    fn is_admin() -> bool {
        let mut is_admin = false;

        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) != 0 {
                if GetTokenInformation(
                    token,
                    TokenElevation,
                    &mut elevation as *mut _ as *mut c_void,
                    size,
                    &mut size,
                ) != 0
                {
                    is_admin = elevation.TokenIsElevated != 0;
                }
                CloseHandle(token);
            }
        }

        is_admin
    }

    fn to_wide_chars(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    pub fn install_command() -> Result<(),ENVINSTALLError> {

        if is_admin(){
            let current_path = env::current_dir()?;
            let program_dir = current_path.join(
                &env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::PROGRAMDIR_WIN
                )
            );
            let translation_dir = current_path.join(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::TRANSLATIONSDIR_WIN
                )
            );
            match copy_file(&program_dir,Path::new(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::PROGRAMDIR_DESTINATIONPATH_WIN
                )
            )) {
                Ok(()) => {
                    print!(
                        "{}", gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::GitCraftSUCCESS
                            )
                        )
                    )
                },
                Err(e) => {
                    println!("{}", e);
                    Err(ENVINSTALLError::with_source(
                        gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::GitCraftFAILED
                            )
                        )
                        ,e))?
                }
            }
            match copy_file(&translation_dir,Path::new(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::TRANSLATIONS_DESTINATIONPATH_WIN
                )
            )) {
                Ok(()) => {
                    print!(
                        "{}", gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::TranslationsSUCCESS
                            )
                        )
                    )
                },
                Err(e) => {
                    Err(ENVINSTALLError::with_source(
                        gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::TranslationsFAILED
                            )
                        ),e
                    ))?
                }
            }
            let env = env_utils::Environment::new()?;
            let path = env.get_variable(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::PATH
                )
            )?;
            let new_path = if !path.contains(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::ENVDIR_WIN
                )
            ) {
                format!("{};{}", path,
                        env_utils_table::ENVIRONMENTCharacters::get(
                            env_utils_table::ENVIRONMENTEnum::ENVDIR_WIN
                        )
                )
            } else {
                path
            };
            env.set_variable(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::PATH
                ),
                &new_path
            )?;

            notify_environment_change();

            Ok(())
        } else {
            elevate_privileges();
            Ok(())
        }

    }

}
fn path_buf_to_str<'a>(path_buf:&'a PathBuf) -> Result<&'a str,&'static str> {
    match path_buf.as_path().to_str() {
        Some(path_str) => Ok(path_str),
        None => panic!("{}",
                       gettext(
                           env_prompt_message::ENVPromptMsgCharacters::get(
                               env_prompt_message::ENVPromptMsg::DIRCODError
                           )
                       )
        )
    }
}
fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        create_dir_all(dst)?;
    }
    for entry in read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
fn copy_file(src: &Path, dst: &Path) -> Result<(), ENVINSTALLError> {
    if src.is_dir() {
        copy_dir(src, dst).map_err(|e| ENVINSTALLError::from(e))
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        Ok(())
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod command_install{
    use std::env;
    use std::path::Path;
    use gettextrs::gettext;
    use crate::lfs::commands::command_install::copy_file;
    use crate::lfs::errors::install_error::ENVINSTALLError;
    use crate::lfs::tools::constant_table::{env_utils_table,env_prompt_message};

    fn is_root() -> bool {
        unsafe {
            libc::getuid() == 0
        }
    }
    pub fn install_command() -> Result<(),ENVINSTALLError> {
        if is_root() {
            let current_path = env::current_dir()?;
            let program_dir = current_path.join(
                &env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::PROGRAMDIR_Unix_Like
                )
            );
            let translation_dir = current_path.join(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::TRANSLATIONSDIR_Unix_Like
                )
            );
            match copy_file(&program_dir,Path::new(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::PROGRAMDIR_DESTINATIONPATH_Unix_Like
                )
            )) {
                Ok(()) => {
                    print!(
                        "{}", gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::GitCraftSUCCESS
                            )
                        )
                    )
                },
                Err(e) => {
                    println!("{}", e);
                    Err(ENVINSTALLError::with_source(
                        gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::GitCraftFAILED
                            )
                        )
                        ,e))?
                }
            }
            match copy_file(&translation_dir,Path::new(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::TRANSLATIONS_DESTINATIONPATH_Unix_Like
                )
            )) {
                Ok(()) => {
                    print!(
                        "{}", gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::TranslationsSUCCESS
                            )
                        )
                    )
                },
                Err(e) => {
                    Err(ENVINSTALLError::with_source(
                        gettext(
                            env_prompt_message::ENVPromptMsgCharacters::get(
                                env_prompt_message::ENVPromptMsg::TranslationsFAILED
                            )
                        ),e
                    ))?
                }
            }
            Ok(())
        } else {
            panic!("{}",gettext(
                env_prompt_message::ENVPromptMsgCharacters::get(
                    env_prompt_message::ENVPromptMsg::NOT_ROOT_RUN
                )
            ));
        }
    }
}