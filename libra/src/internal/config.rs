use std::collections::HashSet;
use std::mem::swap;

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use crate::internal::db::get_db_conn_instance;
use crate::internal::model::config;
use crate::internal::model::config::Model;

pub struct Config;

pub struct RemoteConfig {
    pub name: String,
    pub url: String,
}

pub struct BranchConfig {
    pub name: String,
    pub merge: String,
    pub remote: String,
}

impl Config {
    // todo accept a db connect or a transaction from outside
    pub async fn insert(configuration: &str, name: Option<&str>, key: &str, value: &str) {
        let db = get_db_conn_instance().await;
        let config = config::ActiveModel {
            configuration: Set(configuration.to_owned()),
            name: Set(name.map(|s| s.to_owned())),
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
            ..Default::default()
        };
        config.save(db).await.unwrap();
    }

    async fn query(configuration: &str, name: Option<&str>, key: &str) -> Vec<Model> {
        let db = get_db_conn_instance().await;
        config::Entity::find()
            .filter(config::Column::Configuration.eq(configuration))
            .filter(config::Column::Name.eq(name))
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

    /// Get all configuration values
    /// - e.g. remote.origin.url can be multiple
    pub async fn get_all(configuration: &str, name: Option<&str>, key: &str) -> Vec<String> {
        Self::query(configuration, name, key)
            .await
            .iter()
            .map(|c| c.value.to_owned())
            .collect()
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
            assert!(config_entries.len() == 2);
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
