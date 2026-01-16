use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use dagrs::{Action, EnvVar, InChannels, OutChannels, Output};
use reqwest::Client;
use serde::de::Error;
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::oneshot;

use crate::{RAG_OUTPUT, SEARCH_NODE};

#[derive(Debug, Error)]
pub enum GenError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct GenerationNode {
    url: String,
    client: Client,
    result_tx: Mutex<Option<oneshot::Sender<String>>>,
}

impl GenerationNode {
    pub fn new(url: &str, result_tx: Option<oneshot::Sender<String>>) -> Self {
        Self {
            url: url.to_string(),
            client: Client::new(),
            result_tx: Mutex::new(result_tx),
        }
    }

    pub async fn generate(&self, context: &str) -> Result<String, GenError> {
        let response = self
            .client
            .post(&self.url)
            .json(&json!({
                "model": "deepseek-r1:7b",
                "messages": [
                    {
                        "role": "user",
                        "content": context
                    }
                ],
                "stream": false
            }))
            .send()
            .await
            .map_err(GenError::Http)?;

        // Parse the returned JSON
        let body: Value = response.json().await.map_err(GenError::Http)?;

        // Write JSON to a file
        let file_path = RAG_OUTPUT; // Replace with the path where you want to save the file

        let file = match File::create(file_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create file {file_path}: {e}");
                return Err(GenError::Io(e));
            }
        };
        if let Err(e) = serde_json::to_writer(file, &body) {
            eprintln!("Failed to write JSON data to file {file_path}: {e}");
            return Err(GenError::Json(e));
        }

        // Extract the generated text from the returned JSON
        let message = match body
            .get("message")
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
        {
            Some(content) => content,
            None => {
                eprintln!("Failed to extract 'content' from JSON response: {body:?}");
                return Err(GenError::Json(serde_json::Error::custom(
                    "Missing or invalid 'content' in JSON response",
                )));
            }
        };
        Ok(message.to_string())
    }
}

#[async_trait]
impl Action for GenerationNode {
    async fn run(
        &self,
        in_channels: &mut InChannels,
        _out_channels: &mut OutChannels,
        env: Arc<EnvVar>,
    ) -> Output {
        log::info!("GenerationNode is running");

        while let Ok(content) = in_channels
            .recv_from(env.get_ref(SEARCH_NODE).unwrap())
            .await
        {
            log::info!("Received content for generation");
            let context: &String = content.get().unwrap();
            let message = self.generate(context).await;
            match message {
                Ok(msg) => {
                    println!("{msg}");
                    if let Some(tx) = self.result_tx.lock().unwrap().take() {
                        let _ = tx.send(msg);
                    }
                }
                Err(e) => eprintln!("Generation error: {e}"),
            }
        }

        log::info!("GenerationNode finished processing");
        Output::empty()
    }
}
