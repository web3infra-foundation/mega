use std::str::FromStr;
use sea_orm::DbConn;
use tokio::sync::OnceCell;
use venus::hash::SHA1;
use crate::db;
use crate::model::reference;

// singleton pattern
static DB_CONN: OnceCell<DbConn> = OnceCell::const_new();
async fn get_db_conn() -> &'static DbConn {
    DB_CONN.get_or_init(|| async {
        db::get_db_conn().await.unwrap()
    }).await
}

pub struct Branch;

impl Branch {
    /// list all local branches
    pub async fn list_local() -> Vec<String> {
        let db_conn = get_db_conn().await;
        let branches = reference::Model::find_all_branches(db_conn, None)
            .await
            .unwrap();
        branches.iter().map(|branch| branch.name.as_ref().unwrap().clone()).collect()
    }

    /// is the branch exists
    pub async fn exists(branch_name: &str) -> bool {
        let db_conn = get_db_conn().await;
        let branch = reference::Model::find_branch_by_name(db_conn, branch_name)
            .await
            .unwrap();
        branch.is_some()
    }

    /// Get the commit hash of a branch
    pub async fn current_commit(branch_name: &str) -> Option<SHA1> {
        let db_conn = get_db_conn().await;
        let branch = reference::Model::find_branch_by_name(db_conn, branch_name)
            .await
            .unwrap();
        match branch {
            Some(branch) => {
                let commit_hash = branch.commit;
                commit_hash.map(|hash| SHA1::from_str(&hash).unwrap())
            }
            None => {
                None // empty branch, no commit
            }
        }
    }
}