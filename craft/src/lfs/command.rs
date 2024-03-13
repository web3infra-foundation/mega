use clap::{Args, Subcommand};
use gettextrs::*;

use crate::lfs::errors::get_locale::get_locale;
use crate::lfs::tools::git_attributes_manager::{DefaultGitAttributesManager};
use crate::lfs::tools::constant_table::env_utils_table;
use crate::lfs::tools::locale_tools;
use crate::lfs::commands::{command_track,command_untrack,command_install::command_install,command_clean};

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

        pattern:Option<String>,
    },
    Clean {
        pattern:String
    },
    Install

}
#[cfg(target_os = "windows")]
fn gettext_init() {
    bindtextdomain("messages",
                   env_utils_table::ENVIRONMENTCharacters::get(
                       env_utils_table::ENVIRONMENTEnum::TRANSLATIONS_DESTINATIONPATH_WIN
                   )
    );
    let domain_name = "messages";
    let codeset_name = "UTF-8";
    bind_textdomain_codeset(domain_name,codeset_name);
    textdomain("messages");
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn gettext_init() {
    bindtextdomain("messages",
                   env_utils_table::ENVIRONMENTCharacters::get(
                       env_utils_table::ENVIRONMENTEnum::TRANSLATIONS_DESTINATIONPATH_Unix_Like
                   )
    );
    let domain_name = "messages";
    let codeset_name = "UTF-8";
    bind_textdomain_codeset(domain_name,codeset_name);
    textdomain("messages");
}
pub fn handle(args: LfsArgs){
    locale_tools::with_locale(get_locale(),||{
        gettext_init();
        let manager = DefaultGitAttributesManager::new();
        match args.action {
            LfsCommands::Track { pattern } => {
                if let Err(e) = command_track::track_command(&manager, pattern){
                    eprintln!("{}", e);
                }
            },
            LfsCommands::Untrack {pattern} => {
                if let Err(e) = command_untrack::untrack_command(&manager,pattern){
                    eprintln!("Error: {}", e);
                }
            },
            LfsCommands::Install => {
                if let Err(e) = command_install::install_command(){
                    eprintln!("Error: {}", e);
                }
            },
            LfsCommands::Clean {pattern} => {
                 if let Err(e) = command_clean::clean_command(pattern) {
                     std::process::exit(1);
                 }
            }
        }
    });

}
