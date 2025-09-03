use crate::{vectorization, GENERATION_NODE};
use async_trait::async_trait;
use dagrs::{Action, Content, EnvVar, InChannels, OutChannels, Output};
use log::debug;
use qdrant_client::qdrant::SearchPointsBuilder;
use qdrant_client::Qdrant;
use std::sync::Arc;
use vectorization::VectClient;

pub struct SearchNode {
    client: Qdrant,
    vect_client: VectClient,
    collection_name: String,
    prompt: String,
}

impl SearchNode {
    pub fn new(
        vect_url: &str,
        qdrant_url: &str,
        collection_name: &str,
        prompt: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Qdrant::from_url(qdrant_url).build()?;
        let vect_client = VectClient::new(vect_url);
        Ok(Self {
            client,
            vect_client,
            collection_name: collection_name.to_string(),
            prompt: prompt.to_string(),
        })
    }

    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Option<(String, String)>, Box<dyn std::error::Error>> {
        // Vectorized query
        let query_vector = self.vect_client.vectorize(query).await?;

        // Search in Qdrant and only return the most similar result
        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(
                    &self.collection_name,
                    query_vector
                        .clone()
                        .into_iter()
                        .map(|x| x as f32)
                        .collect::<Vec<f32>>(),
                    1,
                )
                .with_payload(true), // Key: Payload must be explicitly requested
            )
            .await?;
        debug!("search_result: {:?}", search_result);
        // Convert the result to content and item_type
        if let Some(point) = search_result.result.into_iter().next() {
            let payload = point.payload;
            let content = payload
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or("Missing or invalid 'content' in payload")?;
            let item_type = payload
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or("Missing or invalid 'type' in payload")?;
            Ok(Some((content, item_type)))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl Action for SearchNode {
    async fn run(
        &self,
        _in_channels: &mut InChannels,
        out_channels: &mut OutChannels,
        env: Arc<EnvVar>,
    ) -> Output {
        let input = self.prompt.clone();
        let out_node_id = env.get_ref(GENERATION_NODE).unwrap();
        // Execute search
        let result = match self.search(input.trim()).await {
            Ok(Some((content, item_type))) => {
                log::info!("原prompt: {:?}", input.trim());
                println!("\nSearch result:");
                println!("Type: {}", item_type);
                println!("Content:\n{}", content);
                format!(
                    "{}\nThe enhanced information after local RAG may be helpful, but it is not necessarily accurate:\n Related information type: {}\nRelated information Content: {}",
                    input.trim(),
                    item_type,
                    content
                )
            }
            Ok(None) => {
                println!("\nNo relevant results found");
                input.trim().to_string()
            }
            Err(e) => {
                eprintln!("Error during search: {}", e);
                input.trim().to_string()
            }
        };

        out_channels.broadcast(Content::new(result)).await;
        out_channels.close(out_node_id);

        Output::empty()
    }
}
