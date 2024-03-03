
#[cfg(target_os = "windows")]
pub mod metadata_same{
    extern crate winapi;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::fileapi::GetVolumeInformationByHandleW;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::winnt::{FILE_ATTRIBUTE_NORMAL, HANDLE};
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;
    use crate::lfs::tools::constant_table::env_prompt_message;
    pub fn is_metadata_same(src: &Path, dst: &Path) -> Result<bool,io::Error> {
        let src_serial_number = get_volume_serial_number(src)?;
        let dst_serial_number = get_volume_serial_number(dst)?;
        Ok(src_serial_number == dst_serial_number)
    }
    fn get_volume_serial_number(path: &Path) -> io::Result<DWORD> {
        let file = match  OpenOptions::new()
            .read(true)
            .attributes(FILE_ATTRIBUTE_NORMAL)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
        {
            Ok(file) => file,
            Err(_) => {
                let parent_path = path.parent().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, env_prompt_message::ENVPromptMsgCharacters::get(
                        env_prompt_message::ENVPromptMsg::PATH_ERROR
                    ))
                })?;
                OpenOptions::new()
                    .read(true)
                    .attributes(FILE_ATTRIBUTE_NORMAL)
                    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                    .open(parent_path)?
            }
        };
        let handle = file.as_raw_handle() as HANDLE;
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut serial_number = 0;
        let success = unsafe {
            GetVolumeInformationByHandleW(
                handle,
                std::ptr::null_mut(),
                0,
                &mut serial_number,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        };
        if success == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(serial_number)
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod metadata_same{
    use std::{
        fs,
        io,
        os::unix::fs::MetadataExt,
        path::Path,
    };
    fn get_metadata_or_parent(path: &Path) -> io::Result<fs::Metadata> {
        let mut current_path = path.to_path_buf();
        loop {
            match current_path.metadata() {
                Ok(metadata) => return Ok(metadata),
                Err(e) => {
                    if e.kind() == io::ErrorKind::NotFound {
                        if let Some(parent) = current_path.parent() {
                            if parent.as_os_str().is_empty() {
                                return parent.metadata();
                            }
                            current_path = parent.to_path_buf();
                        } else {
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }
    pub fn is_metadata_same(src: &Path, dst: &Path) -> Result<bool,std::io::Error>{
        let src_meta = get_metadata_or_parent(src)?;
        let dst_meta =get_metadata_or_parent(dst)?;
        Ok(src_meta.dev() == dst_meta.dev())
    }
}



