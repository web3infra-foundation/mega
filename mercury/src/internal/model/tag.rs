use std::str::FromStr;

use crate::internal::model::sea_models::{git_tag as sea_git_tag, mega_tag as sea_mega_tag};

use crate::internal::model::generate_id;
use crate::{
    hash::SHA1,
    internal::object::{signature::Signature, tag::Tag, types::ObjectType},
};

impl From<Tag> for sea_mega_tag::Model {
    fn from(value: Tag) -> Self {
        sea_mega_tag::Model {
            id: generate_id(),
            tag_id: value.id.to_string(),
            object_id: value.object_hash.to_string(),
            object_type: value.object_type.to_string(),
            tag_name: value.tag_name,
            tagger: String::from_utf8_lossy(&value.tagger.to_data().unwrap()).to_string(),
            message: value.message,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Tag> for sea_git_tag::Model {
    fn from(value: Tag) -> Self {
        sea_git_tag::Model {
            id: generate_id(),
            repo_id: 0,
            tag_id: value.id.to_string(),
            object_id: value.object_hash.to_string(),
            object_type: value.object_type.to_string(),
            tag_name: value.tag_name,
            tagger: String::from_utf8_lossy(&value.tagger.to_data().unwrap()).to_string(),
            message: value.message,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<sea_mega_tag::Model> for Tag {
    fn from(value: sea_mega_tag::Model) -> Self {
        Self {
            id: SHA1::from_str(&value.tag_id).expect("Invalid tag_id in database"),
            object_hash: SHA1::from_str(&value.object_id).unwrap(),
            object_type: ObjectType::from_string(&value.object_type).unwrap(),
            tag_name: value.tag_name,
            tagger: Signature::from_data(value.tagger.into_bytes()).unwrap(),
            message: value.message,
        }
    }
}

impl From<sea_git_tag::Model> for Tag {
    fn from(value: sea_git_tag::Model) -> Self {
        Self {
            id: SHA1::from_str(&value.tag_id).unwrap(),
            object_hash: SHA1::from_str(&value.object_id).unwrap(),
            object_type: ObjectType::from_string(&value.object_type).unwrap(),
            tag_name: value.tag_name,
            tagger: Signature::from_data(value.tagger.into_bytes()).unwrap(),
            message: value.message,
        }
    }
}
