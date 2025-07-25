//! In a Git commit, the author signature contains the name, email address, timestamp, and timezone
//! of the person who authored the commit. This information is stored in a specific format, which
//! consists of the following fields:
//!
//! - Name: The name of the author, encoded as a UTF-8 string.
//! - Email: The email address of the author, encoded as a UTF-8 string.
//! - Timestamp: The timestamp of when the commit was authored, encoded as a decimal number of seconds
//!   since the Unix epoch (January 1, 1970, 00:00:00 UTC).
//! - Timezone: The timezone offset of the author's local time from Coordinated Universal Time (UTC),
//!   encoded as a string in the format "+HHMM" or "-HHMM".
//!
use std::{fmt::Display, str::FromStr};

use bincode::{Decode, Encode};
use bstr::ByteSlice;
use chrono::Offset;
use serde::{Deserialize, Serialize};

use crate::errors::GitError;

/// In addition to the author signature, Git also includes a "committer" signature, which indicates
/// who committed the changes to the repository. The committer signature is similar in structure to
/// the author signature, but includes the name, email address, and timestamp of the committer instead.
/// This can be useful in situations where multiple people are working on a project and changes are
/// being reviewed and merged by someone other than the original author.
///
/// In the following example, it's has the only one who authored and committed.
/// ```bash
/// author Eli Ma <eli@patch.sh> 1678102132 +0800
/// committer Quanyi Ma <eli@patch.sh> 1678102132 +0800
/// ```
///
/// So, we design a `SignatureType` enum to indicate the signature type.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Decode, Encode)]
pub enum SignatureType {
    Author,
    Committer,
    Tagger,
}

impl Display for SignatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SignatureType::Author => write!(f, "author"),
            SignatureType::Committer => write!(f, "committer"),
            SignatureType::Tagger => write!(f, "tagger"),
        }
    }
}
impl FromStr for SignatureType {
    type Err = GitError;
    /// The `from_str` method is used to convert a string to a `SignatureType` enum.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "author" => Ok(SignatureType::Author),
            "committer" => Ok(SignatureType::Committer),
            "tagger" => Ok(SignatureType::Tagger),
            _ => Err(GitError::InvalidSignatureType(s.to_string())),
        }
    }
}
impl SignatureType {
    /// The `from_data` method is used to convert a `Vec<u8>` to a `SignatureType` enum.
    pub fn from_data(data: Vec<u8>) -> Result<Self, GitError> {
        let s = String::from_utf8(data.to_vec())?;
        SignatureType::from_str(s.as_str())
    }

    /// The `to_bytes` method is used to convert a `SignatureType` enum to a `Vec<u8>`.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            SignatureType::Author => "author".to_string().into_bytes(),
            SignatureType::Committer => "committer".to_string().into_bytes(),
            SignatureType::Tagger => "tagger".to_string().into_bytes(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Decode, Encode)]
pub struct Signature {
    pub signature_type: SignatureType,
    pub name: String,
    pub email: String,
    pub timestamp: usize,
    pub timezone: String,
}

impl Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{} <{}>", self.name, self.email).unwrap();
        // format the timestamp to a human-readable date format
        let date =
            chrono::DateTime::<chrono::Utc>::from_timestamp(self.timestamp as i64, 0).unwrap();
        writeln!(f, "Date: {} {}", date, self.timezone)
    }
}

impl Signature {
    pub fn from_data(data: Vec<u8>) -> Result<Signature, GitError> {
        // Make a mutable copy of the input data vector.
        let mut sign = data;

        // Find the index of the first space byte in the data vector.
        let name_start = sign.find_byte(0x20).unwrap();

        // Parse the author name from the bytes up to the first space byte.
        // If the parsing fails, unwrap will panic.
        let signature_type = SignatureType::from_data(sign[..name_start].to_vec())?;

        let (name, email) = {
            let email_start = sign.find_byte(0x3C).unwrap();
            let email_end = sign.find_byte(0x3E).unwrap();

            unsafe {
                (
                    sign[name_start + 1..email_start - 1]
                        .to_str_unchecked()
                        .to_string(),
                    sign[email_start + 1..email_end]
                        .to_str_unchecked()
                        .to_string(),
                )
            }
        };

        // Update the data vector to remove the author and email bytes.
        sign = sign[sign.find_byte(0x3E).unwrap() + 2..].to_vec();

        // Find the index of the second space byte in the updated data vector.
        let timestamp_split = sign.find_byte(0x20).unwrap();

        // Parse the timestamp integer from the bytes up to the second space byte.
        // If the parsing fails, unwrap will panic.
        let timestamp = unsafe {
            sign[0..timestamp_split]
                .to_str_unchecked()
                .parse::<usize>()
                .unwrap()
        };

        // Parse the timezone string from the bytes after the second space byte.
        // If the parsing fails, unwrap will panic.
        let timezone = unsafe { sign[timestamp_split + 1..].to_str_unchecked().to_string() };

        // Return a Result object indicating success
        Ok(Signature {
            signature_type,
            name,
            email,
            timestamp,
            timezone,
        })
    }

    pub fn to_data(&self) -> Result<Vec<u8>, GitError> {
        // Create a new empty vector to store the encoded data.
        let mut sign = Vec::new();

        // Append the author name bytes to the data vector, followed by a space byte.
        sign.extend_from_slice(&self.signature_type.to_bytes());
        sign.extend_from_slice(&[0x20]);

        // Append the name bytes to the data vector, followed by a space byte.
        sign.extend_from_slice(self.name.as_bytes());
        sign.extend_from_slice(&[0x20]);

        // Append the email address bytes to the data vector, enclosed in angle brackets.
        sign.extend_from_slice(format!("<{}>", self.email).as_bytes());
        sign.extend_from_slice(&[0x20]);

        // Append the timestamp integer bytes to the data vector, followed by a space byte.
        sign.extend_from_slice(self.timestamp.to_string().as_bytes());
        sign.extend_from_slice(&[0x20]);

        // Append the timezone string bytes to the data vector.
        sign.extend_from_slice(self.timezone.as_bytes());

        // Return the data vector as a Result object indicating success.
        Ok(sign)
    }

    /// Represents a signature with author, email, timestamp, and timezone information.
    pub fn new(sign_type: SignatureType, author: String, email: String) -> Signature {
        // Get the current local time (with timezone)
        let local_time = chrono::Local::now();

        // Get the offset from UTC in minutes (local time - UTC time)
        let offset = local_time.offset().fix().local_minus_utc();

        // Calculate the hours part of the offset (divide by 3600 to convert from seconds to hours)
        let hours = offset / 60 / 60;

        // Calculate the minutes part of the offset (remaining minutes after dividing by 60)
        let minutes = offset / 60 % 60;

        // Format the offset as a string (e.g., "+0800", "-0300", etc.)
        let offset_str = format!("{hours:+03}{minutes:02}");

        // Return the Signature struct with the provided information
        Signature {
            signature_type: sign_type, // The type of signature (e.g., commit, merge)
            name: author,              // The author's name
            email,                     // The author's email
            timestamp: chrono::Utc::now().timestamp() as usize, // The timestamp of the signature (seconds since Unix epoch)
            timezone: offset_str, // The timezone offset (e.g., "+0800")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::DateTime;

    use crate::internal::object::signature::{Signature, SignatureType};

    #[test]
    fn test_signature_type_from_str() {
        assert_eq!(
            SignatureType::from_str("author").unwrap(),
            SignatureType::Author
        );

        assert_eq!(
            SignatureType::from_str("committer").unwrap(),
            SignatureType::Committer
        );
    }

    #[test]
    fn test_signature_type_from_data() {
        assert_eq!(
            SignatureType::from_data("author".to_string().into_bytes()).unwrap(),
            SignatureType::Author
        );

        assert_eq!(
            SignatureType::from_data("committer".to_string().into_bytes()).unwrap(),
            SignatureType::Committer
        );
    }

    #[test]
    fn test_signature_type_to_bytes() {
        assert_eq!(
            SignatureType::Author.to_bytes(),
            "author".to_string().into_bytes()
        );

        assert_eq!(
            SignatureType::Committer.to_bytes(),
            "committer".to_string().into_bytes()
        );
    }

    #[test]
    fn test_signature_new_from_data() {
        let sign = Signature::from_data(
            "author Quanyi Ma <eli@patch.sh> 1678101573 +0800"
                .to_string()
                .into_bytes(),
        )
        .unwrap();

        assert_eq!(sign.signature_type, SignatureType::Author);
        assert_eq!(sign.name, "Quanyi Ma");
        assert_eq!(sign.email, "eli@patch.sh");
        assert_eq!(sign.timestamp, 1678101573);
        assert_eq!(sign.timezone, "+0800");
    }

    #[test]
    fn test_signature_to_data() {
        let sign = Signature::from_data(
            "committer Quanyi Ma <eli@patch.sh> 1678101573 +0800"
                .to_string()
                .into_bytes(),
        )
        .unwrap();

        let dest = sign.to_data().unwrap();

        assert_eq!(
            dest,
            "committer Quanyi Ma <eli@patch.sh> 1678101573 +0800"
                .to_string()
                .into_bytes()
        );
    }

    /// When the test case run in the GitHub Action, the timezone is +0000, so we ignore it.
    #[test]
    #[ignore]
    fn test_signature_with_time() {
        let sign = Signature::new(
            SignatureType::Author,
            "MEGA".to_owned(),
            "admin@mega.com".to_owned(),
        );
        assert_eq!(sign.signature_type, SignatureType::Author);
        assert_eq!(sign.name, "MEGA");
        assert_eq!(sign.email, "admin@mega.com");
        // assert_eq!(sign.timezone, "+0800");//it depends on the local timezone

        let naive_datetime = DateTime::from_timestamp(sign.timestamp as i64, 0).unwrap();
        println!("Formatted DateTime: {}", naive_datetime.naive_local());
    }
}
