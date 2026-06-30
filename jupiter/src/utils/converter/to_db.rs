use callisto::{
    git_blob, git_commit, git_tag, git_tree, mega_blob, mega_commit, mega_tag, mega_tree,
};
use common::utils::generate_id;
use git_internal::internal::{
    metadata::EntryMeta,
    object::{ObjectTrait, blob::Blob, commit::Commit, tag::Tag, tree::Tree},
};

use super::traits::{IntoGitModel, IntoMegaModel};

impl IntoMegaModel for Blob {
    type MegaTarget = mega_blob::Model;

    /// Converts a Blob object to a mega_blob::Model
    ///
    /// This function creates a new mega_blob::Model from a Blob object.
    /// The resulting model will have a newly generated ID, the blob's ID as string,
    /// and default values for size, commit_id, and name.
    ///
    /// # Returns
    ///
    /// A new mega_blob::Model instance populated with data from the blob
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_blob::Model {
            id: generate_id(),
            blob_id: self.id.to_string(),
            size: 0,
            commit_id: String::new(),
            name: String::new(),
            pack_id: meta.pack_id.unwrap_or_default(),
            file_path: meta.file_path.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            is_delta_in_pack: meta.is_delta.unwrap_or(false),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoMegaModel for Commit {
    type MegaTarget = mega_commit::Model;

    /// Converts a Commit object to a mega_commit::Model
    ///
    /// This function transforms a Commit object into a mega_commit::Model for database storage.
    /// It preserves all relevant commit metadata including tree reference, parent commit IDs,
    /// author information, committer information, and commit message.
    ///
    /// # Returns
    ///
    /// A new mega_commit::Model instance populated with data from the commit
    ///
    /// # Panics
    ///
    /// This function will panic if author or committer signature data cannot be converted to bytes
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_commit::Model {
            id: generate_id(),
            commit_id: self.id.to_string(),
            tree: self.tree_id.to_string(),
            parents_id: self
                .parent_commit_ids
                .iter()
                .map(|x| x.to_string())
                .collect(),
            author: Some(String::from_utf8_lossy(&self.author.to_data().unwrap()).to_string()),
            committer: Some(
                String::from_utf8_lossy(&self.committer.to_data().unwrap()).to_string(),
            ),
            content: Some(self.message.clone()),
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoMegaModel for Tag {
    type MegaTarget = mega_tag::Model;

    /// Converts a Tag object to a mega_tag::Model
    ///
    /// This function transforms a Tag object into a mega_tag::Model for database storage.
    /// It preserves all tag metadata including the referenced object hash, object type,
    /// tag name, tagger information, and tag message.
    ///
    /// # Returns
    ///
    /// A new mega_tag::Model instance populated with data from the tag
    ///
    /// # Panics
    ///
    /// This function will panic if tagger signature data cannot be converted to bytes
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_tag::Model {
            id: generate_id(),
            tag_id: self.id.to_string(),
            object_id: self.object_hash.to_string(),
            object_type: self.object_type.to_string(),
            tag_name: self.tag_name,
            tagger: String::from_utf8_lossy(&self.tagger.to_data().unwrap()).to_string(),
            message: self.message,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoMegaModel for Tree {
    type MegaTarget = mega_tree::Model;

    /// Converts a Tree object to a mega_tree::Model
    ///
    /// This function transforms a Tree object into a mega_tree::Model for database storage.
    /// It serializes the tree structure into binary data and stores essential metadata
    /// like the tree ID. Size is set to 0 and commit_id to an empty string by default.
    ///
    /// # Returns
    ///
    /// A new mega_tree::Model instance populated with data from the tree
    ///
    /// # Panics
    ///
    /// This function will panic if the tree's data cannot be serialized
    fn into_mega_model(self, meta: EntryMeta) -> Self::MegaTarget {
        mega_tree::Model {
            id: generate_id(),
            tree_id: self.id.to_string(),
            sub_trees: self.to_data().unwrap(),
            size: 0,
            commit_id: String::new(),
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Blob {
    type GitTarget = git_blob::Model;

    /// Converts a Blob object to a git_blob::Model
    ///
    /// This function creates a new git_blob::Model from a Blob object.
    /// The resulting model will have a newly generated ID, the blob's ID as string,
    /// repository ID set to 0, and default values for size and name.
    ///
    /// # Returns
    ///
    /// A new git_blob::Model instance populated with data from the blob
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_blob::Model {
            id: generate_id(),
            repo_id: 0,
            blob_id: self.id.to_string(),
            size: 0,
            name: None,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            file_path: meta.file_path.unwrap_or_default(),
            is_delta_in_pack: meta.is_delta.unwrap_or(false),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Commit {
    type GitTarget = git_commit::Model;

    /// Converts a Commit object to a git_commit::Model
    ///
    /// This function transforms a Commit object into a git_commit::Model for Git repository
    /// database storage. It preserves all relevant commit metadata including tree reference,
    /// parent commit IDs, author information, committer information, and commit message.
    /// The repository ID is set to 0 by default.
    ///
    /// # Returns
    ///
    /// A new git_commit::Model instance populated with data from the commit
    ///
    /// # Panics
    ///
    /// This function will panic if author or committer signature data cannot be converted to bytes
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_commit::Model {
            id: generate_id(),
            repo_id: 0,
            commit_id: self.id.to_string(),
            tree: self.tree_id.to_string(),
            parents_id: self
                .parent_commit_ids
                .iter()
                .map(|x| x.to_string())
                .collect(),
            author: Some(String::from_utf8_lossy(&self.author.to_data().unwrap()).to_string()),
            committer: Some(
                String::from_utf8_lossy(&self.committer.to_data().unwrap()).to_string(),
            ),
            content: Some(self.message.clone()),
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Tag {
    type GitTarget = git_tag::Model;

    /// Converts a Tag object to a git_tag::Model
    ///
    /// This function transforms a Tag object into a git_tag::Model for Git repository
    /// database storage. It preserves all tag metadata including the referenced object hash,
    /// object type, tag name, tagger information, and tag message.
    /// The repository ID is set to 0 by default.
    ///
    /// # Returns
    ///
    /// A new git_tag::Model instance populated with data from the tag
    ///
    /// # Panics
    ///
    /// This function will panic if tagger signature data cannot be converted to bytes
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_tag::Model {
            id: generate_id(),
            repo_id: 0,
            tag_id: self.id.to_string(),
            object_id: self.object_hash.to_string(),
            object_type: self.object_type.to_string(),
            tag_name: self.tag_name,
            tagger: String::from_utf8_lossy(&self.tagger.to_data().unwrap()).to_string(),
            message: self.message,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl IntoGitModel for Tree {
    type GitTarget = git_tree::Model;

    /// Converts a Tree object to a git_tree::Model
    ///
    /// This function transforms a Tree object into a git_tree::Model for Git repository
    /// database storage. It serializes the tree structure into binary data and stores
    /// essential metadata like the tree ID. The repository ID is set to 0 and size
    /// is set to 0 by default.
    ///
    /// # Returns
    ///
    /// A new git_tree::Model instance populated with data from the tree
    ///
    /// # Panics
    ///
    /// This function will panic if the tree's data cannot be serialized
    fn into_git_model(self, meta: EntryMeta) -> Self::GitTarget {
        git_tree::Model {
            id: generate_id(),
            repo_id: 0,
            tree_id: self.id.to_string(),
            sub_trees: self.to_data().unwrap(),
            size: 0,
            pack_id: meta.pack_id.unwrap_or_default(),
            pack_offset: meta.pack_offset.unwrap_or(0) as i64,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}
