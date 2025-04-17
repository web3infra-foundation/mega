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

        // 解析返回的 JSON
        let body: Value = response.json().await?;

        // 将 JSON 写入文件
        let file_path = "output.json"; // 替换为你想保存文件的路径
        let file = File::create(file_path).unwrap();

        serde_json::to_writer(file, &body).unwrap(); // 将 JSON 数据写入文件

        // 从返回的 JSON 中提取生成的文本
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
            self.generate(context).await;
        }

        log::info!("GenerationNode finished processing");
        Output::empty()
    }
}
