use std::io;
use std::io::Write;
use gettextrs::gettext;
use std::string::String;

use crate::lfs::errors::track_error::DefaultGitAttributesError;
use crate::lfs::tools::git_attributes_manager::{DefaultGitAttributesManager, GitAttributesManger};
use crate::lfs::tools::constant_table::{track_prompt_message,git_attributes_table};

fn print_attributes<'a,I>(attributes :I) ->io::Result<()>
    where I:IntoIterator<Item = &'a str>
{
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for attribute in attributes {
        writeln!(handle,"{}",attribute)?;
    }
    Ok(())
}

fn filter_lfs_attribute<'a>(v: &'a[String]) -> Vec<&'a str> {
    v.iter()
        .filter(|line| line.contains(
            git_attributes_table::GitAttributesPatterns::get(
                git_attributes_table::GitAttributesPatternsEnum::CONFIGURATION
            )
        ))
        .map(|line| line.as_str())
        .collect()
}

fn remove_lfs_attributes<'a,I>(lines:I) -> Vec<String>
    where I:IntoIterator<Item = &'a str>
{
    lines.into_iter()
        .map(|line| line.replace(
            git_attributes_table::GitAttributesPatterns::get(
                git_attributes_table::GitAttributesPatternsEnum::CONFIGURATION
            ),""
        ))
        .collect()
}

pub fn track_command(manager: &DefaultGitAttributesManager,pattern: Option<String>) -> Result<(),DefaultGitAttributesError> {
    if let Some(p) = pattern {
        manager.add_pattern(&p)?
    } else {
        print!("{}", gettext(
            track_prompt_message::TrackPromptMsgCharacters::get(
                track_prompt_message::TrackPromptMsg::LISTING
            )
        ));
        let attributes = manager.read_attributes()?;
        let filtered_attributes = filter_lfs_attribute(&attributes);
        let cleaned_attributes = remove_lfs_attributes(filtered_attributes.iter().copied());
        print_attributes(cleaned_attributes.iter().map(String::as_str))?;
    };
    Ok(())
}