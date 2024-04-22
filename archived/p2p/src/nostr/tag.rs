use std::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeSeq;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tag {
    Generic(TagKind, Vec<String>),
}

/// Tag kind
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TagKind {
    /// Public key
    P,
    PeerId,
    RepoName,
    RepoTarget,
    RepoAction,
    RepoUrl,
    RepoCommitId,
    RepoIssueContent,
    /// Custom tag kind
    Custom(String),
}

/// [`Tag`] error
#[derive(Debug)]
pub enum Error {
    /// Impossible to parse [`Tag`]
    TagParseError,
    /// Impossible to find tag kind
    KindNotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TagParseError => write!(f, "Impossible to parse tag"),
            Self::KindNotFound => write!(f, "Impossible to find tag kind"),
        }
    }
}

impl Tag {
    /// Parse [`Tag`] from string vector
    pub fn parse<S>(data: Vec<S>) -> Result<Self, Error>
        where
            S: AsRef<str>,
    {
        Tag::try_from(data)
    }

    /// Get [`Tag`] as string vector
    pub fn as_vec(&self) -> Vec<String> {
        self.clone().into()
    }
}

impl<S> TryFrom<Vec<S>> for Tag
    where
        S: AsRef<str>,
{
    type Error = Error;

    fn try_from(tag: Vec<S>) -> Result<Self, Error> {
        let tag_kind: TagKind = match tag.first() {
            Some(kind) => TagKind::from(kind),
            None => return Err(Error::KindNotFound),
        };
        {
            Ok(Self::Generic(
                tag_kind,
                tag[1..].iter().map(|s| s.as_ref().to_owned()).collect(),
            ))
        }
    }
}

impl From<Tag> for Vec<String> {
    fn from(data: Tag) -> Self {
        match data {
            Tag::Generic(kind, data) => [vec![kind.to_string()], data].concat(),
        }
    }
}

impl fmt::Display for TagKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::P => write!(f, "p"),
            Self::PeerId => write!(f, "peer_id"),
            Self::RepoName => write!(f, "repo_name"),
            Self::RepoTarget => write!(f, "repo_target"),
            Self::RepoAction => write!(f, "repo_action"),
            Self::RepoUrl => write!(f, "repo_url"),
            Self::RepoCommitId => write!(f, "repo_commit_id"),
            Self::RepoIssueContent => write!(f, "repo_issue_content"),
            Self::Custom(tag) => write!(f, "{tag}"),
        }
    }
}

impl<S> From<S> for TagKind
    where
        S: AsRef<str>,
{
    fn from(tag: S) -> Self {
        match tag.as_ref() {
            "p" => Self::P,
            "peer_id" => Self::PeerId,
            "repo_name" => Self::RepoName,
            "repo_target" => Self::RepoTarget,
            "repo_action" => Self::RepoAction,
            "repo_url" => Self::RepoUrl,
            "repo_commit_id" => Self::RepoCommitId,
            "repo_issue_content" => Self::RepoIssueContent,
            t => Self::Custom(t.to_owned()),
        }
    }
}

impl From<TagKind> for String {
    fn from(e: TagKind) -> String {
        match e {
            TagKind::P => "p".to_string(),
            TagKind::PeerId => "peer_id".to_string(),
            TagKind::RepoName => "repo_name".to_string(),
            TagKind::RepoTarget => "repo_target".to_string(),
            TagKind::RepoAction => "repo_action".to_string(),
            TagKind::RepoUrl => "repo_url".to_string(),
            TagKind::RepoCommitId => "repo_commit_id".to_string(),
            TagKind::RepoIssueContent => "repo_issue_content".to_string(),
            TagKind::Custom(tag) => tag.to_string(),
        }
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let data: Vec<String> = self.as_vec();
        let mut seq = serializer.serialize_seq(Some(data.len()))?;
        for element in data.into_iter() {
            seq.serialize_element(&element)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        type Data = Vec<String>;
        let vec: Vec<String> = Data::deserialize(deserializer)?;
        Self::try_from(vec).map_err(serde::de::Error::custom)
    }
}
