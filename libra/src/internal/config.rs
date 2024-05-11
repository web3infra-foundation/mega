use std::collections::HashSet;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::ActiveValue::Set;

use crate::internal::db::get_db_conn_instance;
use crate::internal::model::config;

pub struct Config;

pub struct RemoteConfig {
    pub name: String,
    pub url: String,
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

    pub async fn remote_configs() -> Vec<RemoteConfig> {
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

        // for remote_name in remote_names {
        //     let url = remotes
        //         .iter()
        //         .find(|remote| remote.name.as_ref().unwrap() == &remote_name)
        //         .unwrap()
        //         .value.to_owned();
        //     println!("{} {}", remote_name, url);
        // }
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
}
