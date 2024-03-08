use callisto::mega_tag;
use common::utils::generate_id;

use crate::internal::object::tag::Tag;



impl From<Tag> for mega_tag::Model {
    fn from(value: Tag) -> Self {
        mega_tag::Model {
            id: generate_id(),
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