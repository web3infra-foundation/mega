//! According to the [Lingyiwanwu API Reference](https://platform.lingyiwanwu.com/docs/api-reference), the Lingyiwanwu API is identical to the OpenAI API.
//! Therefore, this is just a wrapper around the OpenAI API, with a different API base URL.

use crate::api::openai::OpenAIClient;
use crate::AskModel;

/// yi-large, yi-medium, yi-vision, yi-medium-200k, yi-spark, vi-larqe-raq, yi-large-turbo, yi-large-fc
pub enum LingyiwanwuModels {
    YiLarge,
    YiMedium,
    YiVision,
    YiMedium200k,
    YiSpark,
    ViLargeRaq,
    YiLargeTurbo,
    YiLargeFc,
}

impl crate::Model for LingyiwanwuModels {
    fn as_str(&self) -> &str {
        match self {
            LingyiwanwuModels::YiLarge => "yi-large",
            LingyiwanwuModels::YiMedium => "yi-medium",
            LingyiwanwuModels::YiVision => "yi-vision",
            LingyiwanwuModels::YiMedium200k => "yi-medium-200k",
            LingyiwanwuModels::YiSpark => "yi-spark",
            LingyiwanwuModels::ViLargeRaq => "vi-large-raq",
            LingyiwanwuModels::YiLargeTurbo => "yi-large-turbo",
            LingyiwanwuModels::YiLargeFc => "yi-large-fc",
        }
    }
}

const LING_YI_WAN_WU_API_BASE: &str = "https://api.lingyiwanwu.com/v1";

pub struct LingyiwanwuClient {
    openai_client: OpenAIClient,
}

impl LingyiwanwuClient {
    pub fn new(api_key: String, model: LingyiwanwuModels, api_base: Option<String>) -> Self {
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(&api_key)
            .with_api_base(api_base.unwrap_or(LING_YI_WAN_WU_API_BASE.to_owned()));
        let client = async_openai::Client::with_config(config);
        Self {
            openai_client: OpenAIClient::from_client_and_model(client, Box::new(model)),
        }
    }
}

impl AskModel for LingyiwanwuClient {
    async fn ask_model_with_context(
        &self,
        context: crate::ChatMessage,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.openai_client.ask_model_with_context(context).await
    }
}

#[cfg(test)]
mod tests {
    use crate::api::{
        lingyiwanwu::{LingyiwanwuClient, LingyiwanwuModels},
        test::test_client_with_context,
    };

    #[tokio::test]
    async fn test_lingyiwanwu_client_with_context() {
        let api_key = std::env::var("LINGYI_KEY");
        let api_base = std::env::var("LINGYI_API_BASE");

        match (api_key, api_base) {
            (Ok(api_key), Ok(api_base)) => {
                let client =
                    LingyiwanwuClient::new(api_key, LingyiwanwuModels::YiLarge, Some(api_base));

                test_client_with_context(client).await;
            }
            (Ok(api_key), Err(_)) => {
                let client = LingyiwanwuClient::new(api_key, LingyiwanwuModels::YiLarge, None);

                test_client_with_context(client).await;
            }
            _ => eprintln!("LINGYI_KEY is not set, skip this test."),
        }
    }
}
