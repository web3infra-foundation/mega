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
        .expect("Time went backwards")
        .as_secs();

    let log_path = format!("log/log_{}.ans", timestamp);
    let parent_dir = std::path::Path::new(&log_path)
            .parent() 
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Invalid log path")).expect("Failed to get parent directory");

    fs::create_dir_all(parent_dir).expect("Failed to create log directory");
    let file = File::create(&log_path).expect("Unable to create log file");
    // 设置日志记录器
    tracing_subscriber::fmt()
        .with_writer(file)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting with log file: {}", log_path);

    let connection_string = format!(
            "host={} port={} user={} password={} dbname=cratespro",
            env::var("POSTGRES_HOST_IP").expect("Must get POSTGRES_HOST_IP"),
            env::var("POSTGRES_HOST_PORT").expect("Must get POSTGRES_HOST_PORT"),
            env::var("POSTGRES_USER").expect("Must get POSTGRES_USER"),
            env::var("POSTGRES_PASSWORD").expect("Must get POSTGRES_PASSWORD")
        );
    let (client, connection) = match tokio_postgres::connect(
        &connection_string,
        NoTls,
    )
    .await {
        Ok((client, connection)) => (client, connection),
        Err(e) => {
            tracing::error!("Failed to connect to PostgreSQL: {}", e);
            std::process::exit(1);
        }
    };
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Database connection error: {}", e);
        }
    });

    let dbhandler = DBHandler{client};
    let tugraph_bolt_url = env::var("TUGRAPH_BOLT_URL").expect("must get TUGRAPH_BOLT_URL");
    let tugraph_user_name = env::var("TUGRAPH_USER_NAME").expect("must get TUGRAPH_USER_NAME");
    let tugraph_user_password = env::var("TUGRAPH_USER_PASSWORD").expect("must get TUGRAPH_USER_PASSWORD");
    let tugraph_cratespro_db = env::var("TUGRAPH_CRATESPRO_DB").expect("must get TUGRAPH_CRATESPRO_DB");

    let datareader = DataReader::new(&tugraph_bolt_url,&tugraph_user_name,&tugraph_user_password,&tugraph_cratespro_db).await.expect("Failed to create datareader");

    let kafka_broker = env::var("KAFKA_BROKER").expect("Must get KAFKA_BROKER");
    let consumer_group_id = env::var("KAFKA_CONSUMER_GROUP_ID").expect("Must get KAFKA_CONSUMER_GROUP_ID");

    let import_handler = KafkaHandler::new_consumer(
        &kafka_broker,
        &consumer_group_id,
        &env::var("KAFKA_CVEID_TOPIC").expect("Must get KAFKA_CVEID_TOPIC"),
    )
    .expect("Invalid import kafka handler");
    loop{
        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Ok(message) = import_handler.consume_once().await{
            let model = message.payload()
                .and_then(|p| serde_json::from_slice::<model::CveId>(p).ok())
                ;
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
            let url = model.clone().expect("failed to get url").url;
            let id = model.clone().expect("failed to get id").id;
            let res_info = get_cve_info(id.clone(), url.clone()).await;
            if res_info.is_ok(){
                tracing::info!("Start inserting/updating id {} to pg",id.clone());
                let one_res = res_info.expect("failed to get res_info");
                
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
                ).await.map_err(|e| {
                    tracing::error!("Failed to insert/update id {} in pg: {:?}", id.clone(), e);
                })
                .ok();
            }
            tracing::info!("Finish inserting/updating id {} to pg",id.clone());
            analyze_cve(&datareader, &dbhandler, id.clone()).await.expect("Failed to analyze cve");
            tracing::info!("start sending id {} to rag kafka topic",id.clone());
            let sec_url = "https://rustsec.org/advisories/".to_string()+&id+".html";
            let cve_info = CveId{ id, url:sec_url};
            let kafka_broker = env::var("KAFKA_BROKER").expect("Must get KAFKA_BROKER");
            let topic_rag = env::var("KAFKA_RAG_TOPIC").expect("Must get KAFKA_RAG_TOPIC");
            let sender_handler = KafkaHandler::new_producer(&kafka_broker).expect("Invalid import kafka handler");
            let cve_info_json = match serde_json::to_string(&cve_info) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize cve_info: {:?}", e);
                    continue;
                }
            };
            sender_handler.send_message(&topic_rag, "", &cve_info_json).await;
        }
        else{
            continue;
        }
    }
    
}
