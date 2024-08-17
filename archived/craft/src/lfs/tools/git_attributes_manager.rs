use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
};

use gettextrs::gettext;

use crate::lfs::{
    errors::track_error::{GitAttributesError, DefaultGitAttributesError},
    tools::{
        constant_table::{
            default_git_attributes_error_table,
            git_attributes_error_table,
            git_attributes_table,
            git_repo_table,
            track_prompt_message,
        },
        git_repository_checker::{DefaultGitRepositoryChecker, GitRepositoryChecker},
    },
};
pub trait GitAttributesManger {
    fn read_attributes(&self) -> Result<Vec<String>, GitAttributesError>;
    fn write_attributes(&self, lines: &[String]) -> Result<(), GitAttributesError>;
    fn pattern_exists(&self, pattern: &str) -> Result<bool, GitAttributesError>;
}

pub struct DefaultGitAttributesManager;

impl DefaultGitAttributesManager {
    pub(crate) fn new() -> Self {
        DefaultGitAttributesManager {}
    }
    fn current_in_git_repo(&self) -> Result<(), DefaultGitAttributesError> {
        if !DefaultGitRepositoryChecker.is_git_repository() {
            File::create(
                git_repo_table::GitRepoCharacters::get(
                    git_repo_table::GitRepo::GITATTRIBUTES
                )
            )
                .map_err(|e| DefaultGitAttributesError::with_source(
                    gettext(
                        default_git_attributes_error_table::DefaultGitAttributesErrorCharacters::get(
                            default_git_attributes_error_table::DefaultGitAttributesError::GITATTRIBUTESFAILED
                        )
                    ),
                    e,
                ))?;
            Ok(())
        } else {
            Ok(())
        }
    }
    fn check_gitattributes(&self) -> Result<(),DefaultGitAttributesError> {
        if !Path::new(
            git_repo_table::GitRepoCharacters::get(
                git_repo_table::GitRepo::GITATTRIBUTES
            )
        ).exists(){
            File::create(
                git_repo_table::GitRepoCharacters::get(
                    git_repo_table::GitRepo::GITATTRIBUTES
                )
            )?;
            Ok(())
        } else {
            Ok(())
        }
    }
    fn replaced_pattern(&self,pattren:&str) -> Result<String,DefaultGitAttributesError> {
        let lfs_replaced_pattern = pattren
            .replace(
                git_attributes_table::GitAttributesCharacters::get(
                    git_attributes_table::GitAttributesCharactersEnum::SPACE
                ),
                git_attributes_table::GitAttributesPatterns::get(
                    git_attributes_table::GitAttributesPatternsEnum::SPACE_PATTERN
                )
            )
            .replace(
                git_attributes_table::GitAttributesCharacters::get(
                    git_attributes_table::GitAttributesCharactersEnum::CROSSFIRE
                ),
                git_attributes_table::GitAttributesPatterns::get(
                    git_attributes_table::GitAttributesPatternsEnum::CROSSFIRE_PATTERN
                )
            );
        let lfs_track_string = format!(
            "{} {}",  lfs_replaced_pattern,
            git_attributes_table::GitAttributesPatterns::get(
                git_attributes_table::GitAttributesPatternsEnum::CONFIGURATION
            )
        );
        Ok(lfs_track_string)
    }
    fn replaced_characters<'a>(&self,pattern: &[&'a str]) -> Result<Vec<String>,DefaultGitAttributesError> {
        let mut replaced = Vec::new();
        for line in pattern {
            let replace_line = line.replace(
                git_attributes_table::GitAttributesPatterns::get(
                    git_attributes_table::GitAttributesPatternsEnum::SPACE_PATTERN
                ),
                git_attributes_table::GitAttributesCharacters::get(
                    git_attributes_table::GitAttributesCharactersEnum::SPACE
                )
            )
                .replace(
                    git_attributes_table::GitAttributesPatterns::get(
                        git_attributes_table::GitAttributesPatternsEnum::CROSSFIRE_PATTERN
                    ),
                    git_attributes_table::GitAttributesCharacters::get(
                        git_attributes_table::GitAttributesCharactersEnum::CROSSFIRE
                    )
                )
                .replace(
                    git_attributes_table::GitAttributesPatterns::get(
                        git_attributes_table::GitAttributesPatternsEnum::CONFIGURATION
                    ),
                    ""
                )
                .trim_end()
                .to_string();
            replaced.push(replace_line);
        }
        Ok(replaced)
    }
    pub(crate) fn add_pattern(&self, pattern: &str) -> Result<(), DefaultGitAttributesError> {
        match DefaultGitRepositoryChecker.is_git_repository_loop() {
            Ok(true) => {
                self.current_in_git_repo()?;
                self.check_gitattributes()?;
                let mut attributes = self.read_attributes()?;
                let pattern_exists = self.pattern_exists(pattern)?;
                if !pattern_exists {
                    let processed_pattern = self.replaced_pattern(pattern);
                    attributes.push(processed_pattern?);
                    match self.write_attributes(&attributes) {
                        Ok(()) => println!("{},{}",pattern,
                        gettext(
                            track_prompt_message::TrackPromptMsgCharacters::get(
                                track_prompt_message::TrackPromptMsg::SUCCESS
                            )
                        )
                        ),
                        Err(e) => Err(DefaultGitAttributesError::with_source(
                            gettext(
                                default_git_attributes_error_table::DefaultGitAttributesErrorCharacters::get(
                                    default_git_attributes_error_table::DefaultGitAttributesError::GITATTRIBUTESWRITEFAIED
                                )
                            )
                            ,e))?
                    }
                } else {
                    println!("{},{}",pattern,
                    gettext(
                        track_prompt_message::TrackPromptMsgCharacters::get(
                            track_prompt_message::TrackPromptMsg::EXIST
                        )
                    )
                    )
                }
                Ok(())
            }
            Ok(false) => {
                return Err(
                    DefaultGitAttributesError::new(
                        gettext(
                            default_git_attributes_error_table::DefaultGitAttributesErrorCharacters::get(
                                default_git_attributes_error_table::DefaultGitAttributesError::NOTGITREPOSITORY
                            )
                        )
                    )
                );
            }
            Err(e) => {
                return Err(DefaultGitAttributesError::with_source(
                    gettext(
                        default_git_attributes_error_table::DefaultGitAttributesErrorCharacters::get(
                            default_git_attributes_error_table::DefaultGitAttributesError::GITDIRERROR
                        )
                    ),
                    e,
                ));
            }
        }
    }
    pub(crate) fn remove_pattern(&self, pattern: &str) -> Result<(), DefaultGitAttributesError> {
        let mut attributes = self.read_attributes()?;
        let mut index_to_remove = None;

        for (index, line) in attributes.iter().enumerate() {
            if line.contains(pattern) {
                index_to_remove = Some(index);
                break;
            }
        }

        if let Some(index) = index_to_remove {
            attributes.remove(index);
            self.write_attributes(&attributes)?;
        }

        Ok(())
    }
}

impl GitAttributesManger for DefaultGitAttributesManager {
    fn read_attributes(&self) -> Result<Vec<String>, GitAttributesError> {
        let git_attributes_path = Path::new(
            git_repo_table::GitRepoCharacters::get(
                git_repo_table::GitRepo::GITATTRIBUTES
            )
        );
        let file = OpenOptions::new()
            .read(true).
            open(git_attributes_path).
            map_err(|e| GitAttributesError::with_source(
                gettext(
                    git_attributes_error_table::GitAttributesErrorCharacters::get(
                        git_attributes_error_table::GitAttributesError::GITREADFAILED
                    )
                ),
                e,
            ))?;
        let reader = BufReader::new(file);
        let mut attributes = Vec::new();

        for line in reader.lines() {
            let line = line?;
            attributes.push(line);
        }
        Ok(attributes)
    }

    fn write_attributes(&self, lines: &[String]) -> Result<(), GitAttributesError> {
        let git_attributes_path = Path::new(
            git_repo_table::GitRepoCharacters::get(
                git_repo_table::GitRepo::GITATTRIBUTES
            )
        );
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(git_attributes_path)
            .map_err(|e| GitAttributesError::with_source(gettext(
                git_attributes_error_table::GitAttributesErrorCharacters::get(
                    git_attributes_error_table::GitAttributesError::GITATTRIBUTESWRITEFAIED
                )
            ), e))?;

        for line in lines {
            writeln!(file, "{}", line)?;
        }

        Ok(())
    }

    fn pattern_exists(&self, pattern: &str) -> Result<bool, GitAttributesError> {
        let attributes = self.read_attributes()?;
        let  lfs_lines: Vec<&str> = attributes.iter()
            .filter(|line| line.contains(
                git_attributes_table::GitAttributesPatterns::get(
                    git_attributes_table::GitAttributesPatternsEnum::CONFIGURATION
                )
            ))
            .map(AsRef::as_ref)
            .collect();
        let replaced_pattern = pattern.replace(
            git_attributes_table::GitAttributesPatterns::get(
                git_attributes_table::GitAttributesPatternsEnum::CROSSFIRE_PATTERN
            ),
            git_attributes_table::GitAttributesCharacters::get(
                git_attributes_table::GitAttributesCharactersEnum::CROSSFIRE
            )
        );
        for line in self.replaced_characters(&lfs_lines) {
            if line.contains(&replaced_pattern.to_owned()) {
                return Ok::<bool,GitAttributesError>(true);
            }
        }
       Ok(false)
    }
}