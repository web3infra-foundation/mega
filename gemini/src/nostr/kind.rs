use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::hash::{Hash, Hasher};

/// Event [`NostrKind`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NostrKind {
    /// Metadata (NIP01 and NIP05)
    Metadata,
    /// Short Text Note (NIP01)
    TextNote,
    /// for mega p2p use
    Mega,
    /// Custom
    Custom(u64),
}

impl NostrKind {
    /// Get [`NostrKind`] as `u32`
    pub fn as_u32(&self) -> u32 {
        self.as_u64() as u32
    }

    /// Get [`NostrKind`] as `u64`
    pub fn as_u64(&self) -> u64 {
        (*self).into()
    }
}

impl fmt::Display for NostrKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_u64())
    }
}

impl From<u64> for NostrKind {
    fn from(u: u64) -> Self {
        match u {
            0 => Self::Metadata,
            1 => Self::TextNote,
            111 => Self::Mega,
            x => Self::Custom(x),
        }
    }
}

impl From<NostrKind> for u64 {
    fn from(e: NostrKind) -> u64 {
        match e {
            NostrKind::Metadata => 0,
            NostrKind::TextNote => 1,
            NostrKind::Mega => 111,
            NostrKind::Custom(u) => u,
        }
    }
}

impl Hash for NostrKind {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.as_u64().hash(state);
    }
}

impl Serialize for NostrKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(From::from(*self))
    }
}

impl<'de> Deserialize<'de> for NostrKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u64(KindVisitor)
    }
}

struct KindVisitor;

impl Visitor<'_> for KindVisitor {
    type Value = NostrKind;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "an unsigned number")
    }

    fn visit_u64<E>(self, v: u64) -> Result<NostrKind, E>
    where
        E: Error,
    {
        Ok(From::<u64>::from(v))
    }
}
