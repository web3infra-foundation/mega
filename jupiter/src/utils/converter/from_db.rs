use std::str::FromStr;

use callisto::{git_commit, git_tag, git_tree, mega_commit, mega_tag, mega_tree};
use git_internal::{
    hash::ObjectHash,
    internal::object::{
        ObjectTrait, commit::Commit, signature::Signature, tag::Tag, tree::Tree, types::ObjectType,
    },
};

use super::traits::{FromGitModel, FromMegaModel};

/// Helper function to convert commit model data to Commit object
fn commit_from_model(
    commit_id: &str,
    tree: &str,
    parents_id: &serde_json::Value,
    author: Option<String>,
    committer: Option<String>,
    content: Option<String>,
) -> Commit {
    // Parse parents_id JSON array into Vec<ObjectHash>
    let parent_commit_ids: Vec<ObjectHash> =
        match serde_json::from_value::<Vec<String>>(parents_id.clone()) {
            Ok(parents_array) => parents_array
                .into_iter()
                .filter(|s: &String| !s.is_empty())
                .map(|s: String| ObjectHash::from_str(&s).unwrap())
                .collect(),
            Err(_) => Vec::new(),
        };

    Commit {
        id: ObjectHash::from_str(commit_id).unwrap(),
        tree_id: ObjectHash::from_str(tree).unwrap(),
        parent_commit_ids,
        author: Signature::from_data(author.unwrap().into_bytes()).unwrap(),
        committer: Signature::from_data(committer.unwrap().into_bytes()).unwrap(),
        message: content.unwrap(),
    }
}
// Reverse conversion implementations
impl FromMegaModel for Tag {
    type MegaSource = mega_tag::Model;

    /// Converts a mega_tag::Model to a Tag object
    ///
    /// This function reconstructs a Tag object from a mega_tag::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to a Signature.
    ///
    /// # Arguments
    ///
    /// * `model` - The mega_tag::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tag instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tag_id string cannot be parsed into a valid ObjectHash
    /// - The object_id string cannot be parsed into a valid ObjectHash
    /// - The object_type string is not a recognized ObjectType
    /// - The tagger string cannot be converted into a valid Signature
    fn from_mega_model(model: Self::MegaSource) -> Self {
        Tag {
            id: ObjectHash::from_str(&model.tag_id).expect("Invalid tag_id in database"),
            object_hash: ObjectHash::from_str(&model.object_id).unwrap(),
            object_type: ObjectType::from_string(&model.object_type).unwrap(),
            tag_name: model.tag_name,
            tagger: Signature::from_data(model.tagger.into_bytes()).unwrap(),
            message: model.message,
        }
    }
}

impl FromGitModel for Tag {
    type GitSource = git_tag::Model;

    /// Converts a git_tag::Model to a Tag object
    ///
    /// This function reconstructs a Tag object from a git_tag::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to a Signature.
    ///
    /// # Arguments
    ///
    /// * `model` - The git_tag::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tag instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tag_id string cannot be parsed into a valid ObjectHash
    /// - The object_id string cannot be parsed into a valid ObjectHash
    /// - The object_type string is not a recognized ObjectType
    /// - The tagger string cannot be converted into a valid Signature
    fn from_git_model(model: Self::GitSource) -> Self {
        Tag {
            id: ObjectHash::from_str(&model.tag_id).unwrap(),
            object_hash: ObjectHash::from_str(&model.object_id).unwrap(),
            object_type: ObjectType::from_string(&model.object_type).unwrap(),
            tag_name: model.tag_name,
            tagger: Signature::from_data(model.tagger.into_bytes()).unwrap(),
            message: model.message,
        }
    }
}

impl FromMegaModel for Tree {
    type MegaSource = mega_tree::Model;

    /// Converts a mega_tree::Model to a Tree object
    ///
    /// This function reconstructs a Tree object from a mega_tree::Model retrieved from the database.
    /// It parses the binary sub_trees data back into a structured Tree object and
    /// uses the tree_id string to recreate the ObjectHash identifier.
    ///
    /// # Arguments
    ///
    /// * `model` - The mega_tree::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tree instance reconstructed from the model data
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tree_id string cannot be parsed into a valid ObjectHash
    /// - The binary sub_trees data cannot be parsed into a valid Tree structure
    fn from_mega_model(model: Self::MegaSource) -> Self {
        Tree::from_bytes(
            &model.sub_trees,
            ObjectHash::from_str(&model.tree_id).unwrap(),
        )
        .unwrap()
    }
}

impl FromGitModel for Tree {
    type GitSource = git_tree::Model;

    /// Converts a git_tree::Model to a Tree object
    ///
    /// This function reconstructs a Tree object from a git_tree::Model retrieved from the database.
    /// It parses the binary sub_trees data back into a structured Tree object and
    /// uses the tree_id string to recreate the ObjectHash hash identifier.
    ///
    /// # Arguments
    ///
    /// * `model` - The git_tree::Model to convert
    ///
    /// # Returns
    ///
    /// A new Tree instance reconstructed from the model data
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The tree_id string cannot be parsed into a valid ObjectHash
    /// - The binary sub_trees data cannot be parsed into a valid Tree structure
    fn from_git_model(model: Self::GitSource) -> Self {
        Tree::from_bytes(
            &model.sub_trees,
            ObjectHash::from_str(&model.tree_id).unwrap(),
        )
        .unwrap()
    }
}

impl FromMegaModel for Commit {
    type MegaSource = mega_commit::Model;

    /// Converts a mega_commit::Model to a Commit object
    ///
    /// This function reconstructs a Commit object from a mega_commit::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to Signature objects.
    ///
    /// # Arguments
    ///
    /// * `model` - The mega_commit::Model to convert
    ///
    /// # Returns
    ///
    /// A new Commit instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The commit_id string cannot be parsed into a valid ObjectHash
    /// - The tree string cannot be parsed into a valid ObjectHash
    /// - Any parent ID in parents_id cannot be parsed into a valid ObjectHash
    /// - The author or committer strings cannot be converted into valid Signatures
    fn from_mega_model(model: Self::MegaSource) -> Self {
        commit_from_model(
            &model.commit_id,
            &model.tree,
            &model.parents_id,
            model.author,
            model.committer,
            model.content,
        )
    }
}

impl FromGitModel for Commit {
    type GitSource = git_commit::Model;

    /// Converts a git_commit::Model to a Commit object
    ///
    /// This function reconstructs a Commit object from a git_commit::Model retrieved from the database.
    /// It parses the stored strings back into their original types, such as converting
    /// string IDs back to ObjectHash hashes and string data back to Signature objects.
    ///
    /// # Arguments
    ///
    /// * `model` - The git_commit::Model to convert
    ///
    /// # Returns
    ///
    /// A new Commit instance populated with data from the model
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The commit_id string cannot be parsed into a valid ObjectHash hash
    /// - The tree string cannot be parsed into a valid ObjectHash hash
    /// - Any parent ID in parents_id cannot be parsed into a valid ObjectHash hash
    /// - The author or committer strings cannot be converted into valid Signatures
    fn from_git_model(model: Self::GitSource) -> Self {
        commit_from_model(
            &model.commit_id,
            &model.tree,
            &model.parents_id,
            model.author,
            model.committer,
            model.content,
        )
    }
}
