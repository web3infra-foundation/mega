use std::str::FromStr;

use sea_orm::ActiveValue::Set;
use sea_orm::ConnectionTrait;
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

/*
 * =================================================================================
 * NOTE: Transaction Safety Pattern (`_with_conn`)
 * =================================================================================
 *
 * This module follows the `_with_conn` pattern for transaction safety.
 *
 * - Public functions (e.g., `get`, `update`) acquire a new database
 *   connection from the pool and are suitable for single, non-transactional operations.
 *
 * - `*_with_conn` variants (e.g., `get_with_conn`, `update_with_conn`)
 *   accept an existing connection or transaction handle (`&C where C: ConnectionTrait`).
 *
 * **WARNING**: To use these functions within a database transaction (e.g., inside
 * a `db.transaction(|txn| { ... })` block), you MUST call the `*_with_conn`
 * variant, passing the transaction handle `txn`. Calling a public version from
 * inside a transaction will try to acquire a second connection from the pool,
 * leading to a deadlock.
 *
 * Correct Usage (in a transaction): `Head::update_with_conn(txn, ...).await;`
 * Incorrect Usage (in a transaction): `Head::update(...).await;` // DEADLOCK!
 */

impl Head {
    async fn query_local_head_with_conn<C>(db: &C) -> reference::Model
    where
        C: ConnectionTrait,
    {
        reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Head))
            .filter(reference::Column::Remote.is_null())
            .one(db)
            .await
            .unwrap()
            .expect("fatal: storage broken, HEAD not found")
    }

    async fn query_remote_head_with_conn<C>(db: &C, remote: &str) -> Option<reference::Model>
    where
        C: ConnectionTrait,
    {
        reference::Entity::find()
            .filter(reference::Column::Kind.eq(reference::ConfigKind::Head))
            .filter(reference::Column::Remote.eq(remote))
            .one(db)
            .await
            .unwrap()
    }

    pub async fn current_with_conn<C>(db: &C) -> Head
    where
        C: ConnectionTrait,
    {
        let head = Self::query_local_head_with_conn(db).await;
        match head.name {
            Some(name) => Head::Branch(name),
            None => {
                let commit_hash = head.commit.expect("detached head without commit");
                Head::Detached(SHA1::from_str(commit_hash.as_str()).unwrap())
            }
        }
    }

    pub async fn current() -> Head {
        let db_conn = get_db_conn_instance().await;
        Self::current_with_conn(db_conn).await
    }

    pub async fn remote_current_with_conn<C>(db: &C, remote: &str) -> Option<Head>
    where
        C: ConnectionTrait,
    {
        match Self::query_remote_head_with_conn(db, remote).await {
            Some(head) => Some(match head.name {
                Some(name) => Head::Branch(name),
                None => {
                    let commit_hash = head.commit.expect("detached head without commit");
                    Head::Detached(SHA1::from_str(commit_hash.as_str()).unwrap())
                }
            }),
            None => None,
        }
    }

    pub async fn remote_current(remote: &str) -> Option<Head> {
        let db_conn = get_db_conn_instance().await;
        Self::remote_current_with_conn(db_conn, remote).await
    }

    pub async fn current_commit_with_conn<C>(db: &C) -> Option<SHA1>
    where
        C: ConnectionTrait,
    {
        match Self::current_with_conn(db).await {
            Head::Detached(commit_hash) => Some(commit_hash),
            Head::Branch(name) => {
                let branch = Branch::find_branch_with_conn(db, &name, None).await;
                branch.map(|b| b.commit)
            }
        }
    }

    /// get the commit hash of current head, return `None` if no commit
    pub async fn current_commit() -> Option<SHA1> {
        let db_conn = get_db_conn_instance().await;
        Self::current_commit_with_conn(db_conn).await
    }

    pub async fn update_with_conn<C>(db: &C, new_head: Self, remote: Option<&str>)
    where
        C: ConnectionTrait,
    {
        let head = match remote {
            Some(remote) => Self::query_remote_head_with_conn(db, remote).await,
            None => Some(Self::query_local_head_with_conn(db).await),
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
                head.update(db).await.unwrap();
            }
            None => {
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
                head.save(db).await.unwrap();
            }
        }
    }

    // HEAD is unique, update if exists, insert if not
    pub async fn update(new_head: Self, remote: Option<&str>) {
        let db_conn = get_db_conn_instance().await;
        Self::update_with_conn(db_conn, new_head, remote).await;
    }
}
