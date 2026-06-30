use callisto::{
    git_blob, git_commit, git_tag, git_tree, mega_blob, mega_commit, mega_tag, mega_tree,
};
use git_internal::internal::{
    metadata::EntryMeta,
    object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree},
};

pub trait IntoMegaModel {
    type MegaTarget;
    fn into_mega_model(self, ext_meta: EntryMeta) -> Self::MegaTarget;
}

pub trait IntoGitModel {
    type GitTarget;
    fn into_git_model(self, ext_meta: EntryMeta) -> Self::GitTarget;
}

pub trait FromMegaModel {
    type MegaSource;
    fn from_mega_model(model: Self::MegaSource) -> Self;
}

pub trait FromGitModel {
    type GitSource;
    fn from_git_model(model: Self::GitSource) -> Self;
}

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
    Blob(git_blob::Model, Vec<u8>),
    Tag(git_tag::Model),
}

pub enum MegaObjectModel {
    Commit(mega_commit::Model),
    Tree(mega_tree::Model),
    Blob(mega_blob::Model, Vec<u8>),
    Tag(mega_tag::Model),
}

impl GitObject {
    pub fn convert_to_mega_model(self, meta: EntryMeta) -> MegaObjectModel {
        match self {
            GitObject::Commit(commit) => MegaObjectModel::Commit(commit.into_mega_model(meta)),
            GitObject::Tree(tree) => MegaObjectModel::Tree(tree.into_mega_model(meta)),
            GitObject::Blob(blob) => {
                let blob_data = blob.data.clone();
                let mega_model = blob.into_mega_model(meta);
                MegaObjectModel::Blob(mega_model, blob_data)
            }
            GitObject::Tag(tag) => MegaObjectModel::Tag(tag.into_mega_model(meta)),
        }
    }

    pub fn convert_to_git_model(self, meta: EntryMeta) -> GitObjectModel {
        match self {
            GitObject::Commit(commit) => GitObjectModel::Commit(commit.into_git_model(meta)),
            GitObject::Tree(tree) => GitObjectModel::Tree(tree.into_git_model(meta)),
            GitObject::Blob(blob) => {
                let blob_data = blob.data.clone();
                let git_model = blob.into_git_model(meta);
                GitObjectModel::Blob(git_model, blob_data)
            }
            GitObject::Tag(tag) => GitObjectModel::Tag(tag.into_git_model(meta)),
        }
    }
}
