#[cfg(target_os = "windows")]
pub mod disk_judgment {
    extern crate winapi;
    use std::error::Error;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, Prefix};
    use std::ptr::null_mut;
    use winapi::shared::minwindef::{DWORD};
    use winapi::um::fileapi::{ CreateFileW};
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::ioapiset::DeviceIoControl;
    use winapi::um::winioctl::{StorageDeviceProperty, STORAGE_PROPERTY_QUERY, STORAGE_PROPERTY_ID};
    use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE};
    use winapi::um::fileapi::OPEN_EXISTING;
    use winapi::ctypes::c_void;
    use crate::lfs::tools::constant_table::disk_judgment_error;
    fn get_drive_letter(path: &str) -> Option<String> {
        let path = Path::new(path);
        path.components().next().and_then(|component| match component {
            std::path::Component::Prefix(prefix_component) => match prefix_component.kind() {
                Prefix::Disk(drive_letter) | Prefix::VerbatimDisk(drive_letter) => {
                    let drive_letter_char = (drive_letter as u8 as char).to_string().to_uppercase();
                    Some(drive_letter_char)
                },
                _ => None,
            },
            _ => None,
        })
    }

    pub fn is_ssd(path: &str) -> Result<bool, Box<dyn Error>> {
         match get_drive_letter(path) {
            Some(p) =>{
                let drive_path = format!(r#"\\.\{}:"# , p);
                let wide_path: Vec<u16> = OsStr::new(&drive_path)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let lpcwstr: *const u16 = wide_path.as_ptr();
                let handle = unsafe {
                    CreateFileW(
                        lpcwstr,
                        0,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        null_mut(),
                        OPEN_EXISTING,
                        0,
                        null_mut(),
                    )
                };

                if handle.is_null() {
                    eprintln!("{}",disk_judgment_error::DiskJudgmentEnumCharacters::get(
                        disk_judgment_error::DiskJudgmentEnum::HANDLE_ERROR
                    ));
                    return Ok(false)
                }
                let mut property_query = STORAGE_PROPERTY_QUERY {
                    PropertyId: StorageDeviceProperty as STORAGE_PROPERTY_ID,
                    QueryType: 0,
                    AdditionalParameters: [0],
                };
                let mut device_descriptor = [0u8; 1024];
                let mut returned_bytes = 0;
                let result = unsafe {
                    DeviceIoControl(
                        handle,
                        winapi::um::winioctl::IOCTL_STORAGE_QUERY_PROPERTY,
                        &mut property_query as *mut _ as *mut winapi::ctypes::c_void,
                        std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as DWORD,
                        device_descriptor.as_mut_ptr() as *mut c_void,
                        device_descriptor.len() as DWORD,
                        &mut returned_bytes,
                        null_mut(),
                    )
                };
                unsafe {
                    CloseHandle(handle);
                }
                if result == 0 {
                    eprintln!("{}",disk_judgment_error::DiskJudgmentEnumCharacters::get(
                        disk_judgment_error::DiskJudgmentEnum::DEVICE_IO_CONTROL_ERROR
                    ));
                    return Ok(false)
                }
                const BUS_TYPE_NVME: DWORD = 0x11;
                let bus_type_offset = 28;
                let bus_type = device_descriptor[bus_type_offset] as DWORD;
               return  Ok(bus_type == BUS_TYPE_NVME)
            },
             None => {
                 println!("{} '{}'", disk_judgment_error::DiskJudgmentEnumCharacters::get(
                     disk_judgment_error::DiskJudgmentEnum::DRIVE_LETTER_ERROR
                 ),path);
                 return Ok(false)
             }
        }
    }
}
#[cfg( target_os = "macos")]
pub mod disk_judgment {
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