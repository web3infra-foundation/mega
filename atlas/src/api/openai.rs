use crate::AskModel;
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestSystemMessageArgs, CreateChatCompletionRequestArgs},
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
    pub fn new(api_key: String, model: OpenAIModels) -> Self {
        let config = OpenAIConfig::new().with_api_key(&api_key);
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
    async fn ask_model(&self, question: &str) -> Result<String, Box<dyn std::error::Error>> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.as_str())
            .messages([ChatCompletionRequestSystemMessageArgs::default()
                .content(question)
                .build()?
                .into()])
            .build()
            .unwrap();

        // debug, make request to json
        let json_str = serde_json::to_string(&request).unwrap();
        println!("json_str: {}", json_str);
        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| format!("Failed to get response : {}", e))?;

        Ok(response.choices[0].message.content.clone().unwrap())
    }
}
