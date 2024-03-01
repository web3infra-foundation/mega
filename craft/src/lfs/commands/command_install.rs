use std::fs::{ File};
use std::{fs, io};
use std::io::{ Read};
use std::path::{Path, PathBuf};
use gettextrs::gettext;
use rayon::prelude::*;
use crate::lfs::commands::utils::disk_judgment::disk_judgment::is_ssd;
use crate::lfs::tools::constant_table::env_prompt_message;
use crate::lfs::commands::utils::file_metadata::metadata_same::is_metadata_same;



#[cfg(target_os = "windows")]
pub mod command_install{
    use std::{env,io, ptr};
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
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
    use winapi::um::shlobj::{CSIDL_PROFILE, SHGetFolderPathW};
    use std::ptr::null_mut;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::process::Command;

    use crate::lfs::commands::command_install::{copy_file,path_buf_to_str};
    use crate::lfs::errors::install_error::ENVINSTALLError;
    use crate::lfs::tools::constant_table::{env_utils_table, env_prompt_message, vault_config,git_repo_table};
    use crate::lfs::tools::env_utils::env_utils;
    use crate::lfs::tools::gettext_format::remove_trailing_newlines;

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
    fn gitconfig_exists_in_path(directory: &Path) -> bool {
        let gitconfig_path = directory.join(git_repo_table::GitRepoCharacters::get(
            git_repo_table::GitRepo::GITCONFIG
        ));
        gitconfig_path.exists() && gitconfig_path.is_file()
    }
    fn get_user_home() -> PathBuf{
        let mut path_buf = vec![0u16; winapi::shared::minwindef::MAX_PATH];
        unsafe {
            SHGetFolderPathW(
                null_mut(),
                CSIDL_PROFILE,
                null_mut(),
                0,
                path_buf.as_mut_ptr(),
            );
        }
        let pos = path_buf.iter().position(|&c| c == 0).unwrap();
        let path_buf = &path_buf[..pos];
        let path = OsString::from_wide(path_buf);
        match path.to_str() {
            Some(path_str) => {
                let path = PathBuf::from(path_str);
                if gitconfig_exists_in_path(&path){
                    return path
                } else {
                    panic!("{}",env_prompt_message::ENVPromptMsgCharacters::get(
                        env_prompt_message::ENVPromptMsg::GITCONFIG_NOT_EXIST_ERROR
                    ))
                }
            },
            None => {
                panic!("{}",env_prompt_message::ENVPromptMsgCharacters::get(
                    env_prompt_message::ENVPromptMsg::HOME_DIR_ERROR
                ))
            }
        }
    }

    fn set_git_vault_filter(user_git_config_path: PathBuf) -> Result<(), ENVINSTALLError> {
        if !user_git_config_path.exists() {
            let error_msg = remove_trailing_newlines(
                gettext(
                    env_prompt_message::ENVPromptMsgCharacters::get(
                        env_prompt_message::ENVPromptMsg::GITCONFIG_NOT_EXIST_ERROR
                    )
                )
            );
            return Err(ENVINSTALLError::new(format!("{} {:?}",error_msg,user_git_config_path)));
        }
        let commands = vec![
            (vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::SMUDGE_KEY
            ), vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::SMUDGE_VALUE
            )),
            (vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::CLEAN_KEY
            ), vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::CLEAN_VALUE
            )),
        ];
        for (key, value) in commands {
            let output = Command::new("git")
                .args(&["config", "--global", key, value])
                .output();
            match output {
                Ok(output) if output.status.success() => continue,
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(ENVINSTALLError::new(
                        format!("{} {}", env_prompt_message::ENVPromptMsgCharacters::get(
                            env_prompt_message::ENVPromptMsg::GITCONFIG_ERROR
                        ),stderr
                        )
                    ));
                }
                Err(e) => {
                    let error_msg = remove_trailing_newlines(
                        gettext(env_prompt_message::ENVPromptMsgCharacters::get(
                            env_prompt_message::ENVPromptMsg::FAILED_GIT_CONFIG
                        ))
                    );
                    return Err(ENVINSTALLError::new(
                        format!("{}{}",error_msg,e)
                    ));
                }
            }
        }
        Ok(())
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

            match set_git_vault_filter(get_user_home()) {
                Ok(()) =>{
                    print!("{}", env_prompt_message::ENVPromptMsgCharacters::get(
                        env_prompt_message::ENVPromptMsg::VAULT_CONFIG_SUCCESS
                    ));
                },
                Err(e) => {
                    panic!("{}",e);
                }
            }

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
const BUFFER_SIZE: usize = 8 * 1024; // 8 KiB
fn calculate_md5(path: &Path) -> io::Result<[u8; 16]> {
    let mut file = File::open(path)?;
    let mut context = md5::Context::new();
    let mut buffer = [0; 1024];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.consume(&buffer[..count]);
    }

    Ok(context.compute().into())
}
fn are_files_identical(src: &Path, dst: &Path) -> io::Result<bool> {
    if !dst.exists() {
        return Ok(false);
    }

    let src_md5 = calculate_md5(src)?;
    let dst_md5 = calculate_md5(dst)?;

    Ok(src_md5 == dst_md5)
}
fn parallel_copy_dir(src: &Path, dst: &Path, is_same_disk: bool) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    let entries = fs::read_dir(src)?
        .collect::<Result<Vec<_>, io::Error>>()?;
    if is_same_disk {
        for entry in entries {
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                parallel_copy_dir(&src_path, &dst_path, true)?;
            } else if file_type.is_file() {
                if !dst_path.exists() || !are_files_identical(&src_path, &dst_path)? {
                    fs::copy(&src_path, &dst_path)?;
                }
            }
        }
    } else {
        entries.into_par_iter().try_for_each(|entry| {
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                parallel_copy_dir(&src_path, &dst_path, false)
            } else if file_type.is_file() {
                if !dst_path.exists() || !are_files_identical(&src_path, &dst_path)? {
                    fs::copy(&src_path, &dst_path).map(|_| ())
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        })?;
    }

    Ok(())
}

fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        let b = is_metadata_same(src, dst)?;
        let a=is_parallel(is_ssd(dst.to_str().unwrap()), b);
        parallel_copy_dir(src,dst,a)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        if !are_files_identical(&src ,&dst)?{
            fs::copy(src, dst)?;
        }
        Ok(())
    }
}



fn is_parallel(is_ssd:Result<bool,Box<dyn std::error::Error>>,is_same_disk:bool) -> bool {
    match is_ssd {
        Ok(ssd) => {
            if ssd {
                return false
            } else {
                return is_same_disk
            }
        },
        Err(e) => {
            panic!("{}",e)
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod command_install{
    use std::env;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use gettextrs::gettext;
    use crate::lfs::commands::command_install::{copy_file};

    use crate::lfs::tools::gettext_format::remove_trailing_newlines;
    use crate::lfs::errors::install_error::ENVINSTALLError;
    use crate::lfs::tools::constant_table::{env_utils_table,env_prompt_message,git_repo_table,vault_config };

    fn is_root() -> bool {
        unsafe {
            libc::getuid() == 0
        }
    }
    #[cfg( target_os = "linux")]
    fn get_user_home() -> PathBuf {
        let user = if is_root() {
            env::var("SUDO_USER").ok()
        } else {
            env::var("USER").ok()
        };
        if let Some(username) = user {
            let output = Command::new("getent")
                .args(&["passwd", &username])
                .output()
                .expect("failed to get user info");
            if output.status.success() {
                let user_info = String::from_utf8_lossy(&output.stdout);
                if let Some(home_dir) = user_info.split(':').nth(5) {
                    return PathBuf::from(home_dir.trim().join(git_repo_table::GitRepoCharacters::get(
                        git_repo_table::GitRepo::GITCONFIG
                    )));
                }
            }
        }
        env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| {
            panic!("{}", env_prompt_message::ENVPromptMsgCharacters::get(
                env_prompt_message::ENVPromptMsg::HOME_DIR_ERROR
            ))
        })
    }
    #[cfg( target_os = "macos")]
    fn get_user_home() -> PathBuf {
        match env::var(env_utils_table::ENVIRONMENTCharacters::get(
            env_utils_table::ENVIRONMENTEnum::USER_HOME
        )) {
            Ok(home_dir) => {
                let mut config_path = PathBuf::from(home_dir);
                config_path.push(git_repo_table::GitRepoCharacters::get(
                    git_repo_table::GitRepo::GITCONFIG
                ));
                config_path
            },
            Err(e) => {
                panic!("{} {}", gettext(
                    env_prompt_message::ENVPromptMsgCharacters::get(
                        env_prompt_message::ENVPromptMsg::HOME_DIR_ERROR
                    )
                ),e);
            },
        }
    }
    fn set_git_vault_filter(user_git_config_path: PathBuf) -> Result<(), ENVINSTALLError> {
        if !user_git_config_path.exists() {
            let error_msg = remove_trailing_newlines(
                gettext(
                    env_prompt_message::ENVPromptMsgCharacters::get(
                        env_prompt_message::ENVPromptMsg::GITCONFIG_NOT_EXIST_ERROR
                    )
                )
            );
            return Err(ENVINSTALLError::new(format!("{} {:?}",error_msg,user_git_config_path)));
        }
        let commands = vec![
            (vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::SMUDGE_KEY
            ), vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::SMUDGE_VALUE
            )),
            (vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::CLEAN_KEY
            ), vault_config::VaultConfigEnumCharacters::get(
                vault_config::VaultConfigEnum::CLEAN_VALUE
            )),
        ];
        for (key, value) in commands {
            let output = Command::new("git")
                .args(&["config", "--global", key, value])
                .output();
            match output {
                Ok(output) if output.status.success() => continue,
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(ENVINSTALLError::new(
                        format!("{} {}", env_prompt_message::ENVPromptMsgCharacters::get(
                            env_prompt_message::ENVPromptMsg::GITCONFIG_ERROR
                        ),stderr
                        )
                    ));
                }
                Err(e) => {
                    let error_msg = remove_trailing_newlines(
                        gettext(env_prompt_message::ENVPromptMsgCharacters::get(
                            env_prompt_message::ENVPromptMsg::FAILED_GIT_CONFIG
                        ))
                    );
                    return Err(ENVINSTALLError::new(
                        format!("{}{}",error_msg,e)
                    ));
                }
            }
        }
        Ok(())
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
            match set_git_vault_filter(get_user_home()) {
                Ok(()) =>{
                    print!("{}", env_prompt_message::ENVPromptMsgCharacters::get(
                        env_prompt_message::ENVPromptMsg::VAULT_CONFIG_SUCCESS
                    ));
                },
                Err(e) => {
                    panic!("{}",e);
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