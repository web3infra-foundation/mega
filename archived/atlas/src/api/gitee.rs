//! The GiteeAI Serverless API is similar to the OpenAI API, so we can use the same code with a different API base URL.
//! Note that GiteeAI does not guarantee to maintain the same API structure as OpenAI, so this may break in the future.
//! GiteeAI uses URL parameters to specify the model, so there's no need to set the model in the request body.

use crate::api::openai::OpenAIClient;
use crate::{AskModel, Model};

/// [GiteeAI Serverless API](https://ai.gitee.com/serverless-api)
#[derive(Debug, Clone)]
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
    pub fn new(api_key: String, model: GiteeServerlessModels, api_base: Option<String>) -> Self {
        let api_base = api_base.unwrap_or(GITEE_SERVERLESS_API_BASE.to_owned());
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(&api_key)
            .with_api_base(format!("{}/{}", api_base, model.as_str()));
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

    use crate::api::{
        gitee::{GiteeServerlessClient, GiteeServerlessModels},
        test::test_client_with_context,
    };

    #[tokio::test]
    async fn test_openai_rs_with_gitee() {
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(std::env::var("GITEEAI_KEY").unwrap())
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
        let api_key = std::env::var("GITEEAI_KEY");
        let api_base = std::env::var("GITEEAI_API_BASE");

        match (api_key, api_base) {
            (Ok(api_key), Ok(api_base)) => {
                let client = GiteeServerlessClient::new(
                    api_key,
                    GiteeServerlessModels::Qwen2_7bInstruct,
                    Some(api_base),
                );

                test_client_with_context(client).await;
            }
            (Ok(api_key), Err(_)) => {
                let client = GiteeServerlessClient::new(
                    api_key,
                    GiteeServerlessModels::Qwen2_7bInstruct,
                    None,
                );

                test_client_with_context(client).await;
            }
            _ => eprintln!("GITEEAI_KEY is not set, skip this test."),
        }
    }
}
