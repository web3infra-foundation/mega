use std::str::FromStr;

use callisto::mega_tag;
use common::utils::generate_id;

use crate::{hash::SHA1, internal::object::{signature::Signature, tag::Tag, types::ObjectType}};

impl From<Tag> for mega_tag::Model {
    fn from(value: Tag) -> Self {
        mega_tag::Model {
            id: generate_id(),
            repo_id: 0,
            tag_id: value.id.to_plain_str(),
            object_id: value.object_hash.to_plain_str(),
            object_type: value.object_type.to_string(),
            tag_name: value.tag_name,
            tagger: value.tagger.to_string(),
            message: value.message,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<mega_tag::Model> for Tag {
    fn from(value: mega_tag::Model) -> Self {
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
