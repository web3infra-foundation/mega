use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};

use crate::{AskModel, ChatRole};

/// refs: https://api-docs.deepseek.com/zh-cn/quick_start/pricing
#[derive(Debug, Clone)]
pub enum DeepSeekModels {
    DeepSeekChat,
    DeepSeekReasoner,
}

impl DeepSeekModels {
    pub fn as_str(&self) -> &str {
        match self {
            DeepSeekModels::DeepSeekChat => "deepseek-chat",
            DeepSeekModels::DeepSeekReasoner => "deepseek-reasoner",
        }
    }
}

const DEEPSEEK_BASE_API: &str = "https://api.deepseek.com/v1";

#[derive(Debug, Clone)]
pub struct DeepSeekClient {
    client: Client<OpenAIConfig>,
    model: DeepSeekModels,
}

/// refs: https://api-docs.deepseek.com/zh-cn/
/// The DeepSeek API is fully compatible with OpenAI's API,
/// so the same client as OpenAI is used to make requests here.
impl DeepSeekClient {
    pub fn new(api_key: String, model: DeepSeekModels, api_base: Option<String>) -> Self {
        let api_base = match api_base {
            Some(api) => api,
            None => DEEPSEEK_BASE_API.to_owned(),
        };

        let config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(&api_key);

        let client = Client::with_config(config);

        Self { client, model }
    }
}

impl AskModel for DeepSeekClient {
    async fn ask_model_with_context(
        &self,
        context: crate::ChatMessage,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut messages: Vec<ChatCompletionRequestMessage> = vec![];
        for (role, content) in context.messages.iter() {
            match role {
                ChatRole::User => {
                    messages.push(
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(content.as_str())
                            .build()?
                            .into(),
                    );
                }
                ChatRole::Model => {
                    messages.push(
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(content.as_str())
                            .build()?
                            .into(),
                    );
                }
            }
        }

        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.as_str())
            .messages(messages)
            .build()
            .unwrap();

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| format!("Failed to get response : {}", e))?;

        Ok(response.choices[0].message.content.clone().unwrap())
    }
}
