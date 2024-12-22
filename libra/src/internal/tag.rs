use std::str::FromStr;

use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use mercury::hash::SHA1;

use crate::internal::db::get_db_conn_instance;
use crate::internal::model::reference;

pub struct TagInfo {
    pub name: String,
    pub commit: SHA1,
}

impl TagInfo {
    
    pub async fn query_reference(tag_name: &str) -> Option<reference::Model> {
        let db_conn = get_db_conn_instance().await;
        reference::Entity::find()
            .filter(reference::Column::Name.eq(tag_name))
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Tag))
            .one(db_conn)
            .await
            .unwrap()
    }

    /// list all tags
    pub async fn list_tags() -> Vec<TagInfo> {
        let db_conn = get_db_conn_instance().await;
        let tags = reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Tag))
            .all(db_conn)
            .await
            .unwrap();

        tags
            .iter()
            .map(|tag| TagInfo {
                name: tag.name.as_ref().unwrap().clone(),
                commit: SHA1::from_str(tag.commit.as_ref().unwrap()).unwrap(),
            })
            .collect()
    }

    /// is the tag exists
    pub async fn exists(tag_name: &str) -> bool {
        let tag = Self::find_tag(tag_name).await;
        tag.is_some()
    }

    /// get the tag by name
    pub async fn find_tag(tag_name: &str) -> Option<TagInfo> {
        let tag = Self::query_reference(tag_name).await;
        match tag {
            Some(tag) => Some(TagInfo {
                name: tag.name.as_ref().unwrap().clone(),
                commit: SHA1::from_str(tag.commit.as_ref().unwrap()).unwrap(),
            }),
            None => None,
        }
    }

    /// update the tag
    pub async fn update_tag(tag_name: &str, commit_hash: &str) {
        let db_conn = get_db_conn_instance().await;
        // check if tag exists
        let tag = Self::query_reference(tag_name).await;

        match tag {
            Some(tag) => {
                let mut tag: reference::ActiveModel = tag.into();
                tag.commit = Set(Some(commit_hash.to_owned()));
                tag.update(db_conn).await.unwrap();
            }
            None => {
                reference::ActiveModel {
                    name: Set(Some(tag_name.to_owned())),
                    kind: Set(reference::ConfigKind::Tag),
                    commit: Set(Some(commit_hash.to_owned())),
                    ..Default::default()
                }
                .insert(db_conn)
                .await
                .unwrap();
            }
        }
    }

    pub async fn delete_tag(tag_name: &str) {
        let db_conn = get_db_conn_instance().await;
        let tag: reference::ActiveModel =
            Self::query_reference(tag_name).await.unwrap().into();
        tag.delete(db_conn).await.unwrap();
    }


}
