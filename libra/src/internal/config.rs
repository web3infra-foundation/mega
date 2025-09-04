use std::collections::HashSet;
use std::mem::swap;

use sea_orm::entity::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ModelTrait, QueryFilter};

use crate::internal::db::get_db_conn_instance;
use crate::internal::head::Head;
use crate::internal::model::config::{self, ActiveModel, Model};

pub struct Config;

#[derive(Clone)]
pub struct RemoteConfig {
    pub name: String,
    pub url: String,
}
#[allow(dead_code)]
pub struct BranchConfig {
    pub name: String,
    pub merge: String,
    pub remote: String,
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
 * Correct Usage (in a transaction): `Config::update_with_conn(txn, ...).await;`
 * Incorrect Usage (in a transaction): `Config::update(...).await;` // DEADLOCK!
 */
impl Config {
    // _with_conn version for insert
    pub async fn insert_with_conn<C: ConnectionTrait>(
        db: &C,
        configuration: &str,
        name: Option<&str>,
        key: &str,
        value: &str,
    ) {
        let config = ActiveModel {
            configuration: Set(configuration.to_owned()),
            name: Set(name.map(|s| s.to_owned())),
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
            ..Default::default()
        };
        config.save(db).await.unwrap();
    }

    // _with_conn version for update
    pub async fn update_with_conn<C: ConnectionTrait>(
        db: &C,
        configuration: &str,
        name: Option<&str>,
        key: &str,
        value: &str,
    ) -> Model {
        let mut config: ActiveModel = config::Entity::find()
            .filter(config::Column::Configuration.eq(configuration))
            .filter(match name {
                Some(str) => config::Column::Name.eq(str),
                None => config::Column::Name.is_null(),
            })
            .filter(config::Column::Key.eq(key))
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .into();
        config.value = Set(value.to_owned());
        config.update(db).await.unwrap()
    }

    // _with_conn version for query
    async fn query_with_conn<C: ConnectionTrait>(
        db: &C,
        configuration: &str,
        name: Option<&str>,
        key: &str,
    ) -> Vec<Model> {
        config::Entity::find()
            .filter(config::Column::Configuration.eq(configuration))
            .filter(match name {
                Some(str) => config::Column::Name.eq(str),
                None => config::Column::Name.is_null(),
            })
            .filter(config::Column::Key.eq(key))
            .all(db)
            .await
            .unwrap()
    }

    // _with_conn version for get
    pub async fn get_with_conn<C: ConnectionTrait>(
        db: &C,
        configuration: &str,
        name: Option<&str>,
        key: &str,
    ) -> Option<String> {
        let values = Self::query_with_conn(db, configuration, name, key).await;
        values.first().map(|c| c.value.to_owned())
    }

    // _with_conn version for get_remote
    pub async fn get_remote_with_conn<C: ConnectionTrait>(db: &C, branch: &str) -> Option<String> {
        Config::get_with_conn(db, "branch", Some(branch), "remote").await
    }

    // _with_conn version for get_current_remote
    pub async fn get_current_remote_with_conn<C: ConnectionTrait>(
        db: &C,
    ) -> Result<Option<String>, ()> {
        match Head::current_with_conn(db).await {
            Head::Branch(name) => Ok(Config::get_remote_with_conn(db, &name).await),
            Head::Detached(_) => {
                eprintln!("fatal: HEAD is detached, cannot get remote");
                Err(())
            }
        }
    }

    // _with_conn version for get_remote_url
    pub async fn get_remote_url_with_conn<C: ConnectionTrait>(db: &C, remote: &str) -> String {
        match Config::get_with_conn(db, "remote", Some(remote), "url").await {
            Some(url) => url,
            None => panic!("fatal: No URL configured for remote '{remote}'."),
        }
    }

    // _with_conn version for get_current_remote_url
    pub async fn get_current_remote_url_with_conn<C: ConnectionTrait>(db: &C) -> Option<String> {
        match Config::get_current_remote_with_conn(db).await.unwrap() {
            Some(remote) => Some(Config::get_remote_url_with_conn(db, &remote).await),
            None => None,
        }
    }

    // _with_conn version for get_all
    pub async fn get_all_with_conn<C: ConnectionTrait>(
        db: &C,
        configuration: &str,
        name: Option<&str>,
        key: &str,
    ) -> Vec<String> {
        Self::query_with_conn(db, configuration, name, key)
            .await
            .iter()
            .map(|c| c.value.to_owned())
            .collect()
    }

    // _with_conn version for list_all
    pub async fn list_all_with_conn<C: ConnectionTrait>(db: &C) -> Vec<(String, String)> {
        config::Entity::find()
            .all(db)
            .await
            .unwrap()
            .iter()
            .map(|m| {
                (
                    match &m.name {
                        Some(n) => m.configuration.to_owned() + "." + n + "." + &m.key,
                        None => m.configuration.to_owned() + "." + &m.key,
                    },
                    m.value.to_owned(),
                )
            })
            .collect()
    }

    // _with_conn version for remove_config
    pub async fn remove_config_with_conn<C: ConnectionTrait>(
        db: &C,
        configuration: &str,
        name: Option<&str>,
        key: &str,
        valuepattern: Option<&str>,
        delete_all: bool,
    ) {
        let entries: Vec<Model> = Self::query_with_conn(db, configuration, name, key).await;
        for e in entries {
            let _res = match valuepattern {
                Some(vp) => {
                    if e.value.contains(vp) {
                        e.delete(db).await
                    } else {
                        continue;
                    }
                }
                None => e.delete(db).await,
            };
            if !delete_all {
                break;
            }
        }
    }

    // _with_conn version for remove_remote
    pub async fn remove_remote_with_conn<C: ConnectionTrait>(
        db: &C,
        name: &str,
    ) -> Result<(), String> {
        let remote = config::Entity::find()
            .filter(config::Column::Configuration.eq("remote"))
            .filter(config::Column::Name.eq(name))
            .all(db)
            .await
            .unwrap();
        if remote.is_empty() {
            return Err(format!("fatal: No such remote: {name}"));
        }
        for r in remote {
            let r: ActiveModel = r.into();
            r.delete(db).await.unwrap();
        }
        Ok(())
    }

    // _with_conn version for all_remote_configs
    pub async fn all_remote_configs_with_conn<C: ConnectionTrait>(db: &C) -> Vec<RemoteConfig> {
        let remotes = config::Entity::find()
            .filter(config::Column::Configuration.eq("remote"))
            .all(db)
            .await
            .unwrap();
        let remote_names = remotes
            .iter()
            .map(|remote| remote.name.as_ref().unwrap().clone())
            .collect::<HashSet<String>>();

        remote_names
            .iter()
            .map(|name| {
                let url = remotes
                    .iter()
                    .find(|remote| remote.name.as_ref().unwrap() == name)
                    .unwrap()
                    .value
                    .to_owned();
                RemoteConfig {
                    name: name.to_owned(),
                    url,
                }
            })
            .collect()
    }

    // _with_conn version for remote_config
    pub async fn remote_config_with_conn<C: ConnectionTrait>(
        db: &C,
        name: &str,
    ) -> Option<RemoteConfig> {
        let remote = config::Entity::find()
            .filter(config::Column::Configuration.eq("remote"))
            .filter(config::Column::Name.eq(name))
            .one(db)
            .await
            .unwrap();
        remote.map(|r| RemoteConfig {
            name: r.name.unwrap(),
            url: r.value,
        })
    }

    // _with_conn version for branch_config
    pub async fn branch_config_with_conn<C: ConnectionTrait>(
        db: &C,
        name: &str,
    ) -> Option<BranchConfig> {
        let config_entries = config::Entity::find()
            .filter(config::Column::Configuration.eq("branch"))
            .filter(config::Column::Name.eq(name))
            .all(db)
            .await
            .unwrap();
        if config_entries.is_empty() {
            None
        } else {
            assert_eq!(config_entries.len(), 2);
            // if branch_config[0].key == "merge" {
            //     Some(BranchConfig {
            //         name: name.to_owned(),
            //         merge: branch_config[0].value.clone(),
            //         remote: branch_config[1].value.clone(),
            //     })
            // } else {
            //     Some(BranchConfig {
            //         name: name.to_owned(),
            //         merge: branch_config[1].value.clone(),
            //         remote: branch_config[0].value.clone(),
            //     })
            // }
            let mut branch_config = BranchConfig {
                name: name.to_owned(),
                merge: config_entries[0].value.clone(),
                remote: config_entries[1].value.clone(),
            };
            if config_entries[0].key == "remote" {
                swap(&mut branch_config.merge, &mut branch_config.remote);
            }
            branch_config.merge = branch_config.merge[11..].into(); // cut refs/heads/

            Some(branch_config)
        }
    }

    pub async fn insert(configuration: &str, name: Option<&str>, key: &str, value: &str) {
        let db = get_db_conn_instance().await;
        Self::insert_with_conn(db, configuration, name, key, value).await;
    }

    // Update one configuration entry in database using given configuration, name, key and value
    pub async fn update(configuration: &str, name: Option<&str>, key: &str, value: &str) -> Model {
        let db = get_db_conn_instance().await;
        Self::update_with_conn(db, configuration, name, key, value).await
    }

    /// Get one configuration value
    pub async fn get(configuration: &str, name: Option<&str>, key: &str) -> Option<String> {
        let db = get_db_conn_instance().await;
        Self::get_with_conn(db, configuration, name, key).await
    }

    /// Get remote repo name by branch name
    /// - You may need to `[branch::set-upstream]` if return `None`
    pub async fn get_remote(branch: &str) -> Option<String> {
        let db = get_db_conn_instance().await;
        Self::get_remote_with_conn(db, branch).await
    }

    /// Get remote repo name of current branch
    /// - `Error` if `HEAD` is detached
    pub async fn get_current_remote() -> Result<Option<String>, ()> {
        let db = get_db_conn_instance().await;
        Self::get_current_remote_with_conn(db).await
    }

    pub async fn get_remote_url(remote: &str) -> String {
        let db = get_db_conn_instance().await;
        Self::get_remote_url_with_conn(db, remote).await
    }

    /// return `None` if no remote is set
    pub async fn get_current_remote_url() -> Option<String> {
        let db = get_db_conn_instance().await;
        Self::get_current_remote_url_with_conn(db).await
    }

    /// Get all configuration values
    /// - e.g. remote.origin.url can be multiple
    pub async fn get_all(configuration: &str, name: Option<&str>, key: &str) -> Vec<String> {
        let db = get_db_conn_instance().await;
        Self::get_all_with_conn(db, configuration, name, key).await
    }

    /// Get literally all the entries in database without any filtering
    pub async fn list_all() -> Vec<(String, String)> {
        let db = get_db_conn_instance().await;
        Self::list_all_with_conn(db).await
    }

    /// Delete one or all configuration using given key and value pattern
    pub async fn remove_config(
        configuration: &str,
        name: Option<&str>,
        key: &str,
        valuepattern: Option<&str>,
        delete_all: bool,
    ) {
        let db = get_db_conn_instance().await;
        Self::remove_config_with_conn(db, configuration, name, key, valuepattern, delete_all).await;
    }

    /// Delete all the configuration entries using given configuration field (--remove-section)
    // pub async fn remove_by_section(configuration: &str) {
    //     unimplemented!();
    // }
    pub async fn remove_remote(name: &str) -> Result<(), String> {
        let db = get_db_conn_instance().await;
        Self::remove_remote_with_conn(db, name).await
    }

    pub async fn all_remote_configs() -> Vec<RemoteConfig> {
        let db = get_db_conn_instance().await;
        Self::all_remote_configs_with_conn(db).await
    }

    pub async fn remote_config(name: &str) -> Option<RemoteConfig> {
        let db = get_db_conn_instance().await;
        Self::remote_config_with_conn(db, name).await
    }

    pub async fn branch_config(name: &str) -> Option<BranchConfig> {
        let db = get_db_conn_instance().await;
        Self::branch_config_with_conn(db, name).await
    }
}
