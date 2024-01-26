use std::{collections::HashSet, fs, io::{ BufRead, BufReader, Write}, path::Path as StdPath,env};
use std::io::{Seek, SeekFrom};

use fs2::FileExt;
use clap::{Args, Subcommand};

use crate::lfs::constant_table::{CROSSFIRE_PATTERN, SPACE_PATTERN, SPACE, CROSSFIRE};
use crate::lfs::lfs_error::TrackLfsError;

#[derive(Args,Debug)]
pub struct LfsArgs{
    #[command(subcommand)]
    action:LfsCommands,
}
#[derive(Subcommand,Debug,Clone,PartialEq)]
 enum LfsCommands{
    Track{
        pattern:Option<String>,
    },
    Untrack{
        pattern:String,
    }
}

    pub fn handle(args: LfsArgs){
        match args.action {
            LfsCommands::Track { pattern } => {
                match pattern {
                    Some(p) => {
                        if let Err(e) = track_lfs_files(&DefaultGitRepositoryChecker, &p){
                            eprintln!("When the missing person appears: {}", e);
                        }
                    },
                    None => {
                        if let Err(e) = track_lfs_files_pattern_empty(&DefaultGitRepositoryChecker){
                            eprintln!("Error when listing patterns: {}", e);
                        }
                    }
                }
            },
           LfsCommands::Untrack {pattern} => {
               if let Err(e) = untrack_lfs_files(&DefaultGitRepositoryChecker,&pattern){
                   eprintln!("When the missing person appears: {}", e);
               }
           }
        }
    }
trait GitRepositoryChecker {
    fn is_git_repository(&self) -> bool;
}
struct DefaultGitRepositoryChecker;

impl GitRepositoryChecker for DefaultGitRepositoryChecker {
    fn is_git_repository(&self) -> bool {
        StdPath::new(".git").exists()
    }
}
fn track_lfs_files_pattern_empty<T: GitRepositoryChecker>(checker: &T)-> Result<(), TrackLfsError> {
    if !checker.is_git_repository() {
        return Err(TrackLfsError::from(TrackLfsError::NotAGitRepository));
    }
    let git_attributes_path = StdPath::new(".gitattributes");
    let file = fs::OpenOptions::new()
            .read(true)
            .open(git_attributes_path)?;
    let file_for_lock = file.try_clone()?;
    file_for_lock.lock_shared()?;
    let reader = BufReader::new(file);
    let mut tracked_patterns = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.contains("filter=lfs") {
            tracked_patterns.push(line);
        }
    }
    file_for_lock.sync_all()?;
    file_for_lock.unlock()?;
    if tracked_patterns.is_empty() {
        println!("No patterns are currently being tracked by LFS.");
    } else {
        println!("Currently tracked patterns by LFS:");
        for pattern in tracked_patterns {
            println!("{}", pattern);
        }
    }

    Ok(())
}
fn track_lfs_files<T: GitRepositoryChecker>(checker: &T, pattern: &str) -> Result<(), TrackLfsError> {
    if !checker.is_git_repository() {
        return Err(TrackLfsError::from(TrackLfsError::NotAGitRepository));
    }
    let git_attributes_path = StdPath::new(".gitattributes");

    let space_replaced_pattern = pattern
        .replace(SPACE,SPACE_PATTERN)//常量表
        .replace(CROSSFIRE,CROSSFIRE_PATTERN);
    let lfs_track_string = format!("{} filter=lfs diff=lfs merge=lfs -text\n",space_replaced_pattern);

    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(git_attributes_path)?;

    let _lock = file.lock_exclusive()?;

    let mut existing_patterns = HashSet::new();
    let reader  = BufReader::new(&file);
    for line in reader.lines() {
        let line = line?;
        if line.contains("filter=lfs"){
            existing_patterns.insert(line);
        }
    }
    if existing_patterns.contains(&lfs_track_string.trim_end().to_string()){
        file.unlock()?;
        println!("Pattern '{}' is already being tracked.",space_replaced_pattern);
        return Ok(())
    } else {
        file.write_all(lfs_track_string.as_bytes())?;
        println!("Pattern '{}' is now being tracked by LFS..",space_replaced_pattern);
    }
    file.sync_all()?;
    file.unlock()?;
    Ok(())
}
fn untrack_lfs_files<T: GitRepositoryChecker>(checker: &T, pattern: &str) -> Result<(), TrackLfsError> {
    if !checker.is_git_repository() {
        return Err(TrackLfsError::from(TrackLfsError::NotAGitRepository));
    }
    let git_attributes_path = StdPath::new(".gitattributes");
    let space_replaced_pattern = pattern
        .replace(SPACE,SPACE_PATTERN)
        .replace(CROSSFIRE,CROSSFIRE_PATTERN);
    let lfs_track_string = format!("{} filter=lfs diff=lfs merge=lfs -text\n",space_replaced_pattern);
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(git_attributes_path)?;
    let _lock = file.lock_exclusive()?;
    let reader = BufReader::new(&file);
    let mut attributes_lines: Vec<String> = Vec::new();
    let mut found = false;

    for line in reader.lines() {
        let line = line?;
        if line.trim_end() != lfs_track_string.trim_end() {
            attributes_lines.push(line);
        } else {
            found = true;
        }
    }

    if found {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        for line in attributes_lines {
            writeln!(file, "{}", line)?;
        }
        println!("Pattern '{}' is no longer tracked by LFS.", space_replaced_pattern);
    } else {
        println!("Pattern '{}' was not tracked by LFS.", space_replaced_pattern);
    }

    file.sync_all()?;
    file.unlock()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs};

    use tempfile::tempdir;

    use crate::lfs::lfs_error::TestTrackLfsError;

    struct MockGitRepositoryChecker{
        is_git_repo: bool,
    }

    impl MockGitRepositoryChecker {
        fn new(is_git_repo: bool) -> MockGitRepositoryChecker {
            MockGitRepositoryChecker { is_git_repo }
        }
    }
    impl GitRepositoryChecker for MockGitRepositoryChecker {
        fn is_git_repository(&self) -> bool {
            self.is_git_repo
        }
    }
    #[test]
    fn test_track_lfs_files_new_pattern() -> Result<(), TestTrackLfsError>{
        let dir = tempdir().unwrap();
        let checker = MockGitRepositoryChecker::new(true);
        let git_attributes_path = dir.path().join(".gitattributes");
        fs::write(&git_attributes_path, "").unwrap();
        env::set_current_dir(&dir).map_err(|e| TestTrackLfsError::IoError(e))?;
        let _current_dir = env::current_dir().map_err(TestTrackLfsError::CurrentDirectoryError)?;
        println!("Current directory is {}", _current_dir.display());
        let pattern = "test/*.a";
        let result = track_lfs_files(&checker,pattern);
        assert!(result.is_ok());
        let contents = fs::read_to_string(git_attributes_path).unwrap();
        let space_replaced_pattern = pattern
            .replace(SPACE,SPACE_PATTERN)
            .replace(CROSSFIRE,CROSSFIRE_PATTERN);
        assert!(contents.contains(&format!("{} filter=lfs diff=lfs merge=lfs -text\n", space_replaced_pattern)));
        Ok(())
    }

    #[test]
    fn test_track_lfs_files_existing_pattern() -> Result<(), TestTrackLfsError> {
        let dir = tempdir().unwrap();
        let checker = MockGitRepositoryChecker::new(true);
        let git_attributes_path = dir.path().join(".gitattributes");
        let existing_pattern = "test/*.a filter=lfs diff=lfs merge=lfs -text\n";
        fs::write(&git_attributes_path, existing_pattern).unwrap();
        env::set_current_dir(&dir).map_err(|e| TestTrackLfsError::IoError(e))?;
        let _current_dir = env::current_dir().map_err(TestTrackLfsError::CurrentDirectoryError);
        let pattern = "test/*.a";
        let result = track_lfs_files(&checker,pattern);
        assert!(result.is_ok());
        let contents = fs::read_to_string(git_attributes_path).unwrap();
        assert_eq!(contents, existing_pattern);
        Ok(())
    }
    #[test]
    fn test_track_lfs_files_not_git_repo() -> Result<(), TestTrackLfsError> {
        let checker = MockGitRepositoryChecker::new(false);
        let dir = tempdir()?;
        env::set_current_dir(&dir).map_err(|e| TestTrackLfsError::IoError(e))?;
        let _current_dir = env::current_dir().map_err(TestTrackLfsError::CurrentDirectoryError);
        let pattern = "test_pattern";
        let result = track_lfs_files(&checker, pattern);
        assert!(matches!(result, Err(TrackLfsError::NotAGitRepository)));
        Ok(())
    }
    #[test]
    fn test_untrack_lfs_files_not_git_repo() -> Result<(), TestTrackLfsError> {
        let checker = MockGitRepositoryChecker::new(false);
        let dir = tempdir()?;
        env::set_current_dir(&dir).map_err(|e| TestTrackLfsError::IoError(e))?;
        let _current_dir = env::current_dir().map_err(TestTrackLfsError::CurrentDirectoryError);
        let pattern = "test_pattern";
        let result = untrack_lfs_files(&checker, pattern);
        assert!(matches!(result, Err(TrackLfsError::NotAGitRepository)));
        Ok(())
    }
    #[test]
    fn test_untrack_lfs_files_success() -> Result<(), TestTrackLfsError> {
        let checker = MockGitRepositoryChecker::new(true);
        let pattern = "some_pattern";
        let dir = tempdir()?;
        env::set_current_dir(&dir).map_err(|e| TestTrackLfsError::IoError(e))?;
        let _current_dir = env::current_dir().map_err(TestTrackLfsError::CurrentDirectoryError);
        let git_attributes_path = dir.path().join(".gitattributes");
        let mut file = fs::File::create(&git_attributes_path)?;
        writeln!(file, "{} filter=lfs diff=lfs merge=lfs -text", pattern)?;
        let result = untrack_lfs_files(&checker, pattern);
        assert!(result.is_ok());
        let contents = fs::read_to_string(git_attributes_path)?;
        assert!(!contents.contains(&format!("{} filter=lfs diff=lfs merge=lfs -text", pattern)));
        Ok(())
    }
}

