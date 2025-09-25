use std::str::FromStr;

use common::utils::generate_id;

// Callisto models
use callisto::{
    git_blob, git_commit, git_tag, git_tree, mega_blob, mega_commit, mega_tag, mega_tree, raw_blob,
    sea_orm_active_enums::StorageTypeEnum,
};

// Mercury types
use mercury::{
    hash::SHA1,
    internal::{
        object::{
            ObjectTrait, blob::Blob, commit::Commit, signature::Signature, tag::Tag, tree::Tree,
            types::ObjectType
        },
        pack::entry::Entry,
    },
};

// Import conversion traits
use crate::utils::converter::{IntoMegaModel, IntoGitModel, ToRawBlob};

#[derive(PartialEq, Debug, Clone)]
pub enum GitObject {
    Commit(Commit),
    Tree(Tree),
    Blob(Blob),
    Tag(Tag),
}

#[derive(PartialEq, Debug, Clone)]
pub enum GitObjectModel {
    Commit(git_commit::Model),
    Tree(git_tree::Model),
    Blob(git_blob::Model, raw_blob::Model),
    Tag(git_tag::Model),
}

pub enum MegaObjectModel {
    Commit(mega_commit::Model),
    Tree(mega_tree::Model),
    Blob(mega_blob::Model, raw_blob::Model),
    Tag(mega_tag::Model),
}

impl GitObject {
    pub fn convert_to_mega_model(self) -> MegaObjectModel {
        match self {
            GitObject::Commit(commit) => MegaObjectModel::Commit(commit.into_mega_model()),
            GitObject::Tree(tree) => MegaObjectModel::Tree(tree.into_mega_model()),
            GitObject::Blob(blob) => MegaObjectModel::Blob(blob.clone().into_mega_model(), blob.to_raw_blob()),
            GitObject::Tag(tag) => MegaObjectModel::Tag(tag.into_mega_model()),
        }
    }

    pub fn convert_to_git_model(self) -> GitObjectModel {
        match self {
            GitObject::Commit(commit) => GitObjectModel::Commit(commit.into_git_model()),
            GitObject::Tree(tree) => GitObjectModel::Tree(tree.into_git_model()),
            GitObject::Blob(blob) => GitObjectModel::Blob(blob.clone().into_git_model(), blob.to_raw_blob()),
            GitObject::Tag(tag) => GitObjectModel::Tag(tag.into_git_model()),
        }
    }
}

pub fn process_entry(entry: Entry) -> GitObject {
    match entry.obj_type {
        ObjectType::Commit => {
            GitObject::Commit(Commit::from_bytes(&entry.data, entry.hash).unwrap())
        }
        ObjectType::Tree => GitObject::Tree(Tree::from_bytes(&entry.data, entry.hash).unwrap()),
        ObjectType::Blob => GitObject::Blob(Blob::from_bytes(&entry.data, entry.hash).unwrap()),
        ObjectType::Tag => GitObject::Tag(Tag::from_bytes(&entry.data, entry.hash).unwrap()),
        _ => unreachable!("can not parse delta!"),
    }
}

/// Converts a Blob object to a mega_blob::Model
///
/// This function creates a new mega_blob::Model from a Blob object.
/// The resulting model will have a newly generated ID, the blob's ID as string,
/// and default values for size, commit_id, and name.
///
/// # Arguments
///
/// * `blob` - A reference to the Blob object to convert
///
/// # Returns
///
/// A new mega_blob::Model instance populated with data from the blob
pub fn blob_to_mega_blob(blob: &Blob) -> mega_blob::Model {
    mega_blob::Model {
        id: generate_id(),
        blob_id: blob.id.to_string(),
        size: 0,
        commit_id: String::new(),
        name: String::new(),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a Blob object to a git_blob::Model
///
/// This function creates a new git_blob::Model from a Blob object.
/// The resulting model will have a newly generated ID, the blob's ID as string,
/// repository ID set to 0, and default values for size and name.
///
/// # Arguments
///
/// * `blob` - A reference to the Blob object to convert
///
/// # Returns
///
/// A new git_blob::Model instance populated with data from the blob
pub fn blob_to_git_blob(blob: &Blob) -> git_blob::Model {
    git_blob::Model {
        id: generate_id(),
        repo_id: 0,
        blob_id: blob.id.to_string(),
        size: 0,
        name: None,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a Blob object to a raw_blob::Model
///
/// This function creates a new raw_blob::Model from a Blob object.
/// The resulting model will include the blob's data and ID.
/// Storage type is set to Database by default.
///
/// # Arguments
///
/// * `blob` - A reference to the Blob object to convert
///
/// # Returns
///
/// A new raw_blob::Model instance populated with data from the blob
pub fn blob_to_raw_blob(blob: &Blob) -> raw_blob::Model {
    raw_blob::Model {
        id: generate_id(),
        sha1: blob.id.to_string(),
        storage_type: StorageTypeEnum::Database,
        data: Some(blob.data.clone()),
        content: None,
        file_type: None,
        local_path: None,
        remote_url: None,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a raw_blob::Model to a Blob object
///
/// This function extracts the necessary data from a raw_blob::Model
/// to create a new Blob object. It parses the SHA1 hash from the string
/// representation and unwraps the binary data.
///
/// # Arguments
///
/// * `raw_blob` - The raw_blob::Model to convert
///
/// # Returns
///
/// A new Blob instance containing the ID and data from the raw_blob model
///
/// # Panics
///
/// This function will panic if the SHA1 string cannot be parsed or if
/// the data field is None
pub fn raw_blob_to_blob(raw_blob: raw_blob::Model) -> Blob {
    Blob {
        id: SHA1::from_str(&raw_blob.sha1).unwrap(),
        data: raw_blob.data.unwrap(),
    }
}

/// Converts a Commit object to a mega_commit::Model
///
/// This function transforms a Commit object into a mega_commit::Model for database storage.
/// It preserves all relevant commit metadata including tree reference, parent commit IDs,
/// author information, committer information, and commit message.
///
/// # Arguments
///
/// * `commit` - The Commit object to convert
///
/// # Returns
///
/// A new mega_commit::Model instance populated with data from the commit
///
/// # Panics
///
/// This function will panic if author or committer signature data cannot be converted to bytes
pub fn commit_to_mega_commit(commit: Commit) -> mega_commit::Model {
    mega_commit::Model {
        id: generate_id(),
        commit_id: commit.id.to_string(),
        tree: commit.tree_id.to_string(),
        parents_id: commit
            .parent_commit_ids
            .iter()
            .map(|x| x.to_string())
            .collect(),
        author: Some(String::from_utf8_lossy(&commit.author.to_data().unwrap()).to_string()),
        committer: Some(String::from_utf8_lossy(&commit.committer.to_data().unwrap()).to_string()),
        content: Some(commit.message.clone()),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a Commit object to a git_commit::Model
///
/// This function transforms a Commit object into a git_commit::Model for Git repository
/// database storage. It preserves all relevant commit metadata including tree reference,
/// parent commit IDs, author information, committer information, and commit message.
/// The repository ID is set to 0 by default.
///
/// # Arguments
///
/// * `commit` - The Commit object to convert
///
/// # Returns
///
/// A new git_commit::Model instance populated with data from the commit
///
/// # Panics
///
/// This function will panic if author or committer signature data cannot be converted to bytes
pub fn commit_to_git_commit(commit: Commit) -> git_commit::Model {
    git_commit::Model {
        id: generate_id(),
        repo_id: 0,
        commit_id: commit.id.to_string(),
        tree: commit.tree_id.to_string(),
        parents_id: commit
            .parent_commit_ids
            .iter()
            .map(|x| x.to_string())
            .collect(),
        author: Some(String::from_utf8_lossy(&commit.author.to_data().unwrap()).to_string()),
        committer: Some(String::from_utf8_lossy(&commit.committer.to_data().unwrap()).to_string()),
        content: Some(commit.message.clone()),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a pack Entry to a Commit object
///
/// This function creates a Commit object from a pack Entry by parsing
/// the binary data stored in the entry. The entry's hash is used as the
/// commit's hash identifier.
///
/// # Arguments
///
/// * `entry` - The Entry containing the commit data
///
/// # Returns
///
/// A new Commit instance created from the entry data
///
/// # Panics
///
/// This function will panic if the entry data cannot be parsed into a valid Commit
pub fn entry_to_commit(entry: Entry) -> Commit {
    Commit::from_bytes(&entry.data, entry.hash).unwrap()
}

/// Converts a Tag object to a mega_tag::Model
///
/// This function transforms a Tag object into a mega_tag::Model for database storage.
/// It preserves all tag metadata including the referenced object hash, object type,
/// tag name, tagger information, and tag message.
///
/// # Arguments
///
/// * `tag` - The Tag object to convert
///
/// # Returns
///
/// A new mega_tag::Model instance populated with data from the tag
///
/// # Panics
///
/// This function will panic if tagger signature data cannot be converted to bytes
pub fn tag_to_mega_tag(tag: Tag) -> mega_tag::Model {
    mega_tag::Model {
        id: generate_id(),
        tag_id: tag.id.to_string(),
        object_id: tag.object_hash.to_string(),
        object_type: tag.object_type.to_string(),
        tag_name: tag.tag_name,
        tagger: String::from_utf8_lossy(&tag.tagger.to_data().unwrap()).to_string(),
        message: tag.message,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a Tag object to a git_tag::Model
///
/// This function transforms a Tag object into a git_tag::Model for Git repository
/// database storage. It preserves all tag metadata including the referenced object hash,
/// object type, tag name, tagger information, and tag message.
/// The repository ID is set to 0 by default.
///
/// # Arguments
///
/// * `tag` - The Tag object to convert
///
/// # Returns
///
/// A new git_tag::Model instance populated with data from the tag
///
/// # Panics
///
/// This function will panic if tagger signature data cannot be converted to bytes
pub fn tag_to_git_tag(tag: Tag) -> git_tag::Model {
    git_tag::Model {
        id: generate_id(),
        repo_id: 0,
        tag_id: tag.id.to_string(),
        object_id: tag.object_hash.to_string(),
        object_type: tag.object_type.to_string(),
        tag_name: tag.tag_name,
        tagger: String::from_utf8_lossy(&tag.tagger.to_data().unwrap()).to_string(),
        message: tag.message,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a mega_tag::Model to a Tag object
///
/// This function reconstructs a Tag object from a mega_tag::Model retrieved from the database.
/// It parses the stored strings back into their original types, such as converting
/// string IDs back to SHA1 hashes and string data back to a Signature.
///
/// # Arguments
///
/// * `mega_tag` - The mega_tag::Model to convert
///
/// # Returns
///
/// A new Tag instance populated with data from the model
///
/// # Panics
///
/// This function will panic if:
/// - The tag_id string cannot be parsed into a valid SHA1 hash
/// - The object_id string cannot be parsed into a valid SHA1 hash
/// - The object_type string is not a recognized ObjectType
/// - The tagger string cannot be converted into a valid Signature
pub fn mega_tag_to_tag(mega_tag: mega_tag::Model) -> Tag {
    Tag {
        id: SHA1::from_str(&mega_tag.tag_id).expect("Invalid tag_id in database"),
        object_hash: SHA1::from_str(&mega_tag.object_id).unwrap(),
        object_type: ObjectType::from_string(&mega_tag.object_type).unwrap(),
        tag_name: mega_tag.tag_name,
        tagger: Signature::from_data(mega_tag.tagger.into_bytes()).unwrap(),
        message: mega_tag.message,
    }
}

/// Converts a git_tag::Model to a Tag object
///
/// This function reconstructs a Tag object from a git_tag::Model retrieved from the database.
/// It parses the stored strings back into their original types, such as converting
/// string IDs back to SHA1 hashes and string data back to a Signature.
///
/// # Arguments
///
/// * `git_tag` - The git_tag::Model to convert
///
/// # Returns
///
/// A new Tag instance populated with data from the model
///
/// # Panics
///
/// This function will panic if:
/// - The tag_id string cannot be parsed into a valid SHA1 hash
/// - The object_id string cannot be parsed into a valid SHA1 hash
/// - The object_type string is not a recognized ObjectType
/// - The tagger string cannot be converted into a valid Signature
pub fn git_tag_to_tag(git_tag: git_tag::Model) -> Tag {
    Tag {
        id: SHA1::from_str(&git_tag.tag_id).unwrap(),
        object_hash: SHA1::from_str(&git_tag.object_id).unwrap(),
        object_type: ObjectType::from_string(&git_tag.object_type).unwrap(),
        tag_name: git_tag.tag_name,
        tagger: Signature::from_data(git_tag.tagger.into_bytes()).unwrap(),
        message: git_tag.message,
    }
}

/// Converts a Tree object to a mega_tree::Model
///
/// This function transforms a Tree object into a mega_tree::Model for database storage.
/// It serializes the tree structure into binary data and stores essential metadata
/// like the tree ID. Size is set to 0 and commit_id to an empty string by default.
///
/// # Arguments
///
/// * `tree` - The Tree object to convert
///
/// # Returns
///
/// A new mega_tree::Model instance populated with data from the tree
///
/// # Panics
///
/// This function will panic if the tree's data cannot be serialized
pub fn tree_to_mega_tree(tree: Tree) -> mega_tree::Model {
    mega_tree::Model {
        id: generate_id(),
        tree_id: tree.id.to_string(),
        sub_trees: tree.to_data().unwrap(),
        size: 0,
        commit_id: String::new(),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a Tree object to a git_tree::Model
///
/// This function transforms a Tree object into a git_tree::Model for Git repository
/// database storage. It serializes the tree structure into binary data and stores
/// essential metadata like the tree ID. The repository ID is set to 0 and size
/// is set to 0 by default.
///
/// # Arguments
///
/// * `tree` - The Tree object to convert
///
/// # Returns
///
/// A new git_tree::Model instance populated with data from the tree
///
/// # Panics
///
/// This function will panic if the tree's data cannot be serialized
pub fn tree_to_git_tree(tree: Tree) -> git_tree::Model {
    git_tree::Model {
        id: generate_id(),
        repo_id: 0,
        tree_id: tree.id.to_string(),
        sub_trees: tree.to_data().unwrap(),
        size: 0,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// Converts a mega_tree::Model to a Tree object
///
/// This function reconstructs a Tree object from a mega_tree::Model retrieved from the database.
/// It parses the binary sub_trees data back into a structured Tree object and
/// uses the tree_id string to recreate the SHA1 hash identifier.
///
/// # Arguments
///
/// * `mega_tree` - The mega_tree::Model to convert
///
/// # Returns
///
/// A new Tree instance reconstructed from the model data
///
/// # Panics
///
/// This function will panic if:
/// - The tree_id string cannot be parsed into a valid SHA1 hash
/// - The binary sub_trees data cannot be parsed into a valid Tree structure
pub fn mega_tree_to_tree(mega_tree: mega_tree::Model) -> Tree {
    Tree::from_bytes(
        &mega_tree.sub_trees,
        SHA1::from_str(&mega_tree.tree_id).unwrap(),
    )
    .unwrap()
}

/// Converts a git_tree::Model to a Tree object
///
/// This function reconstructs a Tree object from a git_tree::Model retrieved from the database.
/// It parses the binary sub_trees data back into a structured Tree object and
/// uses the tree_id string to recreate the SHA1 hash identifier.
///
/// # Arguments
///
/// * `git_tree` - The git_tree::Model to convert
///
/// # Returns
///
/// A new Tree instance reconstructed from the model data
///
/// # Panics
///
/// This function will panic if:
/// - The tree_id string cannot be parsed into a valid SHA1 hash
/// - The binary sub_trees data cannot be parsed into a valid Tree structure
pub fn git_tree_to_tree(git_tree: git_tree::Model) -> Tree {
    Tree::from_bytes(
        &git_tree.sub_trees,
        SHA1::from_str(&git_tree.tree_id).unwrap(),
    )
    .unwrap()
}
