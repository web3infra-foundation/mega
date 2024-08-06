//! The GiteeAI Serverless API is similar to the OpenAI API, so we can use the same code with a different API base URL.
//! Note that GiteeAI does not guarantee to maintain the same API structure as OpenAI, so this may break in the future.
//! GiteeAI uses URL parameters to specify the model, so there's no need to set the model in the request body.

use crate::{AskModel, Model};

use super::openai::OpenAIClient;

/// [GiteeAI Serverless API](https://ai.gitee.com/serverless-api)
pub enum GiteeServerlessModels {
    Qwen2_7bInstruct,
    Qwen2_72bInstruct,
    Yi1_5_34bChat,
}
impl Model for GiteeServerlessModels {
    fn as_str(&self) -> &str {
        match self {
            GiteeServerlessModels::Qwen2_7bInstruct => "YP3A1DT28TAJ",
            GiteeServerlessModels::Qwen2_72bInstruct => "H87ZZLSFILML",
            GiteeServerlessModels::Yi1_5_34bChat => "KIXIB7TOZA1U",
        }
    }
}

const GITEE_SERVERLESS_API_BASE: &str = "https://ai.gitee.com/api/inference/serverless";

pub struct GiteeServerlessClient {
    openai_client: OpenAIClient,
}

impl GiteeServerlessClient {
    pub fn new(api_key: String, model: GiteeServerlessModels) -> Self {
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(&api_key)
            .with_api_base(format!("{}/{}", GITEE_SERVERLESS_API_BASE, model.as_str()));
        let client = async_openai::Client::with_config(config);
        Self {
            openai_client: OpenAIClient::from_client_and_model(client, Box::new(model)),
        }
    }
}

impl AskModel for GiteeServerlessClient {
    async fn ask_model_with_context(
        &self,
        context: crate::ChatMessage,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.openai_client.ask_model_with_context(context).await
    }
}

#[cfg(test)]
mod test {
    use async_openai::types::{
        ChatCompletionRequestSystemMessageArgs, CreateChatCompletionRequestArgs,
    };

    use super::*;
    use crate::{api::test::get_giteeai_key, ChatRole};
    #[tokio::test]
    async fn test_openai_rs_with_gitee() {
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(get_giteeai_key().unwrap())
            .with_api_base("https://ai.gitee.com/api/inference/serverless/H87ZZLSFILML");
        let client = async_openai::Client::with_config(config);

        let request = CreateChatCompletionRequestArgs::default()
            .model("")
            .messages([ChatCompletionRequestSystemMessageArgs::default()
                .content("What is the meaning of life?")
                .build()
                .unwrap()
                .into()])
            .build()
            .unwrap();

        let response = client.chat().create(request).await.unwrap();
        println!("{}", response.choices[0].message.content.clone().unwrap());
    }

    #[tokio::test]
    async fn test_gitee_serverless_client_with_context() {
        let api_key = get_giteeai_key().unwrap();
        let model = GiteeServerlessModels::Qwen2_7bInstruct;
        let client = GiteeServerlessClient::new(api_key, model);
        let _context = crate::ChatMessage {
            messages: vec![
                (
                    ChatRole::User,
                    "Resposponse a '7' no matter what you receive".into(),
                ),
                (
                    ChatRole::Model,
                    "Ok, I will response with a number 7.".into(),
                ),
                (ChatRole::User, "What is the meaning of life?".into()),
            ],
        };
        let response = client.ask_model_with_context(_context).await.unwrap();
        assert!(!response.is_empty());
        println!("GiteeAI response: {}", response);
    }
}
