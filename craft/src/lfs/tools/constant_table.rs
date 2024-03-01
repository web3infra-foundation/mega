#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
pub mod git_attributes_table {
    use crate::create_characters;
    create_characters! {
        GitAttributesPatternsEnum,
     GitAttributesPatterns {
        SPACE_PATTERN: "[[:space:]]",
        CROSSFIRE_PATTERN: "/",
        CONFIGURATION:"filter=lfs diff=lfs merge=lfs -text"
     }
   }
    create_characters! {
        GitAttributesCharactersEnum,
    GitAttributesCharacters {
        SPACE: " ",
        CROSSFIRE: "\\",
    }
  }
}
pub mod vault_config {
    use crate::create_characters;
    create_characters!{
        VaultConfigEnum,
        VaultConfigEnumCharacters{
            SMUDGE_KEY:"filter.vault.smudge",
            SMUDGE_VALUE:"git-craft vault decrypt -n craft",
            CLEAN_KEY:"filter.vault.clean",
            CLEAN_VALUE:"git-craft vault encrypt -n craft"
        }
    }
}
pub mod get_locale_prompt_message {
    use crate::create_characters;
    create_characters!{
        GetLocalePromptMsg,
        GetLocalePromptMsgCharacters{
            FAIL:"Failed to get system zone code, now runs as en_US.UTF-8\n",
            FAIL_ENV_LANG:"Failed to obtain environment variables\n",
            SetThreadLocaleError:"Thread locale operation failed\n",
            RestoreThreadLocaleError:"Restore thread locale operation failed\n"
        }
    }
}

pub mod env_utils_table{
    use crate::create_characters;
    create_characters!{
        ENVIRONMENTEnum,
        ENVIRONMENTCharacters{
            ENVIRONMENT:"Environment",
            PROGRAMDIR_WIN:r#"target\release\git-craft.exe"#,
            TRANSLATIONSDIR_WIN:r#"target\translations"#,
            PATH:"path",
            PROGRAMDIR_DESTINATIONPATH_WIN:r#"C:\Program Files\git-craft\git-craft.exe"#,
            TRANSLATIONS_DESTINATIONPATH_WIN:r#"C:\Program Files\git-craft\translations"#,
            ENVDIR_WIN:r#"C:\Program Files\git-craft\"#,
            PROGRAMDIR_Unix_Like:r#"target/release/git-craft"#,
            TRANSLATIONSDIR_Unix_Like:r#"target/translations"#,
            PROGRAMDIR_DESTINATIONPATH_Unix_Like:r#"/usr/local/bin/git-craft"#,
            TRANSLATIONS_DESTINATIONPATH_Unix_Like:r#"/usr/local/share/git-craft/translations"#,
            USER_HOME:"HOME"
        }
    }
}
pub mod env_prompt_message{
    use crate::create_characters;
    create_characters!{
        ENVPromptMsg,
        ENVPromptMsgCharacters{
            GitCraftSUCCESS:"git craft copy successful\n",
            GitCraftFAILED:"git craft copy failed\n",
            TranslationsSUCCESS:"translations folder copy successful\n",
            TranslationsFAILED:"translations folder copy failed\n",
            DIRCODError:"The current working path contains invalid UTF-8 characters\n",
            NOT_ROOT_RUN:"Please run this command with root privileges\n",
            HOME_DIR_ERROR:"Couldn't read the HOME environment variable:\n",
            GITCONFIG_NOT_EXIST_ERROR:"Git config file does not exist at:\n",
            GITCONFIG_ERROR:"git config error:",
            FAILED_GIT_CONFIG:"Failed to execute git config:\n",
            VAULT_CONFIG_SUCCESS:"Successfully updated git config with vault filter.\n",
            PATH_ERROR:"This path is abnormal"
        }
    }
}
pub mod osget_locale_error {
    use crate::create_characters;
    create_characters!{
        OSGetLocaleErrorMsg,
        OSGetLocaleErrorMsgCharacters{
            ERROE:"OsString to String conversion error occurred\n",
            LCIDError:"Obtained unrecognized LCID\n"
        }
    }
}
pub mod track_prompt_message {
    use crate::create_characters;
    create_characters!{
        TrackPromptMsg,
        TrackPromptMsgCharacters{
            EXIST: "already supported\n",
            SUCCESS: "add to tracking success\n",
            LISTING:"Listing tracked patterns\n"
        }
    }
}

pub mod untrack_prompt_message {
    use crate::create_characters;
    create_characters!{
        UntrackPromptMsg,
        UntrackPromptMsgCharacters{
            PATTERNNONE:"git craft lfs untrack <path> [path]*\n",
            NONE:"This pattern does not exist in .gitattributes\n",
            UNTRACK:"Untracking\n",
            ERRUNTRACK:"An error occurred while removing tracking:\n"
        }
    }
}
pub mod git_repo_table {
    use crate::create_characters;
    create_characters! {
        GitRepo,
    GitRepoCharacters {
        GIT: ".git",
        GITATTRIBUTES: ".gitattributes",
        GITCONFIG:".gitconfig"
    }
  }
}

pub mod git_attributes_error_table {
    use crate::create_characters;
    create_characters! {
        GitAttributesError,
        GitAttributesErrorCharacters {
            GITREADFAILED: ".gitattributes reading failed\n",
             GITATTRIBUTESWRITEFAIED:".gitattributes writing failed\n",
        }
    }
}
pub mod git_repository_checker_error {
    use crate::create_characters;
    create_characters! {
        GitRepositoryCheckerError,
        GitRepositoryCheckerErrorCharacters {
            GITDIRERROR:"An error occurred during git directory check.\n",
        }
    }
}
pub mod default_git_attributes_error_table {
    use crate::create_characters;
    create_characters! {
        DefaultGitAttributesError,
    DefaultGitAttributesErrorCharacters {
        GITATTRIBUTESFAILED: "Failed to create .gitattributes file.\n",
        NOTGITREPOSITORY: "Not in a Git repository.\n",
            GITDIRERROR:"An error occurred during git directory check.\n",
            GITATTRIBUTESWRITEFAIED:".gitattributes writing failed\n",
    }
}
}
#[cfg( target_os = "macos")]
pub mod disk_judgment_table {
    use crate::create_characters;
    create_characters! {
        DiskJudgmentEnum,
        DiskJudgmentEnumCharacters{
            DF:"df",
            DF_ERROR:"df command failed",
            DF_ERROR_RUNNING_ERROR:"unexpected output from df command",
            DF_PARSE_ERROR:"failed to parse device path from df output",
            DISKUTIL:"diskutil",
            INFO:"info",
            DISKUTIL_ERROE:"diskutil command failed",
            SSD:"Solid State",
            YES:"Yes"
        }
    }
}
#[cfg( target_os = "linux")]
pub mod disk_judgment_table {
    use crate::create_characters;
    create_characters! {
        DiskJudgmentEnum,
        DiskJudgmentEnumCharacters{
            RAID_LVM_ERROR:"May be complex LVM logical volumes or RAID arrays",
            MOUNTS_DIR:"/proc/mounts",
            FINDMNT_ERROR:"findmnt command failed",
            DEV:"/dev",
            DEV_:"/dev/",
            MAPPER:"/dev/mapper/",
            BLOCK:"/sys/block",
            ROTATIONAL:"queue/rotational",
            UNICODE_ERROR:"Non-unicode device name",
            DEVICE_ERROE:"Invalid device name",
            UNICODE_DEVICE_ERROR:"Non-unicode device name or Invalid device name"
        }
    }
}
#[cfg(target_os = "windows")]
pub mod disk_judgment_error {
    use crate::create_characters;
    create_characters! {
        DiskJudgmentEnum,
        DiskJudgmentEnumCharacters{
            HANDLE_ERROR:"Windows handle error",
            DEVICE_IO_CONTROL_ERROR:"DeviceIoControl execution failed",
            DRIVE_LETTER_ERROR:"Exception in getting drive letter"
        }
    }
}