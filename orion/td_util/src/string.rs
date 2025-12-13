/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */


use std::fmt;
use std::ops::Deref;

use internment::Intern;
use parse_display::Display;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::Visitor;

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Display)]
#[display("{0}")]
pub struct InternString(Intern<String>);

impl fmt::Debug for InternString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "i{:?}", self.as_str())
    }
}

struct InternStringVisitor;

impl<'de> Visitor<'de> for InternStringVisitor {
    type Value = InternString;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(InternString::new(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(InternString::from_string(v))
    }
}

impl<'de> Deserialize<'de> for InternString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(InternStringVisitor)
    }
}

impl Serialize for InternString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl Default for InternString {
    fn default() -> Self {
        Self::new("")
    }
}

impl Deref for InternString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<'a> PartialEq<&'a str> for InternString {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str() == *other
    }
}

impl<'a> PartialEq<InternString> for &'a str {
    fn eq(&self, other: &InternString) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<String> for InternString {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<InternString> for String {
    fn eq(&self, other: &InternString) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for InternString {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl InternString {
    pub fn new(x: &str) -> Self {
        InternString(Intern::new(x.to_string()))
    }

    pub fn new3(x: &str, y: &str, z: &str) -> Self {
        let s = format!("{}{}{}", x, y, z);
        InternString(Intern::new(s))
    }

    pub fn from_string(x: String) -> Self {
        InternString(Intern::new(x))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.as_str().starts_with(prefix)
    }
    pub fn split_once(&self, sep: char) -> Option<(&str, &str)> {
        self.as_str().split_once(sep)
    }
    pub fn strip_prefix(&self, prefix: &str) -> Option<&str> {
        self.as_str().strip_prefix(prefix)
    }
    pub fn contains(&self, needle: &str) -> bool {
        self.as_str().contains(needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_string() {
        assert_eq!(
            InternString::new("abcdef"),
            InternString::new3("ab", "cde", "f")
        );
        assert_ne!(
            InternString::new("abcdefgh"),
            InternString::new3("ab", "", "defg!")
        );
    }

    #[test]
    fn test_traits() {
        let s = InternString::new("hello");
        let s2 = s; 
        assert_eq!(s, s2);
        
        assert_eq!(s.len(), 5); 
        
        assert_eq!(s, "hello");
        assert_eq!("hello", s);
    }
}