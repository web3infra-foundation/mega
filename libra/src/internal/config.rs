use std::collections::HashSet;
use std::mem::swap;

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

use crate::internal::db::get_db_conn_instance;
use crate::internal::head::Head;
use crate::internal::model::config;
use crate::internal::model::config::Model;

use super::model::config::ActiveModel;

pub struct Config;

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

impl Config {
    // todo accept a db connect or a transaction from outside
    pub async fn insert(configuration: &str, name: Option<&str>, key: &str, value: &str) {
        let db = get_db_conn_instance().await;
        let config = ActiveModel {
            configuration: Set(configuration.to_owned()),
            name: Set(name.map(|s| s.to_owned())),
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
            ..Default::default()
        };
        config.save(db).await.unwrap();
    }

    // Update one configuration entry in database using given configuration, name, key and value
    pub async fn update(configuration: &str, name: Option<&str>, key: &str, value: &str) -> Model {
        let db = get_db_conn_instance().await;
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

    async fn query(configuration: &str, name: Option<&str>, key: &str) -> Vec<Model> {
        let db = get_db_conn_instance().await;
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

    /// Get one configuration value
    pub async fn get(configuration: &str, name: Option<&str>, key: &str) -> Option<String> {
        let values = Self::query(configuration, name, key).await;
        values.first().map(|c| c.value.to_owned())
    }

    /// Get remote repo name by branch name
    /// - You may need to `[branch::set-upstream]` if return `None`
    pub async fn get_remote(branch: &str) -> Option<String> {
        // e.g. [branch "master"].remote = origin
        Config::get("branch", Some(branch), "remote").await
    }

    /// Get remote repo name of current branch
    /// - `Error` if `HEAD` is detached
    pub async fn get_current_remote() -> Result<Option<String>, ()> {
        match Head::current().await {
            Head::Branch(name) => Ok(Config::get_remote(&name).await),
            Head::Detached(_) => {
                eprintln!("fatal: HEAD is detached, cannot get remote");
                Err(())
            }
        }
    }

    pub async fn get_remote_url(remote: &str) -> String {
        match Config::get("remote", Some(remote), "url").await {
            Some(url) => url,
            None => panic!("fatal: No URL configured for remote '{}'.", remote),
        }
    }

    /// return `None` if no remote is set
    pub async fn get_current_remote_url() -> Option<String> {
        match Config::get_current_remote().await.unwrap() {
            Some(remote) => Some(Config::get_remote_url(&remote).await),
            None => None,
        }
    }

    /// Get all configuration values
    /// - e.g. remote.origin.url can be multiple
    pub async fn get_all(configuration: &str, name: Option<&str>, key: &str) -> Vec<String> {
        Self::query(configuration, name, key)
            .await
            .iter()
            .map(|c| c.value.to_owned())
            .collect()
    }

    /// Get literally all the entries in database without any filtering
    pub async fn list_all() -> Vec<(String, String)> {
        let db = get_db_conn_instance().await;
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

    /// Delete one or all configuration using given key and value pattern
    pub async fn remove_config(
        configuration: &str,
        name: Option<&str>,
        key: &str,
        valuepattern: Option<&str>,
        delete_all: bool,
    ) {
        let db = get_db_conn_instance().await;
        let entries: Vec<Model> = Self::query(configuration, name, key).await;
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

    /// Delete all the configuration entries using given configuration field (--remove-section)
    // pub async fn remove_by_section(configuration: &str) {
    //     unimplemented!();
    // }
    pub async fn remove_remote(name: &str) -> Result<(), String> {
        let db = get_db_conn_instance().await;
        let remote = config::Entity::find()
            .filter(config::Column::Configuration.eq("remote"))
            .filter(config::Column::Name.eq(name))
            .all(db)
            .await
            .unwrap();
        if remote.is_empty() {
            return Err(format!("fatal: No such remote: {}", name));
        }
        for r in remote {
            let r: ActiveModel = r.into();
            r.delete(db).await.unwrap();
        }
        Ok(())
    }

    pub async fn all_remote_configs() -> Vec<RemoteConfig> {
        let db = get_db_conn_instance().await;
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

    pub async fn remote_config(name: &str) -> Option<RemoteConfig> {
        let db = get_db_conn_instance().await;
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

    pub async fn branch_config(name: &str) -> Option<BranchConfig> {
        let db = get_db_conn_instance().await;
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
}
