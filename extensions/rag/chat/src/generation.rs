use async_trait::async_trait;
use dagrs::{Action, EnvVar, InChannels, OutChannels, Output};
use reqwest::{Client, Error};
use serde_json::{json, Value};
use std::{fs::File, sync::Arc};

use crate::SEARCH_NODE;

pub struct GenerationNode {
    url: String,
    client: Client,
}

impl GenerationNode {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn generate(&self, context: &str) -> Result<String, Error> {
        let response = self
            .client
            .post(&self.url)
            .json(&json!({
                "model": "deepseek-r1",
                "messages": [
                    {
                        "role": "user",
                        "content": context
                    }
                ],
                "stream": false
            }))
            .send()
            .await?;

        // Parse the returned JSON
        let body: Value = response.json().await?;

        // Write JSON to a file
        let file_path = "output.json"; // Replace with the path where you want to save the file
        let file = File::create(file_path).unwrap();

        serde_json::to_writer(file, &body).unwrap(); // Write JSON data to a file

        // Extract the generated text from the returned JSON
        let message = body["message"]["content"].as_str().unwrap();
        println!("{}", message);
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
            let _ = self.generate(context).await;
            //self.generate(context).await;
        }

        log::info!("GenerationNode finished processing");
        Output::empty()
    }
}
