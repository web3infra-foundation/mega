use std::fs::{create_dir_all, File, read_dir};
use std::{fs, io};
use std::io::{BufReader, Read};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use gettextrs::gettext;
use rayon::prelude::*;
use crate::lfs::commands::command_install::disk_judgment::is_ssd;
use crate::lfs::errors::install_error::ENVINSTALLError;
use crate::lfs::tools::constant_table::env_prompt_message;
#[cfg( target_os = "macos")]
mod disk_judgment {
    use std::error::Error;
    use std::fs;
    use std::process::Command;
    use crate::lfs::tools::constant_table::disk_judgment_table;
    pub fn is_ssd(path: &str) -> Result<bool, Box<dyn Error>> {
        let device_path = if fs::metadata(path).is_ok() {
            let output_df = Command::new(
                disk_judgment_table::DiskJudgmentEnumCharacters::get(
                    disk_judgment_table::DiskJudgmentEnum::DF
                )
            )
                .arg(path)
                .output()?;

            if !output_df.status.success() {
                eprintln!("{}", disk_judgment_table::DiskJudgmentEnumCharacters::get(
                    disk_judgment_table::DiskJudgmentEnum::DF_ERROR
                ));
                return  Ok(false)
            }
            let output_str_df = std::str::from_utf8(&output_df.stdout)?;
            let lines: Vec<&str> = output_str_df.lines().collect();
            if lines.len() < 2 {
                eprintln!("{}",disk_judgment_table::DiskJudgmentEnumCharacters::get(
                    disk_judgment_table::DiskJudgmentEnum::DF_ERROR_RUNNING_ERROR
                ));
                return  Ok(false)
            }
            let maybe_path = lines.get(1).and_then(|line| line.split_whitespace().next());

            let path_str = match maybe_path {
                Some(path) => path.to_string(),
                None => {
                    eprintln!("{}", disk_judgment_table::DiskJudgmentEnumCharacters::get(
                        disk_judgment_table::DiskJudgmentEnum::DF_PARSE_ERROR
                    ));
                    String::new()
                }
            };
            path_str
        } else {
            path.to_string()
        };

        if device_path.is_empty() {
            return Ok(false);
        }

        let output_diskutil = Command::new(
            disk_judgment_table::DiskJudgmentEnumCharacters::get(
                disk_judgment_table::DiskJudgmentEnum::DISKUTIL
            )
        )
            .arg(
                disk_judgment_table::DiskJudgmentEnumCharacters::get(
                    disk_judgment_table::DiskJudgmentEnum::INFO
                )
            )
            .arg(&device_path)
            .output()?;

        if !output_diskutil.status.success() {
            eprintln!("{}",disk_judgment_table::DiskJudgmentEnumCharacters::get(
                disk_judgment_table::DiskJudgmentEnum::DISKUTIL_ERROE
            ));
            return Ok(false)
        }
        let output_str_diskutil = std::str::from_utf8(&output_diskutil.stdout)?;
        let is_ssd = output_str_diskutil.lines()
            .filter(|line| line.contains(
                disk_judgment_table::DiskJudgmentEnumCharacters::get(
                    disk_judgment_table::DiskJudgmentEnum::SSD
                )
            ))
            .any(|line| line.contains(
                disk_judgment_table::DiskJudgmentEnumCharacters::get(
                    disk_judgment_table::DiskJudgmentEnum::YES
                )
            ));

        Ok(is_ssd)
    }
}
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
        parallel_copy_dir(src,dst,is_parallel(is_ssd(dst.to_str().unwrap()), is_metadata_same(src, dst)?))
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

fn is_metadata_same(src: &Path, dst: &Path) -> Result<bool,std::io::Error>{
    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;
    Ok(src_meta.dev() == dst_meta.dev())
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
    use crate::lfs::commands::command_install::{copy_file, is_metadata_same, is_parallel, parallel_copy_dir};

    use crate::lfs::tools::gettext_format::remove_trailing_newlines;
    use crate::lfs::commands::command_install::disk_judgment::is_ssd;
    use crate::lfs::errors::install_error::ENVINSTALLError;
    use crate::lfs::tools::constant_table::{env_utils_table,env_prompt_message,git_repo_table,vault_config };

    fn is_root() -> bool {
        unsafe {
            libc::getuid() == 0
        }
    }
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