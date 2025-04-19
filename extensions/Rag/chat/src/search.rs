use async_trait::async_trait;
use dagrs::{Action, Content, EnvVar, InChannels, OutChannels, Output};
use qdrant_client::qdrant::SearchPointsBuilder;
use qdrant_client::Qdrant;
use std::sync::Arc;
use vectorization::VectClient;

use crate::{vectorization, GENERATION_NODE};

pub struct SearchNode {
    client: Qdrant,
    vect_client: VectClient,
    collection_name: String,
}

impl SearchNode {
    pub fn new(vect_url: &str, qdrant_url: &str, collection_name: &str) -> Self {
        let client = Qdrant::from_url(qdrant_url).build().unwrap();
        let vect_client = VectClient::new(vect_url);
        Self {
            client,
            vect_client,
            collection_name: collection_name.to_string(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Option<(String, String)>, Box<dyn std::error::Error>> {
        // 向量化查询
        let query_vector = self.vect_client.vectorize(query).await?;

        // 在Qdrant中搜索，只返回最相似的一个结果
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
                .with_payload(true), // 关键：必须显式请求payload
            )
            .await?;
        println!("search_result: {:?}", search_result);
        // 转换结果为 content 和 item_type
        if let Some(point) = search_result.result.into_iter().next() {
            let payload = point.payload;
            let content = payload
                .get("content")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            let item_type = payload.get("type").unwrap().as_str().unwrap().to_string();
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
        // 从用户输入获取查询
        let mut input = String::new();
        println!("\n请输入查询内容:");
        std::io::stdin().read_line(&mut input).unwrap();
        println!("input: {}", input);
        let out_node_id = env.get_ref(GENERATION_NODE).unwrap();
        // 执行搜索
        let result = match self.search(input.trim()).await {
            Ok(Some((content, item_type))) => {
                println!("\n搜索结果:");
                println!("\n类型: {}", item_type);
                println!("内容:\n{}", content);
                format!(
                    "查询: {}\n类型: {}\n内容: {}",
                    input.trim(),
                    item_type,
                    content
                )
            }
            Ok(None) => {
                println!("\n未找到相关结果");
                input.trim().to_string()
            }
            Err(e) => {
                eprintln!("搜索时出错: {}", e);
                input.trim().to_string()
            }
        };

        out_channels.broadcast(Content::new(result)).await;
        out_channels.close(out_node_id);

        Output::empty()
    }
}
