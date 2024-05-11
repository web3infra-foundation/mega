use std::str::FromStr;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;

use venus::hash::SHA1;

use crate::internal::db::get_db_conn_instance;
use crate::internal::model::reference;

pub struct Branch {
    pub name: String,
    pub commit: SHA1,
    pub remote: Option<String>,
}

async fn query_reference(branch_name: &str, remote: Option<&str>) -> Option<reference::Model> {
    let db_conn = get_db_conn_instance().await;
    reference::Entity::find()
        .filter(reference::Column::Name.eq(branch_name))
        .filter(reference::Column::Kind.eq(reference::ConfigKind::Branch))
        .filter(match remote {
            Some(remote) => reference::Column::Remote.eq(remote),
            None => reference::Column::Remote.is_null(),
        })
        .one(db_conn)
        .await
        .unwrap()
}

impl Branch {
    /// list all remote branches
    pub async fn list_branches(remote: Option<&str>) -> Vec<Self> {
        let db_conn = get_db_conn_instance().await;

        let branches = reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Branch))
            .filter(match remote {
                Some(remote) => reference::Column::Remote.eq(remote),
                None => reference::Column::Remote.is_null(),
            })
            .all(db_conn)
            .await
            .unwrap();

        branches
            .iter()
            .map(|branch| Branch {
                name: branch.name.as_ref().unwrap().clone(),
                commit: SHA1::from_str(branch.commit.as_ref().unwrap()).unwrap(),
                remote: branch.remote.clone(),
            })
            .collect()
    }

    /// is the branch exists
    pub async fn exists(branch_name: &str) -> bool {
        let branch = Self::find_branch(branch_name, None).await;
        branch.is_some()
    }

    /// get the branch by name
    pub async fn find_branch(branch_name: &str, remote: Option<&str>) -> Option<Self> {
        let branch = query_reference(branch_name, remote).await;
        match branch {
            Some(branch) => Some(Branch {
                name: branch.name.as_ref().unwrap().clone(),
                commit: SHA1::from_str(branch.commit.as_ref().unwrap()).unwrap(),
                remote: branch.remote.clone(),
            }),
            None => None,
        }
    }

    pub async fn update_branch(branch_name: &str, commit_hash: &str, remote: Option<&str>) {
        let db_conn = get_db_conn_instance().await;
        // check if branch exists
        let branch = query_reference(branch_name, remote).await;

        match branch {
            Some(branch) => {
                let mut branch: reference::ActiveModel = branch.into();
                branch.commit = Set(Some(commit_hash.to_owned()));
                branch.update(db_conn).await.unwrap();
            }
            None => {
                reference::ActiveModel {
                    name: Set(Some(branch_name.to_owned())),
                    kind: Set(reference::ConfigKind::Branch),
                    commit: Set(Some(commit_hash.to_owned())),
                    remote: Set(remote.map(|s| s.to_owned())),
                    ..Default::default()
                }
                .insert(db_conn)
                .await
                .unwrap();
            }
        }
    }

    pub async fn delete_branch(branch_name: &str, remote: Option<&str>) {
        let db_conn = get_db_conn_instance().await;
        let branch: reference::ActiveModel =
            query_reference(branch_name, remote).await.unwrap().into();
        branch.delete(db_conn).await.unwrap();
    }
}
