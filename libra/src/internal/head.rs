use std::str::FromStr;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DbConn};
use tokio::sync::OnceCell;
use venus::hash::SHA1;
use crate::db;
use crate::internal::branch::Branch;
use crate::model::reference;

// singleton pattern
static DB_CONN: OnceCell<DbConn> = OnceCell::const_new();
async fn get_db_conn() -> &'static DbConn {
    DB_CONN.get_or_init(|| async {
        db::get_db_conn().await.unwrap()
    }).await
}
pub enum Head {
    Detached(SHA1),
    Branch(String)
}

impl Head {
    pub async fn current() -> Head {
        let db_conn = get_db_conn().await;
        let head = reference::Model::current_head(db_conn)
            .await
            .unwrap();
        match head.name {
            Some(name) => {
                Head::Branch(name)
            }
            None => {
                // detached head
                let commit_hash = head.commit.expect("detached head without commit");
                Head::Detached(SHA1::from_str(commit_hash.as_str()).unwrap())
            }
        }
    }

    /// get the commit hash of the current head, return `None` if no commit
    pub async fn current_commit() -> Option<SHA1> {
        match Self::current().await {
            Head::Detached(commit_hash) => Some(commit_hash),
            Head::Branch(name) => {
                Branch::current_commit(&name).await
            },
        }
    }

    // HEAD is unique, update if exists, insert if not
    pub async fn update(branch_name: &str, remote: Option<&str>) {
        let db_conn = get_db_conn().await;

        let head = match remote {
            Some(remote) => {
                reference::Model::current_head_remote(db_conn, remote).await.unwrap()
            },
            None => {
                Some(reference::Model::current_head(db_conn).await.unwrap())
            }
        };

        match head {
            Some(head) => {
                // update
                let mut head: reference::ActiveModel = head.into();
                head.name = Set(Some(branch_name.to_owned()));
                if remote.is_some() {
                    head.remote = Set(remote.map(|s| s.to_owned()));
                }
                head.update(db_conn).await.unwrap();
            },
            None => {
                // insert
                let head = reference::ActiveModel {
                    name: Set(Some(branch_name.to_owned())),
                    kind: Set(reference::ConfigKind::Head),
                    remote: Set(remote.map(|s| s.to_owned())),
                    ..Default::default()
                };
                head.save(db_conn).await.unwrap();
            }
        }
    }
}