//! Google Gemini API client, see [Google Gemini API](https://ai.google.dev/api/caching)

use serde::{Deserialize, Serialize};

use crate::{AskModel, ChatMessage, ChatRole};

#[derive(Debug, Clone)]
pub enum GeminiModels {
    ChatBison001,
    TextBison001,
    EmbeddingGecko001,
    Gemini10ProLatest,
    Gemini10Pro,
    GeminiPro,
    Gemini10Pro001,
    Gemini10ProVisionLatest,
    GeminiProVision,
    Gemini15ProLatest,
    Gemini15Pro001,
    Gemini15Pro,
    Gemini15FlashLatest,
    Gemini15Flash001,
    Gemini15Flash,
    Embedding001,
    TextEmbedding004,
    AQA,
}

impl GeminiModels {
    pub fn as_str(&self) -> &str {
        match self {
            GeminiModels::ChatBison001 => "models/chat-bison-001",
            GeminiModels::TextBison001 => "models/text-bison-001",
            GeminiModels::EmbeddingGecko001 => "models/embedding-gecko-001",
            GeminiModels::Gemini10ProLatest => "models/gemini-1.0-pro-latest",
            GeminiModels::Gemini10Pro => "models/gemini-1.0-pro",
            GeminiModels::GeminiPro => "models/gemini-pro",
            GeminiModels::Gemini10Pro001 => "models/gemini-1.0-pro-001",
            GeminiModels::Gemini10ProVisionLatest => "models/gemini-1.0-pro-vision-latest",
            GeminiModels::GeminiProVision => "models/gemini-pro-vision",
            GeminiModels::Gemini15ProLatest => "models/gemini-1.5-pro-latest",
            GeminiModels::Gemini15Pro001 => "models/gemini-1.5-pro-001",
            GeminiModels::Gemini15Pro => "models/gemini-1.5-pro",
            GeminiModels::Gemini15FlashLatest => "models/gemini-1.5-flash-latest",
            GeminiModels::Gemini15Flash001 => "models/gemini-1.5-flash-001",
            GeminiModels::Gemini15Flash => "models/gemini-1.5-flash",
            GeminiModels::Embedding001 => "models/embedding-001",
            GeminiModels::TextEmbedding004 => "models/text-embedding-004",
            GeminiModels::AQA => "models/aqa",
        }
    }
}

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com";

#[derive(Debug, Clone)]
pub struct GeminiClient {
    api_key: String,
    model: GeminiModels,
    api_base: String,
}

impl GeminiClient {
    pub fn new(api_key: String, model: GeminiModels, api_base: Option<String>) -> Self {
        Self {
            api_key,
            model,
            api_base: api_base.unwrap_or(GEMINI_API_BASE.to_owned()),
        }
    }
}

impl AskModel for GeminiClient {
    async fn ask_model_with_context(
        &self,
        _context: ChatMessage,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/v1beta/{}:generateContent?key={}",
            self.api_base,
            self.model.as_str(),
            self.api_key
        );
        let client = reqwest::Client::new();
        let mut contents = CachedContents { contents: vec![] };
        _context.messages.iter().for_each(|(role, content)| {
            contents.contents.push(Content {
                parts: vec![Part::Text {
                    text: content.clone(),
                }],
                role: match role {
                    ChatRole::User => "user".to_string(),
                    ChatRole::Model => "model".to_string(),
                },
            });
        });

        let body = serde_json::to_string(&contents).unwrap();
        let res = client
            .post(&url)
            .body(body)
            .header("Content-Type", "application/json; charset=utf-8")
            .send()
            .await?;

        let content = res.text().await?;
        let response: GeminiResponse = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse response from Gemini: {}\n{}", content, e))?;
        let content = &response.candidates[0].content;
        if content.role != "model" && content.parts.len() != 1 {
            return Err("Failed to parse response from Gemini".into());
        }
        Ok(match &content.parts[0] {
            Part::Text { text } => text.clone(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedContents {
    contents: Vec<Content>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    // prompt_feedback: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Content,
    // finish_reason: String,
    // index: i32,
    // safetry_ratings: Vec<SafetyRating>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Content {
    parts: Vec<Part>,
    role: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct UsageMegadata {
    prompt_token_count: i32,
    candidates_token_count: i32,
    total_token_count: i32,
}

#[cfg(test)]
mod test {
    use crate::api::{
        gemini::{GeminiClient, GeminiModels},
        test::test_client_with_context,
    };

    #[tokio::test]
    async fn test_gemini_client_with_context() {
        let api_key = std::env::var("GOOGLE_GEMINI_KEY");
        let api_base = std::env::var("GOOGLE_GEMINI_API_BASE");

        match (api_key, api_base) {
            (Ok(api_key), Ok(api_base)) => {
                let client =
                    GeminiClient::new(api_key, GeminiModels::Gemini15Flash, Some(api_base));

                test_client_with_context(client).await;
            }
            (Ok(api_key), Err(_)) => {
                let client = GeminiClient::new(api_key, GeminiModels::Gemini15Flash, None);

                test_client_with_context(client).await;
            }
            _ => eprintln!("GOOGLE_GEMINI_KEY is not set, skip this test."),
        }
    }
}
