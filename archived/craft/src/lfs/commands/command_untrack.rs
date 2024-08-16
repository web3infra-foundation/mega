use gettextrs::gettext;

use crate::lfs::{
    errors::track_error::DefaultGitAttributesError,
    tools::{
        constant_table::{git_attributes_table, untrack_prompt_message},
        git_attributes_manager::{DefaultGitAttributesManager, GitAttributesManger},
    },
};

fn contains_lfs_configuration(lines:&[String],pattern:&str) -> bool {
    lines.iter().any(|line| line.contains(pattern))
}
fn replaced_pattern(pattren:String) -> String {
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
    lfs_track_string
}
pub fn untrack_command(manager: &DefaultGitAttributesManager,pattern: Option<String>) -> Result<(),DefaultGitAttributesError> {
    if let Some( p) = pattern.clone() {
        let attributes = manager.read_attributes()?;
        let lfs_track_string = replaced_pattern(p.clone());
        if !contains_lfs_configuration(&attributes,&lfs_track_string) {
            println!("{}", gettext(untrack_prompt_message::UntrackPromptMsgCharacters::get(
                untrack_prompt_message::UntrackPromptMsg::NONE
            )));
            return Ok(())
        }
        match manager.remove_pattern(&lfs_track_string) {
            Ok(_) =>{
                println!("{} {}",pattern.as_ref().unwrap(),
                gettext(untrack_prompt_message::UntrackPromptMsgCharacters::get(
                    untrack_prompt_message::UntrackPromptMsg::UNTRACK
                ))
                )
            }
            Err(e) => {
                return  Err(DefaultGitAttributesError::with_source(gettext(
                    untrack_prompt_message::UntrackPromptMsgCharacters::get(
                        untrack_prompt_message::UntrackPromptMsg::ERRUNTRACK
                    )
                ),e))
            }
        };
    } else {
        println!("{}",untrack_prompt_message::UntrackPromptMsgCharacters::get(
            untrack_prompt_message::UntrackPromptMsg::PATTERNNONE
        ))
    }
    Ok(())
}