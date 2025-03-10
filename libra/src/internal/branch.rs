use std::str::FromStr;

use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use mercury::hash::SHA1;

use crate::internal::db::get_db_conn_instance;
use crate::internal::model::reference;

#[derive(Debug)]
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

    /// search branch with full name, return vec of branches
    /// e.g. `origin/sub/master/feature` may means `origin/sub/master` + `feature` or `origin/sub` + `master/feature`
    /// so we need to search all possible branches
    pub async fn search_branch(branch_name: &str) -> Vec<Self> {
        let mut branch_name = branch_name.to_string();
        let mut remote = String::new();

        let mut branches = vec![];
        if let Some(branch) = Self::find_branch(&branch_name, None).await {
            branches.push(branch)
        }

        while let Some(index) = branch_name.find('/') {
            if !remote.is_empty() {
                remote += "/";
            }
            remote += branch_name.get(..index).unwrap();
            branch_name = branch_name.get(index + 1..).unwrap().to_string();
            let branch = Self::find_branch(&branch_name, Some(&remote)).await;
            if let Some(branch) = branch {
                branches.push(branch);
            }
        }
        branches
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

#[cfg(test)]
mod tests {
    use crate::utils::test;
    use serial_test::serial;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_search_branch() {
        test::setup_with_new_libra().await;

        let commit_hash = SHA1::default().to_string();
        Branch::update_branch("upstream/origin/master", &commit_hash, None).await; // should match
        Branch::update_branch("origin/master", &commit_hash, Some("upstream")).await; // should match
        Branch::update_branch("master", &commit_hash, Some("upstream/origin")).await; // should match
        Branch::update_branch("feature", &commit_hash, Some("upstream/origin/master")).await; // should not match

        let branches = Branch::search_branch("upstream/origin/master").await;
        assert_eq!(branches.len(), 3);
    }
}
