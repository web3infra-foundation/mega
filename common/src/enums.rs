// src/enums.rs

//! This file is used to share enums across different modules.
//!
//! By defining enums in a separate file, we can easily import and use them
//! in multiple modules within the project. This promotes code reuse and
//! consistency, especially when multiple modules need to work with the same
//! set of enum variants.

use std::str::FromStr;

/// An enum representing different oauth types.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SupportOauthType {
    GitHub,
}

impl FromStr for SupportOauthType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "github" => Ok(Self::GitHub),
            _ => Err(format!("'{s}' is not a valid oauth type")),
        }
    }
}
