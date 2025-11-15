mod cve_info;
mod model;
mod kafka_handler;
use std::{env, fs};
use std::fs::{ File};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use data_transporter::db::DBHandler;
use rdkafka::{ Message};
use tokio_postgres::{ NoTls};
use tracing_subscriber::EnvFilter;
use crate::cve_info::get_cve_info;
use crate::cve_info::analyze_cve;
use crate::kafka_handler::KafkaHandler;
use crate::model::CveId;
use data_transporter::data_reader::{ DataReader};
#[derive(Debug, Clone)]
struct RustsecInfo {
    id: String,
    subtitle: String,
    reported: String,
    issued: String,
    package: String,
    ttype: String,
    keywords: String,
    aliases: String,
    reference: String,
    patched: String,
    unaffected: String,
    description: String,
    affected:String,
}
#[allow(clippy::io_other_error)]
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let log_path = format!("log/log_{}.ans", timestamp);
    let parent_dir = std::path::Path::new(&log_path)
            .parent() 
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Invalid log path")).unwrap();

    fs::create_dir_all(parent_dir).unwrap();
    let file = File::create(&log_path).expect("Unable to create log file");
    // 设置日志记录器
    tracing_subscriber::fmt()
        .with_writer(file)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting with log file: {}", log_path);

    let connection_string = format!(
            "host={} port={} user={} password={} dbname=cratespro",
            env::var("POSTGRES_HOST_IP").unwrap(),
            env::var("POSTGRES_HOST_PORT").unwrap(),
            env::var("POSTGRES_USER").unwrap(),
            env::var("POSTGRES_PASSWORD").unwrap()
        );
    let (client, connection) = tokio_postgres::connect(
        &connection_string,
        NoTls,
    )
    .await.unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Database connection error: {}", e);
        }
    });

    let dbhandler = DBHandler{client};
    let tugraph_bolt_url = env::var("TUGRAPH_BOLT_URL").unwrap();
            let tugraph_user_name = env::var("TUGRAPH_USER_NAME").unwrap();
            let tugraph_user_password = env::var("TUGRAPH_USER_PASSWORD").unwrap();
            let tugraph_cratespro_db = env::var("TUGRAPH_CRATESPRO_DB").unwrap();

    let datareader = DataReader::new(&tugraph_bolt_url,&tugraph_user_name,&tugraph_user_password,&tugraph_cratespro_db).await.unwrap();

    let kafka_broker = env::var("KAFKA_BROKER").unwrap();
    let consumer_group_id = env::var("KAFKA_CONSUMER_GROUP_ID").unwrap();

    let import_handler = KafkaHandler::new_consumer(
        &kafka_broker,
        &consumer_group_id,
        &env::var("KAFKA_CVEID_TOPIC").unwrap(),
    )
    .expect("Invalid import kafka handler");
    loop{
        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Ok(message) = import_handler.consume_once().await{
            let model = match serde_json::from_slice::<model::CveId>(
                message.payload().unwrap(),
            ){
                Ok(m) => Some(m.clone()),
                Err(e) => {
                    tracing::info!("Error while deserializing message payload: {:?}", e);
                    None
                }
            };
            if model.is_none(){
                continue;
            }
            tracing::info!(
                "Received a message, key: '{:?}', payload: '{:?}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                message.key(),
                model,
                message.topic(),
                message.partition(),
                message.offset(),
                message.timestamp()
            );
            let url = model.clone().unwrap().url;
            let id = model.clone().unwrap().id;
            let res_info = get_cve_info(id.clone(), url.clone()).await;
            if res_info.is_ok(){
                tracing::info!("Start inserting/updating id {} to pg",id.clone());
                let one_res = res_info.unwrap();
                dbhandler.client.execute(
                    "INSERT INTO rustsec_info(
                        id, subtitle, reported, issued, package, type, 
                        keywords, aliases, reference, patched, unaffected, description,affected
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,$13)
                    ON CONFLICT (id) DO UPDATE SET
                        subtitle = EXCLUDED.subtitle,
                        reported = EXCLUDED.reported,
                        issued = EXCLUDED.issued,
                        package = EXCLUDED.package,
                        type = EXCLUDED.type,
                        keywords = EXCLUDED.keywords,
                        aliases = EXCLUDED.aliases,
                        reference = EXCLUDED.reference,
                        patched = EXCLUDED.patched,
                        unaffected = EXCLUDED.unaffected,
                        description = EXCLUDED.description,
                        affected = EXCLUDED.affected;",
                    &[
                        &one_res.id,
                        &one_res.subtitle,
                        &one_res.reported,
                        &one_res.issued,
                        &one_res.package,
                        &one_res.ttype,
                        &one_res.keywords,
                        &one_res.aliases,
                        &one_res.reference,
                        &one_res.patched,
                        &one_res.unaffected,
                        &one_res.description,
                        &one_res.affected,
                    ],
                ).await.unwrap();
            }
            tracing::info!("Finish inserting/updating id {} to pg",id.clone());
            analyze_cve(&datareader, &dbhandler, id.clone()).await.unwrap();
            tracing::info!("start sending id {} to rag kafka topic",id.clone());
            let sec_url = "https://rustsec.org/advisories/".to_string()+&id+".html";
            let cve_info = CveId{ id, url:sec_url};
            let kafka_broker = env::var("KAFKA_BROKER").unwrap();
            let topic_rag = env::var("KAFKA_RAG_TOPIC").unwrap();
            let sender_handler = KafkaHandler::new_producer(&kafka_broker).expect("Invalid import kafka handler");
            sender_handler.send_message(&topic_rag, "", &serde_json::to_string(&cve_info).unwrap()).await;
        }
        else{
            continue;
        }
    }
    
}
