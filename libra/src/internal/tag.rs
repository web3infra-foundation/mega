use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::str::FromStr;

use crate::command::load_object;
use crate::internal::config::Config;
use crate::internal::db::get_db_conn_instance;
use crate::internal::head::Head;
use crate::internal::model::reference;
use crate::utils::client_storage::ClientStorage;
use crate::utils::path;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::ObjectTrait;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::signature::{Signature, SignatureType};
use mercury::internal::object::tag::Tag as MercuryTag;
use mercury::internal::object::tree::Tree;
use mercury::internal::object::types::ObjectType;

// Constants for tag references
const TAG_REF_PREFIX: &str = "refs/tags/";
const DEFAULT_USER: &str = "user";
const DEFAULT_EMAIL: &str = "user@example.com";
const UNKNOWN_TAG: &str = "<unknown>";

/// Enum representing the possible object types a tag can point to.
#[derive(Debug)]
pub enum TagObject {
    Commit(Commit),
    Tag(MercuryTag),
    Tree(Tree),
    Blob(Blob),
}

impl TagObject {
    pub fn get_type(&self) -> ObjectType {
        match self {
            TagObject::Commit(_) => ObjectType::Commit,
            TagObject::Tag(_) => ObjectType::Tag,
            TagObject::Tree(_) => ObjectType::Tree,
            TagObject::Blob(_) => ObjectType::Blob,
        }
    }

    pub fn to_data(&self) -> Result<Vec<u8>, GitError> {
        match self {
            TagObject::Commit(c) => c.to_data(),
            TagObject::Tag(t) => t.to_data(),
            TagObject::Tree(t) => t.to_data(),
            TagObject::Blob(b) => b.to_data(),
        }
    }
}

/// Represents a tag in the context of Libra, containing its name and the object it points to.
pub struct Tag {
    pub name: String,
    pub object: TagObject,
}

/// Creates a new tag, either lightweight or annotated, pointing to the current HEAD commit.
///
/// * `name` - The name of the tag.
/// * `message` - If `Some`, creates an annotated tag with the given message. If `None`, creates a lightweight tag.
pub async fn create(name: &str, message: Option<String>) -> Result<(), anyhow::Error> {
    let head_commit_id = Head::current_commit()
        .await
        .ok_or_else(|| anyhow::anyhow!("Cannot create tag: HEAD does not point to a commit"))?;

    let ref_target_id: SHA1;
    if let Some(msg) = message {
        // Create an annotated tag object
        let user_name = Config::get("user", None, "name")
            .await
            .unwrap_or_else(|| DEFAULT_USER.to_string());
        let user_email = Config::get("user", None, "email")
            .await
            .unwrap_or_else(|| DEFAULT_EMAIL.to_string());
        let tagger_signature = Signature::new(SignatureType::Tagger, user_name, user_email);

        let mercury_tag = MercuryTag::new(
            head_commit_id,
            ObjectType::Commit,
            name.to_string(),
            tagger_signature,
            msg,
        );

        // The ID is now calculated inside MercuryTag::new, so we can use it directly.
        let tag_data = mercury_tag.to_data()?;
        let storage = ClientStorage::init(path::objects());
        storage.put(&mercury_tag.id, &tag_data, mercury_tag.get_type())?;

        ref_target_id = mercury_tag.id;
    } else {
        // For lightweight tags, the target is the commit itself
        ref_target_id = head_commit_id;
    };

    // Save the reference in the database
    let db_conn = get_db_conn_instance().await;
    let new_ref = reference::ActiveModel {
        name: Set(Some(format!("{}{}", TAG_REF_PREFIX, name))),
        kind: Set(reference::ConfigKind::Tag),
        commit: Set(Some(ref_target_id.to_string())),
        ..Default::default()
    };
    new_ref.insert(db_conn).await?;

    Ok(())
}

/// Lists all tags available in the repository.
pub async fn list() -> Result<Vec<Tag>, anyhow::Error> {
    let db_conn = get_db_conn_instance().await;
    let models = reference::Entity::find()
        .filter(reference::Column::Kind.eq(reference::ConfigKind::Tag))
        .all(db_conn)
        .await?;

    let mut tags = Vec::new();
    for m in models {
        let commit_str = m.commit.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Tag '{}' is missing commit field",
                m.name.as_deref().unwrap_or(UNKNOWN_TAG)
            )
        })?;
        let object_id =
            SHA1::from_str(commit_str).map_err(|e| anyhow::anyhow!("Invalid SHA1: {}", e))?;
        let object = load_object_trait(&object_id).await?;
        let tag_name = m
            .name
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tag is missing name field"))?
            .strip_prefix(TAG_REF_PREFIX)
            .unwrap_or_else(|| m.name.as_ref().expect("Name field should exist"))
            .to_string();
        tags.push(Tag {
            name: tag_name,
            object,
        });
    }
    Ok(tags)
}

/// Deletes a tag reference from the repository.
pub async fn delete(name: &str) -> Result<(), anyhow::Error> {
    let db_conn = get_db_conn_instance().await;
    let full_ref_name = format!("{}{}", TAG_REF_PREFIX, name);

    let result = reference::Entity::delete_many()
        .filter(reference::Column::Name.eq(full_ref_name))
        .filter(reference::Column::Kind.eq(reference::ConfigKind::Tag))
        .exec(db_conn)
        .await?;

    if result.rows_affected == 0 {
        Err(anyhow::anyhow!("tag '{}' not found", name))
    } else {
        Ok(())
    }
}

/// Finds a tag by name and returns the tag object and the final commit
pub async fn find_tag_and_commit(name: &str) -> Result<Option<(TagObject, Commit)>, GitError> {
    let db_conn = get_db_conn_instance().await;
    let full_ref_name = format!("{}{}", TAG_REF_PREFIX, name);

    let model = reference::Entity::find()
        .filter(reference::Column::Name.eq(full_ref_name))
        .filter(reference::Column::Kind.eq(reference::ConfigKind::Tag))
        .one(db_conn)
        .await
        .map_err(|e| GitError::CustomError(e.to_string()))?;

    if let Some(m) = model {
        let commit_str = m
            .commit
            .as_ref()
            .ok_or_else(|| GitError::CustomError("Tag is missing commit field".to_string()))?;
        let target_id = SHA1::from_str(commit_str)
            .map_err(|_| GitError::InvalidHashValue(commit_str.to_string()))?;
        let ref_object = load_object_trait(&target_id).await?;

        // If the ref points to a tag object, dereference it to get the commit
        let commit_id = if let TagObject::Tag(tag_object) = &ref_object {
            tag_object.object_hash
        } else {
            target_id
        };

        let commit: Commit = load_object(&commit_id)?;
        Ok(Some((ref_object, commit)))
    } else {
        Ok(None)
    }
}

/// Load a Git object and return it as a `TagObject`.
pub async fn load_object_trait(hash: &SHA1) -> Result<TagObject, GitError> {
    // Use ClientStorage to get the object type first
    let storage = ClientStorage::init(path::objects());
    let obj_type = storage
        .get_object_type(hash)
        .map_err(|e| GitError::ObjectNotFound(format!("{}: {}", hash, e)))?;
    match obj_type {
        ObjectType::Commit => {
            let commit = load_object::<Commit>(hash)
                .map_err(|e| GitError::ObjectNotFound(format!("{}: {}", hash, e)))?;
            Ok(TagObject::Commit(commit))
        }
        ObjectType::Tag => {
            let tag = load_object::<MercuryTag>(hash)
                .map_err(|e| GitError::ObjectNotFound(format!("{}: {}", hash, e)))?;
            Ok(TagObject::Tag(tag))
        }
        ObjectType::Tree => {
            let tree = load_object::<Tree>(hash)
                .map_err(|e| GitError::ObjectNotFound(format!("{}: {}", hash, e)))?;
            Ok(TagObject::Tree(tree))
        }
        ObjectType::Blob => {
            let blob = load_object::<Blob>(hash)
                .map_err(|e| GitError::ObjectNotFound(format!("{}: {}", hash, e)))?;
            Ok(TagObject::Blob(blob))
        }
        _ => Err(GitError::ObjectNotFound(hash.to_string())),
    }
}
