use std::str::FromStr;

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use mercury::hash::SHA1;

use crate::internal::branch::Branch;
use crate::internal::db::get_db_conn_instance;
use crate::internal::model::reference;

#[derive(Debug, Clone)]
pub enum Head {
    Detached(SHA1),
    Branch(String),
}

impl Head {
    async fn query_local_head() -> reference::Model {
        let db_conn = get_db_conn_instance().await;
        reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Head))
            .filter(reference::Column::Remote.is_null())
            .one(db_conn)
            .await
            .unwrap()
            .expect("fatal: storage broken, HEAD not found")
    }

    async fn query_remote_head(remote: &str) -> Option<reference::Model> {
        let db_conn = get_db_conn_instance().await;
        reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Head))
            .filter(reference::Column::Remote.eq(remote))
            .one(db_conn)
            .await
            .unwrap()
    }

    pub async fn current() -> Head {
        let head = Self::query_local_head().await;
        match head.name {
            Some(name) => Head::Branch(name),
            None => {
                // detached head
                let commit_hash = head.commit.expect("detached head without commit");
                Head::Detached(SHA1::from_str(commit_hash.as_str()).unwrap())
            }
        }
    }

    pub async fn remote_current(remote: &str) -> Option<Head> {
        match Self::query_remote_head(remote).await {
            Some(head) => match head.name {
                Some(name) => Some(Head::Branch(name)),
                None => {
                    let commit_hash = head.commit.expect("detached head without commit");
                    Some(Head::Detached(
                        SHA1::from_str(commit_hash.as_str()).unwrap(),
                    ))
                }
            },
            None => None,
        }
    }

    /// get the commit hash of the current head, return `None` if no commit
    pub async fn current_commit() -> Option<SHA1> {
        match Self::current().await {
            Head::Detached(commit_hash) => Some(commit_hash),
            Head::Branch(name) => {
                let branch = Branch::find_branch(&name, None).await;
                branch.map(|b| b.commit)
            }
        }
    }

    // HEAD is unique, update if exists, insert if not
    pub async fn update(new_head: Self, remote: Option<&str>) {
        let db_conn = get_db_conn_instance().await;

        let head = match remote {
            Some(remote) => Self::query_remote_head(remote).await,
            None => Some(Self::query_local_head().await),
        };
        match head {
            Some(head) => {
                // update
                let mut head: reference::ActiveModel = head.into();
                if remote.is_some() {
                    head.remote = Set(remote.map(|s| s.to_owned()));
                }
                match new_head {
                    Head::Detached(commit_hash) => {
                        head.commit = Set(Some(commit_hash.to_string()));
                        head.name = Set(None);
                    }
                    Head::Branch(branch_name) => {
                        head.name = Set(Some(branch_name));
                        head.commit = Set(None);
                    }
                }
                head.update(db_conn).await.unwrap();
            }
            None => {
                // // insert
                let mut head = reference::ActiveModel {
                    kind: Set(reference::ConfigKind::Head),
                    ..Default::default()
                };
                if remote.is_some() {
                    head.remote = Set(remote.map(|s| s.to_owned()));
                }
                match new_head {
                    Head::Detached(commit_hash) => {
                        head.commit = Set(Some(commit_hash.to_string()));
                    }
                    Head::Branch(branch_name) => {
                        head.name = Set(Some(branch_name));
                    }
                }
                head.save(db_conn).await.unwrap();
            }
        }
    }
}
