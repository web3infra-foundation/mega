use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use dagrs::{Action, Content, EnvVar, InChannels, OutChannels, Output};
use reqwest::{Client, Error};
use serde_json::json;

use crate::utils::CodeItem;
use crate::{PROCESS_ITEMS_NODE, QDRANT_NODE};
pub struct VectClient {
    url: String,
    client: Client,
}

impl VectClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn vectorize(&self, content: &str) -> Result<Vec<f64>, Error> {
        let response = self
            .client
            .post(&self.url)
            .json(&json!({
                "model": "bge-m3",
                "prompt": content
            }))
            .send()
            .await?;

        let body: HashMap<String, Vec<f64>> = response.json().await?;
        Ok(body["embedding"].clone())
    }
}

#[async_trait]
impl Action for VectClient {
    async fn run(
        &self,
        in_channels: &mut InChannels,
        out_channels: &mut OutChannels,
        env: Arc<EnvVar>,
    ) -> Output {
        log::info!("VectClient is running");
        let node_id = env.get_ref(PROCESS_ITEMS_NODE).unwrap();
        let out_node_id = env.get_ref(QDRANT_NODE).unwrap();
        println!("vect_client_id: {:?}", node_id);

        while let Ok(content) = in_channels.recv_from(node_id).await {
            log::info!("Received items to vectorize");
            let items: &Vec<CodeItem> = content.get().unwrap();
            for item in items {
                match self.vectorize(&item.content).await {
                    Ok(vector) => {
                        let mut item = item.clone();
                        item.vector = vector;
                        out_channels.broadcast(Content::new(item)).await;
                        log::info!("VectClient has processed an item");
                    }
                    Err(e) => {
                        log::error!("Failed to vectorize content: {}", e);
                        continue;
                    }
                }
            }
        }

        log::info!("VectClient finished processing all items");
        out_channels.close(out_node_id);
        Output::empty()
    }
}

#[cfg(test)]
mod test_vectorization {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_vectorize() {
        let client = VectClient::new("http://localhost:11434/api/embeddings");
        let content = "testcontent";
        let vector = client.vectorize(content).await.unwrap();
        println!("{:?}", vector);
    }
}
