use std::fs;
use std::path::PathBuf;
use gettextrs::gettext;
use crate::lfs::commands::utils::clean_FileManager::FileManager;
use crate::lfs::errors::clean_error::CleanFilterError;
use crate::lfs::tools::constant_table::{clean_filter_error,clean_filter_table};
use crate::lfs::tools::gettext_format::remove_trailing_newlines;

pub fn clean_command(file_path:String) -> Result<(),CleanFilterError> {
    let file_manager = FileManager::new(PathBuf::from(file_path.clone()))
        .ok_or(CleanFilterError::new(
            gettext(
                clean_filter_error::CleanFilterErrorEnumCharacters::get(
                    clean_filter_error::CleanFilterErrorEnum::CLEAN_FILTER_FAILED
                )
            )
        ))?;

    match file_manager.run() {
        Ok((sha256,size)) => {
            println!("{}",format!("{}{}{}\n{}{}", clean_filter_table::CleanFilterTableCharacters::get(
                clean_filter_table::CleanFilterTableEnum::VERSION
            ), clean_filter_table::CleanFilterTableCharacters::get(
                clean_filter_table::CleanFilterTableEnum::SHA256
            ),sha256,clean_filter_table::CleanFilterTableCharacters::get(
                clean_filter_table::CleanFilterTableEnum::SIZE
            ),size));
            match dir_creat(sha256.clone()) {
                Ok(path) => {
                    let rename_path = format!("{}/{}",path,sha256);
                    match copy_and_rename(&file_path,&rename_path) {
                        Err(e) => {
                            return Err(CleanFilterError::with_source(clean_filter_error::CleanFilterErrorEnumCharacters::get(
                                clean_filter_error::CleanFilterErrorEnum::CP_FILE_FAILED
                            ),e))
                        }
                        _ => {}
                    }
                },
                Err(e) => {
                    return Err(CleanFilterError::with_source(clean_filter_error::CleanFilterErrorEnumCharacters::get(
                        clean_filter_error::CleanFilterErrorEnum::CREAT_DIR_FAILED
                    ),e))
                }
            }
            Ok(())
        }
        Err(e) => {
            let error_msg = remove_trailing_newlines(
                gettext(
                    clean_filter_error::CleanFilterErrorEnumCharacters::get(
                        clean_filter_error::CleanFilterErrorEnum::SHA256_FAILED
                    )
                )
            );
            return Err(CleanFilterError::with_source(format!("{}{}",error_msg,file_path),e))
        }
    }
}

fn dir_creat(sha256:String) -> std::io::Result<String> {
    let dir1 = &sha256[0..2];
    let dir2 = &sha256[2..4];
    let path = format!("{}{}/{}/", clean_filter_table::CleanFilterTableCharacters::get(
        clean_filter_table::CleanFilterTableEnum::GIT_LFS_OBJECT_DIR
    ),dir1, dir2);
    fs::create_dir_all(&path)?;
    Ok(path)
}
fn copy_and_rename(src: &str, dest: &str) -> std::io::Result<()> {
    fs::copy(src, dest)?;
    Ok(())
}


