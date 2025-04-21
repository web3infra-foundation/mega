//! Anthropic's Claude API client, see [Anthropic Claude API](https://docs.anthropic.com/en/api/messages).

use serde::{Deserialize, Serialize};

use crate::{AskModel, Model};

/// Refer to the [Claude Models](https://docs.anthropic.com/en/docs/about-claude/models) for more information.
#[derive(Debug, Clone)]
pub enum ClaudeModels {
    Claude3_5Sonnet,
    Claude3Opus,
    Claude3Sonnet,
    Claude3Haiku,
}

impl Model for ClaudeModels {
    fn as_str(&self) -> &str {
        match self {
            ClaudeModels::Claude3_5Sonnet => "claude-3-5-sonnet-20240620",
            ClaudeModels::Claude3Opus => "claude-3-opus-20240229",
            ClaudeModels::Claude3Sonnet => "claude-3-sonnet-20240229",
            ClaudeModels::Claude3Haiku => "claude-3-haiku-20240307",
        }
    }
}

impl ClaudeModels {
    pub fn get_max_tokens(&self) -> usize {
        match self {
            ClaudeModels::Claude3_5Sonnet => 4096,
            ClaudeModels::Claude3Opus => 4096,
            ClaudeModels::Claude3Sonnet => 4096,
            ClaudeModels::Claude3Haiku => 4096,
        }
    }
}

const CLAUDE_API_BASE: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone)]
pub struct ClaudeClient {
    api_key: String,
    model: ClaudeModels,
    api_base: String,
    http_client: reqwest::Client,
}

impl ClaudeClient {
    pub fn new(api_key: String, model: ClaudeModels, api_base: Option<String>) -> Self {
        Self {
            api_key,
            model,
            api_base: api_base.unwrap_or(CLAUDE_API_BASE.to_owned()),
            http_client: reqwest::Client::new(),
        }
    }
}

impl AskModel for ClaudeClient {
    async fn ask_model_with_context(
        &self,
        context: crate::ChatMessage,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut request_body = MessageRequest {
            model: self.model.as_str().to_string(),
            max_tokens: self.model.get_max_tokens(),
            messages: vec![],
        };
        context.messages.iter().for_each(|(role, content)| {
            request_body.messages.push(Message {
                role: match role {
                    crate::ChatRole::User => MessageRole::User,
                    crate::ChatRole::Model => MessageRole::Assistant,
                },
                content: content.clone(),
            });
        });
        let body = serde_json::to_string(&request_body)?;
        let res = self
            .http_client
            .post(format!("{}/messages", &self.api_base))
            .body(body)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .send()
            .await?;

        let content = res.text().await?;
        let response: MessageResponse = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse response from Claude: {}\n{}", content, e))?;
        let content = &response.content[0];
        if response.content.len() != 1 || content.r#type != "text" {
            return Err("Failed to parse response from Claude.".into());
        }
        Ok(content.text.clone())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Message {
    role: MessageRole,
    content: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum MessageRole {
    User,
    Assistant,
}
/* {
  "content": [
    {
      "text": "Hi! My name is Claude.",
      "type": "text"
    }
  ],
  "id": "msg_013Zva2CMHLNnXjNJJKqJ2EF",
  "model": "claude-3-5-sonnet-20240620",
  "role": "assistant",
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "type": "message",
  "usage": {
    "input_tokens": 10,
    "output_tokens": 25
  }
}*/
#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: Vec<MessageContent>,
    // id: String,
    // model: String,
    // role: String,
    // stop_reason: String,
    // stop_sequence: Option<String>,
    // r#type: String,
    // usage: Usage,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    text: String,
    r#type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    input_tokens: i32,
    output_tokens: i32,
}

#[cfg(test)]
mod test {
    use crate::api::{
        claude::{ClaudeClient, ClaudeModels},
        test::test_client_with_context,
    };

    #[tokio::test]
    async fn test_claude_client_with_context() {
        let api_key = std::env::var("CLAUDE_KEY");
        let api_base = std::env::var("CLAUDE_API_BASE");

        match (api_key, api_base) {
            (Ok(api_key), Ok(api_base)) => {
                let client =
                    ClaudeClient::new(api_key, ClaudeModels::Claude3_5Sonnet, Some(api_base));

                test_client_with_context(client).await;
            }
            (Ok(api_key), Err(_)) => {
                let client = ClaudeClient::new(api_key, ClaudeModels::Claude3_5Sonnet, None);

                test_client_with_context(client).await;
            }
            _ => eprintln!("CLAUDE_KEY is not set, skip this test."),
        }
    }
}
