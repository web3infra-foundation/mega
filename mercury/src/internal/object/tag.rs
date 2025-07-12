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
use crate::internal::object::signature::Signature;
use crate::internal::object::ObjectTrait;
use crate::internal::object::ObjectType;

/// The tag object is used to Annotated tag
#[derive(Eq, Debug, Clone)]
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
    
    /// Create a new Tag object
    pub fn new(tag_name: &str, object_hash: &SHA1, message: &str) -> Self {
        // Create tagger signature
        let tagger = Signature::new_now("libra", "libra@example.com");
        
        // Default to marking commit objects
        let object_type = ObjectType::Commit;
        
        // Build the tag data
        let mut tag = Tag {
            id: SHA1::default(),
            object_hash: *object_hash,
            object_type,
            tag_name: tag_name.to_string(),
            tagger,
            message: message.to_string(),
        };
        
        // Calculate tag ID
        let data = tag.to_data().unwrap();
        let header = format!("tag {}", data.len());
        let mut content = Vec::new();
        content.extend_from_slice(header.as_bytes());
        content.push(0);
        content.extend_from_slice(&data);
        
        tag.id = SHA1::new(&content);
        
        tag
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
        let mut data = row_data;

        let hash_begin = data.find_byte(0x20).unwrap();
        let hash_end = data.find_byte(0x0a).unwrap();
        let object_hash = SHA1::from_str(data[hash_begin + 1..hash_end].to_str().unwrap()).unwrap();
        data = &data[hash_end + 1..];

        let type_begin = data.find_byte(0x20).unwrap();
        let type_end = data.find_byte(0x0a).unwrap();
        let object_type =
            ObjectType::from_string(data[type_begin + 1..type_end].to_str().unwrap()).unwrap();
        data = &data[type_end + 1..];

        let tag_begin = data.find_byte(0x20).unwrap();
        let tag_end = data.find_byte(0x0a).unwrap();
        let tag_name = String::from_utf8(data[tag_begin + 1..tag_end].to_vec()).unwrap();
        data = &data[tag_end + 1..];

        let tagger_begin = data.find("tagger").unwrap();
        let tagger_end = data.find_byte(0x0a).unwrap();
        let tagger_data = data[tagger_begin..tagger_end].to_vec();
        let tagger = Signature::from_data(tagger_data).unwrap();
        data = &data[data.find_byte(0x0a).unwrap() + 1..];

        let message = unsafe {
            data[data.find_byte(0x0a).unwrap()..]
                .to_vec()
                .to_str_unchecked()
                .to_string()
        };

        Ok(Tag {
            id: hash,
            object_hash,
            object_type,
            tag_name,
            tagger,
            message,
        })
    }

    fn get_type(&self) -> ObjectType {
        ObjectType::Tag
    }

    fn get_size(&self) -> usize {
        todo!()
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

        data.extend_from_slice("object".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.object_hash.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("type".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.object_type.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("tag".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.tag_name.as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice(self.tagger.to_data()?.as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.message.as_bytes());

        Ok(data)
    }
}
