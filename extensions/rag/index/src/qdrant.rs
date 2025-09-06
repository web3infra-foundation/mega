use async_trait::async_trait;
use dagrs::{Action, Content, EnvVar, InChannels, OutChannels, Output};
use qdrant_client::Qdrant;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::utils::CodeItem;

use crate::VECT_CLIENT_NODE;

pub struct QdrantNode {
    client: Qdrant,
    collection_name: String,
    id_counter: Arc<AtomicU64>,
}

impl QdrantNode {
    pub fn new(url: &str, collection_name: &str, id_counter: Arc<AtomicU64>) -> Self {
        let client = Qdrant::from_url(url).build().unwrap();
        Self {
            client,
            collection_name: collection_name.to_string(),
            id_counter,
        }
    }

    async fn ensure_collection(&self) {
        match self.client.collection_exists(&self.collection_name).await {
            Ok(true) => {
                log::info!("Collection '{}' already exists", self.collection_name);
            }
            Ok(false) => {
                log::info!("Collection '{}' does not exist, creating...", self.collection_name);
                if let Err(e) = self
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
                {
                    log::error!("Failed to create collection '{}': {e}", self.collection_name);
                }
            }
            Err(e) => {
                log::error!("Failed to check collection existence: {e}");
            }
        }
    }
    

    // async fn ensure_collection(&self) {
    //     if self
    //         .client
    //         .create_collection(
    //             qdrant_client::qdrant::CreateCollectionBuilder::new(&self.collection_name)
    //                 .vectors_config(qdrant_client::qdrant::VectorParamsBuilder::new(
    //                     1024,
    //                     qdrant_client::qdrant::Distance::Cosine,
    //                 ))
    //                 .quantization_config(
    //                     qdrant_client::qdrant::ScalarQuantizationBuilder::default(),
    //                 ),
    //         )
    //         .await
    //         .is_err()
    //     {
    //         println!("Collection already exists or error occurred");
    //     }
    // }
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

        let mut processed_count = 0;

        log::info!("Waiting for code items to process...");
        while let Ok(content) = in_channels.recv_from(node_id).await {
            log::info!("Received item to store in Qdrant");
            let item: &CodeItem = content.get().unwrap();

            // Atomically fetch and increment the ID from the shared counter
            let new_id = self.id_counter.fetch_add(1, Ordering::SeqCst);
            let point = item.to_qdrant_point(new_id);

            // Store to Qdrant
            if let Err(e) = self
                .client
                .upsert_points(qdrant_client::qdrant::UpsertPointsBuilder::new(
                    &self.collection_name,
                    vec![point],
                ))
                .await
            {
                log::error!("Error storing item in Qdrant: {e}");
            } else {
                processed_count += 1;
                if processed_count % 100 == 0 {
                    log::info!("Processed {processed_count} items");
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
