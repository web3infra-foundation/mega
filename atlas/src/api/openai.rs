use crate::{AskModel, ChatMessage, ChatRole};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};

pub struct OpenAIClient {
    model: Box<dyn crate::Model + 'static>,
    client: Client<OpenAIConfig>,
}

/// gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4, and gpt-3.5-turbo
pub enum OpenAIModels {
    GPT4O,
    GPT4OMini,
    GPT4Turbo,
    GPT4,
    GPT35Turbo,
}

impl crate::Model for OpenAIModels {
    fn as_str(&self) -> &str {
        match self {
            OpenAIModels::GPT4O => "gpt-4o",
            OpenAIModels::GPT4OMini => "gpt-4o-mini",
            OpenAIModels::GPT4Turbo => "gpt-4-turbo",
            OpenAIModels::GPT4 => "gpt-4",
            OpenAIModels::GPT35Turbo => "gpt-3.5-turbo",
        }
    }
}

impl OpenAIClient {
    pub fn new(api_key: String, model: OpenAIModels, api_base: Option<String>) -> Self {
        let config = match api_base {
            Some(api_base) => OpenAIConfig::new()
                .with_api_key(&api_key)
                .with_api_base(api_base),
            None => OpenAIConfig::new().with_api_key(&api_key),
        };
        let client = Client::with_config(config);

        Self {
            model: Box::new(model),
            client,
        }
    }

    pub fn from_client_and_model(
        client: Client<OpenAIConfig>,
        model: Box<dyn crate::Model + 'static>,
    ) -> Self {
        Self { model, client }
    }
}

impl AskModel for OpenAIClient {
    async fn ask_model_with_context(
        &self,
        _context: ChatMessage,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut messages: Vec<ChatCompletionRequestMessage> = vec![];
        for (role, content) in _context.messages.iter() {
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

#[cfg(test)]
mod test {
    use crate::api::{
        openai::{OpenAIClient, OpenAIModels},
        test::test_client_with_context,
    };

    #[tokio::test]
    async fn test_deepseek_client_with_context() {
        let api_key = std::env::var("OPENAI_KEY");
        let api_base = std::env::var("OPENAI_API_BASE");

        match (api_key, api_base) {
            (Ok(api_key), Ok(api_base)) => {
                let client = OpenAIClient::new(api_key, OpenAIModels::GPT4O, Some(api_base));

                test_client_with_context(client).await;
            }
            (Ok(api_key), Err(_)) => {
                let client = OpenAIClient::new(api_key, OpenAIModels::GPT4O, None);

                test_client_with_context(client).await;
            }
            _ => eprintln!("OPENAI_KEY is not set, skip this test."),
        }
    }
}
