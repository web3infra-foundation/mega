//! In Git objects there are two types of tags: Lightweight tags and annotated tags.
//!
//! A lightweight tag is simply a pointer to a specific commit in Git's version history,
//! without any additional metadata or information associated with it. It is created by
//! running the `git tag` command with a name for the tag and the commit hash that it points to.
//!
//! An annotated tag, on the other hand, is a Git object in its own right, and includes
//! metadata such as the tagger's name and email address, the date and time the tag was created,
//! and a message describing the tag. It is created by running the `git tag -a` command with
//! a name for the tag, the commit hash that it points to, and the additional metadata that
//! should be associated with the tag.
//!
//! When you create a tag in Git, whether it's a lightweight or annotated tag, Git creates a
//! new object in its object database to represent the tag. This object includes the name of the
//! tag, the hash of the commit it points to, and any additional metadata associated with the
//! tag (in the case of an annotated tag).
//!
//! There is no difference in binary format between lightweight tags and annotated tags in Git,
//! as both are represented using the same lightweight object format in Git's object database.
//!
//! The lightweight tag is a reference to a specific commit in Git's version history, not be stored
//! as a separate object in Git's object database. This means that if you create a lightweight tag
//! and then move the tag to a different commit, the tag will still point to the original commit.
//!
//! The lightweight just a text file with the commit hash in it, and the file name is the tag name.
//! If one of -a, -s, or -u \<key-id\> is passed, the command creates a tag object, and requires a tag
//! message. Unless -m \<msg\> or -F \<file\> is given, an editor is started for the user to type in the
//! tag message.
//!
//! ```bash
//! 4b00093bee9b3ef5afc5f8e3645dc39cfa2f49aa
//! ```
//!
//! The annotated tag is a Git object in its own right, and includes metadata such as the tagger's
//! name and email address, the date and time the tag was created, and a message describing the tag.
//!
//! So, we can use the `git cat-file -p <tag>` command to get the tag object, and the command not
//! for the lightweight tag.
use std::fmt::Display;
use std::str::FromStr;

use bstr::ByteSlice;

use crate::errors::GitError;
use crate::hash::SHA1;
use crate::internal::object::ObjectTrait;
use crate::internal::object::ObjectType;
use crate::internal::object::signature::Signature;

/// The tag object is used to Annotated tag
#[derive(Eq, Debug, Clone)]
#[non_exhaustive]
pub struct Tag {
    pub id: SHA1,
    pub object_hash: SHA1,
    pub object_type: ObjectType,
    pub tag_name: String,
    pub tagger: Signature,
    pub message: String,
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "object {}\ntype {}\ntag {}\ntagger {}\n\n{}",
            self.object_hash, self.object_type, self.tag_name, self.tagger, self.message
        )
    }
}

impl Tag {
    // pub fn new_from_meta(meta: Meta) -> Result<Tag, GitError> {
    //     Ok(Tag::new_from_data(meta.data))
    // }

    // pub fn new_from_file(path: &str) -> Result<Tag, GitError> {
    //     let meta = Meta::new_from_file(path)?;
    //     Tag::new_from_meta(meta)
    // }

    pub fn new(
        object_hash: SHA1,
        object_type: ObjectType,
        tag_name: String,
        tagger: Signature,
        message: String,
    ) -> Self {
        // Serialize the tag data to calculate its hash
        let data = format!(
            "object {}\ntype {}\ntag {}\ntagger {}\n\n{}",
            object_hash, object_type, tag_name, tagger, message
        );
        let id = SHA1::from_type_and_data(ObjectType::Tag, data.as_bytes());

        Self {
            id,
            object_hash,
            object_type,
            tag_name,
            tagger,
            message,
        }
    }
}

impl ObjectTrait for Tag {
    /// The tag object is used to Annotated tag, it's binary format is:
    ///
    /// ```bash
    /// object <object_hash> 0x0a # The SHA-1 hash of the object that the annotated tag is attached to (usually a commit)
    /// type <object_type> 0x0a #The type of Git object that the annotated tag is attached to (usually 'commit')
    /// tag <tag_name> 0x0a # The name of the annotated tag(in UTF-8 encoding)
    /// tagger <tagger> 0x0a # The name, email address, and date of the person who created the annotated tag
    /// <message>
    /// ```
    fn from_bytes(row_data: &[u8], hash: SHA1) -> Result<Self, GitError>
    where
        Self: Sized,
    {
        let mut headers = row_data;
        let mut message_start = 0;

        if let Some(pos) = headers.find(b"\n\n") {
            message_start = pos + 2;
            headers = &headers[..pos];
        }

        let mut object_hash: Option<SHA1> = None;
        let mut object_type: Option<ObjectType> = None;
        let mut tag_name: Option<String> = None;
        let mut tagger: Option<Signature> = None;

        for line in headers.lines() {
            if let Some(s) = line.strip_prefix(b"object ") {
                let hash_str = s.to_str().map_err(|_| {
                    GitError::InvalidTagObject("Invalid UTF-8 in object hash".to_string())
                })?;
                object_hash = Some(SHA1::from_str(hash_str).map_err(|_| {
                    GitError::InvalidTagObject("Invalid object hash format".to_string())
                })?);
            } else if let Some(s) = line.strip_prefix(b"type ") {
                let type_str = s.to_str().map_err(|_| {
                    GitError::InvalidTagObject("Invalid UTF-8 in object type".to_string())
                })?;
                object_type = Some(ObjectType::from_string(type_str)?);
            } else if let Some(s) = line.strip_prefix(b"tag ") {
                let tag_str = s.to_str().map_err(|_| {
                    GitError::InvalidTagObject("Invalid UTF-8 in tag name".to_string())
                })?;
                tag_name = Some(tag_str.to_string());
            } else if line.starts_with(b"tagger ") {
                tagger = Some(Signature::from_data(line.to_vec())?);
            }
        }

        let message = if message_start > 0 {
            String::from_utf8_lossy(&row_data[message_start..]).to_string()
        } else {
            String::new()
        };

        Ok(Tag {
            id: hash,
            object_hash: object_hash
                .ok_or_else(|| GitError::InvalidTagObject("Missing object hash".to_string()))?,
            object_type: object_type
                .ok_or_else(|| GitError::InvalidTagObject("Missing object type".to_string()))?,
            tag_name: tag_name
                .ok_or_else(|| GitError::InvalidTagObject("Missing tag name".to_string()))?,
            tagger: tagger
                .ok_or_else(|| GitError::InvalidTagObject("Missing tagger".to_string()))?,
            message,
        })
    }

    fn get_type(&self) -> ObjectType {
        ObjectType::Tag
    }

    fn get_size(&self) -> usize {
        self.to_data().map(|data| data.len()).unwrap_or(0)
    }

    ///
    /// ```bash
    /// object <object_hash> 0x0a # The SHA-1 hash of the object that the annotated tag is attached to (usually a commit)
    /// type <object_type> 0x0a #The type of Git object that the annotated tag is attached to (usually 'commit')
    /// tag <tag_name> 0x0a # The name of the annotated tag(in UTF-8 encoding)
    /// tagger <tagger> 0x0a # The name, email address, and date of the person who created the annotated tag
    /// <message>
    /// ```
    fn to_data(&self) -> Result<Vec<u8>, GitError> {
        let mut data = Vec::new();

        data.extend_from_slice(b"object ");
        data.extend_from_slice(self.object_hash.to_string().as_bytes());
        data.extend_from_slice(b"\n");

        data.extend_from_slice(b"type ");
        data.extend_from_slice(self.object_type.to_string().as_bytes());
        data.extend_from_slice(b"\n");

        data.extend_from_slice(b"tag ");
        data.extend_from_slice(self.tag_name.as_bytes());
        data.extend_from_slice(b"\n");

        data.extend_from_slice(&self.tagger.to_data()?);
        data.extend_from_slice(b"\n\n");

        data.extend_from_slice(self.message.as_bytes());

        Ok(data)
    }
}
