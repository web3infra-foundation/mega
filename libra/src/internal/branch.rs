use crate::db;
use crate::model::reference;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DbConn};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::str::FromStr;
use tokio::sync::OnceCell;
use venus::hash::SHA1;

// singleton pattern
static DB_CONN: OnceCell<DbConn> = OnceCell::const_new();
async fn get_db_conn() -> &'static DbConn {
    DB_CONN
        .get_or_init(|| async { db::get_db_conn().await.unwrap() })
        .await
}

pub struct Branch {
    pub name: String,
    pub commit: SHA1,
    pub remote: Option<String>,
}

impl Branch {
    #[allow(dead_code)]
    /// list all local branches
    pub async fn list_local() -> Vec<Self> {
        let db_conn = get_db_conn().await;

        let branches = reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Branch))
            .filter(reference::Column::Remote.is_null())
            .all(db_conn)
            .await
            .unwrap();

        branches
            .iter()
            .map(|branch| {
                let commit_hash = branch.commit.as_ref().unwrap();
                let commit_hash = SHA1::from_str(commit_hash).unwrap();
                Branch {
                    name: branch.name.as_ref().unwrap().clone(),
                    commit: commit_hash,
                    remote: branch.remote.clone(),
                }
            })
            .collect()
    }

    /// list all remote branches
    pub async fn list_remotes() -> Vec<Self> {
        let db_conn = get_db_conn().await;

        let branches = reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Branch))
            .filter(reference::Column::Remote.is_not_null())
            .all(db_conn)
            .await
            .unwrap();

        branches
            .iter()
            .map(|branch| {
                let commit_hash = branch.commit.as_ref().unwrap();
                let commit_hash = SHA1::from_str(commit_hash).unwrap();
                Branch {
                    name: branch.name.as_ref().unwrap().clone(),
                    commit: commit_hash,
                    remote: branch.remote.clone(),
                }
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
        let db = get_db_conn().await;
        let branch = reference::Entity::find()
            .filter(reference::Column::Name.eq(branch_name))
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Branch))
            .filter(match remote {
                Some(remote) => reference::Column::Remote.eq(remote),
                None => reference::Column::Remote.is_null(),
            })
            .one(db)
            .await
            .unwrap();
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
        let db_conn = get_db_conn().await;
        // check if branch exists
        let branch = reference::Entity::find()
            .filter(reference::Column::Name.eq(branch_name))
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Branch))
            .filter(match remote {
                Some(remote) => reference::Column::Remote.eq(remote),
                None => reference::Column::Remote.is_null(),
            })
            .one(db_conn)
            .await
            .unwrap();

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
        let db_conn = get_db_conn().await;
        let branch: reference::ActiveModel = reference::Entity::find()
            .filter(reference::Column::Name.eq(branch_name))
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Branch))
            .filter(match remote {
                Some(remote) => reference::Column::Remote.eq(remote),
                None => reference::Column::Remote.is_null(),
            })
            .one(db_conn)
            .await
            .unwrap()
            .unwrap()
            .into();
        branch.delete(db_conn).await.unwrap();
    }
}
