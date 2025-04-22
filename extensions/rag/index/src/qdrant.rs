use async_trait::async_trait;
use dagrs::{Action, Content, EnvVar, InChannels, OutChannels, Output};
use qdrant_client::Qdrant;
use std::sync::Arc;

use crate::utils::CodeItem;

use crate::VECT_CLIENT_NODE;

pub struct QdrantNode {
    client: Qdrant,
    collection_name: String,
}

impl QdrantNode {
    pub fn new(url: &str, collection_name: &str) -> Self {
        let client = Qdrant::from_url(url).build().unwrap();
        Self {
            client,
            collection_name: collection_name.to_string(),
        }
    }

    async fn ensure_collection(&self) {
        if self
            .client
            .create_collection(
                qdrant_client::qdrant::CreateCollectionBuilder::new(&self.collection_name)
                    .vectors_config(qdrant_client::qdrant::VectorParamsBuilder::new(
                        1024,
                        qdrant_client::qdrant::Distance::Cosine,
                    ))
                    .quantization_config(
                        qdrant_client::qdrant::ScalarQuantizationBuilder::default(),
                    ),
            )
            .await
            .is_err()
        {
            println!("Collection already exists or error occurred");
        }
    }
}

#[async_trait]
impl Action for QdrantNode {
    async fn run(
        &self,
        in_channels: &mut InChannels,
        out_channels: &mut OutChannels,
        env: Arc<EnvVar>,
    ) -> Output {
        self.ensure_collection().await;
        let node_id = env.get_ref(VECT_CLIENT_NODE).unwrap();
        println!("qdrant_id: {:?}", node_id);
        let mut id_counter = 0;
        let mut processed_count = 0;

        log::info!("Waiting for code items to process...");
        while let Ok(content) = in_channels.recv_from(node_id).await {
            log::info!("Received item to store in Qdrant");
            let item: &CodeItem = content.get().unwrap();

            // Use the to_qdrant_point method of CodeItem to create PointStruct
            let point = item.to_qdrant_point(id_counter);
            id_counter += 1;

            // Store to Qdrant
            if let Err(e) = self
                .client
                .upsert_points(qdrant_client::qdrant::UpsertPointsBuilder::new(
                    &self.collection_name,
                    vec![point],
                ))
                .await
            {
                log::error!("Error storing item in Qdrant: {}", e);
            } else {
                processed_count += 1;
                if processed_count % 100 == 0 {
                    log::info!("Processed {} items", processed_count);
                }
            }
            out_channels.broadcast(Content::new(())).await;
        }

        log::info!(
            "QdrantNode finished processing all items. Total processed: {}",
            processed_count
        );
        let collection_info = self
            .client
            .collection_info(&self.collection_name)
            .await
            .unwrap();
        log::info!("Collection info: {:?}", collection_info);
        Output::empty()
    }
}
