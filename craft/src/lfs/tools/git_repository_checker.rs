use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use gettextrs::gettext;

use crate::lfs::{
    errors::track_error::GitRepositoryCheckerError,
    tools::constant_table::{git_repo_table, git_repository_checker_error},
};
pub struct DefaultGitRepositoryChecker;
pub trait GitRepositoryChecker{
    fn is_git_repository_loop(&self) -> Result<bool,GitRepositoryCheckerError>;
    fn is_git_repository(&self) -> bool;
}

impl GitRepositoryChecker for DefaultGitRepositoryChecker {
    fn is_git_repository_loop(&self) -> Result<bool,GitRepositoryCheckerError> {
        let mut current_path:PathBuf = match env::current_dir() {
           Ok(path) => path,
            Err(e) => {
                return Err(GitRepositoryCheckerError::with_source(
                    gettext(
                        git_repository_checker_error::GitRepositoryCheckerErrorCharacters::get(
                            git_repository_checker_error::GitRepositoryCheckerError::GITDIRERROR
                        )
                    ),
                    e
                ));
            }
        };
        while current_path.exists() {
            if current_path.join(".git").is_dir() {
                return Ok(true);
            }
            match current_path.parent() {
                Some(parent_path) if parent_path != current_path => {
                    current_path.pop();
                },
                _ => break,
            }
        }

        Ok(false)
    }

    fn is_git_repository(&self) -> bool {
        Path::new(
            git_repo_table::GitRepoCharacters::get(
                git_repo_table::GitRepo::GIT
            )
        ).exists()
    }
}