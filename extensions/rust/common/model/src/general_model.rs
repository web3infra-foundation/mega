use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Version {
    pub name: String,
    pub version: String,
}

impl Version {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct VersionWithTag {
    pub name: String,
    pub version: String,
    pub git_url: String,
    pub tag: String,
}

impl VersionWithTag {
    pub fn new(name: &str, version: &str, git_url: &str, tag: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            git_url: git_url.to_string(),
            tag: tag.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Program {
    pub name: String,
    pub mega_url: String,
}

impl Program {
    pub fn new(name: &str, mega_url: &str) -> Self {
        Self {
            name: name.to_string(),
            mega_url: mega_url.to_string(),
        }
    }
}
