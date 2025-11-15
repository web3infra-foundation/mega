mod kafka_handler;
use std::{env, fs::{self, File}, time::{SystemTime, UNIX_EPOCH}};
#[allow(clippy::single_component_path_imports)]
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio_postgres::{NoTls, Row};
use tracing_subscriber::{  EnvFilter};
use crate::kafka_handler::KafkaHandler;
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CveId{
    id:String,
    url:String,
}
#[allow(clippy::io_other_error)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let log_path = format!("log/log_{}.ans", timestamp);
        let parent_dir = std::path::Path::new(&log_path)
            .parent() 
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Invalid log path")).unwrap();

        fs::create_dir_all(parent_dir)?;

        let file = File::create(&log_path).expect("Unable to create log file");

        tracing_subscriber::fmt()
            .with_writer(file)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
        tracing::info!("Starting with log file: {}", log_path);
        let url = "https://rustsec.org/advisories/".to_string();
        let resp = reqwest::get(&url).await?;
        if !resp.status().is_success() {
            tracing::error!("HTTP request error: {}", resp.status());
            return Ok(());
        }
        let body = resp.text().await?;
        let document = Html::parse_document(&body);
        let subtitle_selector = Selector::parse("ul>li>h3>a").unwrap();
        let mut real_ids:Vec<String> = Vec::new();
        for element in document.select(&subtitle_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text.trim_start_matches(|c: char| c.is_whitespace())
            .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            let ids: Vec<&str>= real_res.lines()
            .map(|line| {
                line.split(':')
                    .next()
                    .unwrap_or("")
                    .trim()
            })
            .filter(|id| !id.is_empty())
            .collect();
            for id in ids {
                real_ids.push(id.to_string());
            }
        }
        tracing::info!("len:{}",real_ids.len());
        let connection_string = format!(
            "host={} port={} user={} password={} dbname=cratespro",
            env::var("POSTGRES_HOST_IP").unwrap(),
            env::var("POSTGRES_HOST_PORT").unwrap(),
            env::var("POSTGRES_USER").unwrap(),
            env::var("POSTGRES_PASSWORD").unwrap()
        );
        let (client, connection) =
        tokio_postgres::connect(&connection_string, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("Database connection failed: {}", e);
            }
        });
        client.execute("CREATE TABLE IF NOT EXISTS rustsec_ids (id TEXT PRIMARY KEY);", &[]).await?;
        for id in real_ids{
            let row: Row = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM rustsec_ids WHERE id = $1);",
            &[&id]
            ).await?;
            let exists: bool = row.get(0);
            tracing::info!("{}",exists);
            if !exists {
            // 
                client.execute(
                    "INSERT INTO rustsec_ids (id) VALUES ($1);",
                    &[&id]
                ).await?;
                tracing::info!("Insert new ID: {}", id.clone());
                let sec_url = "https://rustsec.org/advisories/".to_string()+&id+".html";
                let cve_info = CveId{ id, url:sec_url};
                let kafka_broker = env::var("KAFKA_BROKER").unwrap();
                let topic_test = env::var("KAFKA_CVEID_TOPIC").unwrap();
                let sender_handler = KafkaHandler::new_producer(&kafka_broker).expect("Invalid import kafka handler");
                sender_handler.send_message(&topic_test, "", &serde_json::to_string(&cve_info).unwrap()).await;
            } else {
                tracing::info!("ID exist,skip: {}", id);
            }
        }
        Ok(())
}