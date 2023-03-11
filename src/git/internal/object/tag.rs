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
//! If one of -a, -s, or -u <key-id> is passed, the command creates a tag object, and requires a tag
//! message. Unless -m <msg> or -F <file> is given, an editor is started for the user to type in the
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
use std::path::PathBuf;

use bstr::ByteSlice;

use crate::git::hash::Hash;
use crate::git::errors::GitError;
use crate::git::internal::ObjectType;
use crate::git::internal::object::meta::Meta;
use crate::git::internal::object::signature::Signature;


/// The tag object is used to Annotated tag
#[allow(unused)]
pub struct Tag {
    pub meta: Meta,
    pub object_hash: Hash,
    pub object_type: ObjectType,
    pub tag_name: String,
    pub tagger: Signature,
    pub message: String,
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

    /// The tag object is used to Annotated tag, it's binary format is:
    ///
    /// ```bash
    /// object <object_hash> 0x0a # The SHA-1 hash of the object that the annotated tag is attached to (usually a commit)
    /// type <object_type> 0x0a #The type of Git object that the annotated tag is attached to (usually 'commit')
    /// tag <tag_name> 0x0a # The name of the annotated tag(in UTF-8 encoding)
    /// tagger <tagger> 0x0a # The name, email address, and date of the person who created the annotated tag
    /// <message>
    /// ```
    #[allow(unused)]
    pub fn new_from_data(data: Vec<u8>) -> Result<Tag, GitError> {
        let meta = Meta::new_from_data_with_object_type(ObjectType::Tag, data.clone());
        let mut data = data;

        let hash_begin = data.find_byte(0x20).unwrap();
        let hash_end = data.find_byte(0x0a).unwrap();
        let object_hash = Hash::new_from_str(data[hash_begin + 1..hash_end].to_str().unwrap());
        data = data[hash_end + 1..].to_vec();

        let type_begin = data.find_byte(0x20).unwrap();
        let type_end = data.find_byte(0x0a).unwrap();
        let object_type = ObjectType::from_string(data[type_begin + 1..type_end].to_str().unwrap())?;
        data = data[type_end + 1..].to_vec();

        let tag_begin = data.find_byte(0x20).unwrap();
        let tag_end = data.find_byte(0x0a).unwrap();
        let tag_name = String::from_utf8( data[tag_begin + 1..tag_end].to_vec()).unwrap();
        data = data[tag_end + 1..].to_vec();

        let tagger_begin = data.find("tagger").unwrap();
        let tagger_end = data.find_byte(0x0a).unwrap();
        let tagger_data = data[tagger_begin..tagger_end].to_vec();
        let tagger = Signature::new_from_data(tagger_data)?;
        data = data[data.find_byte(0x0a).unwrap() + 1..].to_vec();

        let message = data[data.find_byte(0x0a).unwrap()..].to_vec().to_str().unwrap().to_string();

        Ok(Tag { meta, object_hash, object_type, tag_name, tagger, message })
    }

    #[allow(unused)]
    pub fn new_from_meta(meta: Meta) -> Result<Tag, GitError> {
        Tag::new_from_data(meta.data)
    }

    #[allow(unused)]
    pub fn new_from_file(path: &str) -> Result<Tag, GitError> {
        let meta = Meta::new_from_file(path)?;

        Tag::new_from_meta(meta)
    }

    ///
    /// ```bash
    /// object <object_hash> 0x0a # The SHA-1 hash of the object that the annotated tag is attached to (usually a commit)
    /// type <object_type> 0x0a #The type of Git object that the annotated tag is attached to (usually 'commit')
    /// tag <tag_name> 0x0a # The name of the annotated tag(in UTF-8 encoding)
    /// tagger <tagger> 0x0a # The name, email address, and date of the person who created the annotated tag
    /// <message>
    /// ```
    #[allow(unused)]
    pub fn to_data(&self) -> Result<Vec<u8>, GitError> {
        let mut data = Vec::new();

        data.extend_from_slice("object".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.object_hash.to_plain_str().as_bytes());
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

    #[allow(unused)]
    pub fn to_file(&self, path: &str) -> Result<PathBuf, GitError> {
        self.meta.to_file(path)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_new_from_file(){
        use std::env;
        use std::path::PathBuf;

        let mut source = PathBuf::from(env::current_dir().unwrap());
        source.push("tests/data/objects/85/4aac1e94777f3ffc8722b69f087d1244587ab7");

        let tag = super::Tag::new_from_file(source.to_str().unwrap()).unwrap();

        assert_eq!(tag.meta.id.to_plain_str(), "854aac1e94777f3ffc8722b69f087d1244587ab7");
        assert_eq!(tag.object_hash.to_plain_str(), "4b00093bee9b3ef5afc5f8e3645dc39cfa2f49aa");
        assert_eq!(tag.tag_name, "v.0.1.0");
        assert_eq!(tag.tagger.name, "Quanyi Ma");
    }

    #[test]
    fn test_to_data() {
        use std::env;
        use std::path::PathBuf;

        let mut source = PathBuf::from(env::current_dir().unwrap());
        source.push("tests/data/objects/85/4aac1e94777f3ffc8722b69f087d1244587ab7");

        let tag = super::Tag::new_from_file(source.to_str().unwrap()).unwrap();
        let data = tag.to_data().unwrap();

        assert_eq!(tag.meta.data, data);
    }

    #[test]
    fn test_to_file() {
        use std::env;
        use std::path::PathBuf;
        use std::fs::remove_file;

        let mut source = PathBuf::from(env::current_dir().unwrap());
        source.push("tests/data/objects/85/4aac1e94777f3ffc8722b69f087d1244587ab7");
        let tag = super::Tag::new_from_file(source.to_str().unwrap()).unwrap();

        let mut dest_file = PathBuf::from(env::current_dir().unwrap());
        dest_file.push("tests/objects/85/4aac1e94777f3ffc8722b69f087d1244587ab7");
        if dest_file.exists() {
            remove_file(dest_file.as_path().to_str().unwrap()).unwrap();
        }

        let mut dest = PathBuf::from(env::current_dir().unwrap());
        dest.push("tests");
        dest.push("objects");

        let file = tag.to_file(dest.to_str().unwrap()).unwrap();

        assert_eq!(true, file.exists());
    }
}
