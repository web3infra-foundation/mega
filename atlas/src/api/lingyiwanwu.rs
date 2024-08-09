//! According to the [Lingyiwanwu API Reference](https://platform.lingyiwanwu.com/docs/api-reference), the Lingyiwanwu API is the same as the OpenAI API.
//! So this is just a wrapper around the OpenAI API, change api base.

use crate::AskModel;

use super::openai::OpenAIClient;
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
    pub fn new(api_key: String, model: LingyiwanwuModels) -> Self {
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(&api_key)
            .with_api_base(LING_YI_WAN_WU_API_BASE);
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
    use crate::{api::test::get_01_key, ChatRole};

    use super::*;

    #[tokio::test]
    async fn test_lingyiwanwu_client() {
        let api_key = get_01_key().unwrap();
        let model = LingyiwanwuModels::YiLarge;
        let client = LingyiwanwuClient::new(api_key, model);
        let response = client
            .ask_model("What is the meaning of life?")
            .await
            .unwrap();
        assert!(!response.is_empty());
        println!("Lingyiwanwu response: {}", response);
    }

    #[tokio::test]
    async fn test_lingyiwanwu_client_with_context() {
        let api_key = get_01_key().unwrap();
        let model = LingyiwanwuModels::YiLarge;
        let client = LingyiwanwuClient::new(api_key, model);
        let _context = crate::ChatMessage {
            messages: vec![
                (
                    ChatRole::User,
                    "Resposponse a '0' no matter what you receive".into(),
                ),
                (
                    ChatRole::Model,
                    "Ok, I will response with a number 0.".into(),
                ),
                (ChatRole::User, "What is the meaning of life?".into()),
            ],
        };
        let response = client.ask_model_with_context(_context).await.unwrap();
        assert!(!response.is_empty());
        println!("Lingyiwanwu response: {}", response);
    }
}
