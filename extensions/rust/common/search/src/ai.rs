use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use tokio_postgres::Client as PgClient;

/// 模型请求体结构体
#[derive(Serialize)]
struct RequestBody {
    model: String,
    messages: Vec<Message>,
}

/// 消息结构体
#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

/// 响应选择结构体
#[derive(Deserialize)]
struct ResponseChoice {
    message: Message,
}

/// 响应体结构体
#[derive(Deserialize)]
struct ResponseBody {
    choices: Vec<ResponseChoice>,
}

/// 聊天上下文结构体
struct ChatContext {
    messages: Vec<Message>,
}

impl ChatContext {
    /// 创建新的聊天上下文
    fn new() -> Self {
        ChatContext {
            messages: Vec::new(),
        }
    }

    /// 添加消息到聊天上下文
    fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    /// 获取聊天上下文中的所有消息
    fn get_messages(&self) -> &Vec<Message> {
        &self.messages
    }
}

/// AI 聊天结构体
pub struct AIChat<'a> {
    context: ChatContext,
    client: &'a PgClient,
}

impl<'a> AIChat<'a> {
    /// 创建新的 AI 聊天实例
    pub fn new(client: &'a PgClient) -> Self {
        let mut ret = AIChat {
            context: ChatContext::new(),
            client,
        };
        ret.context.add_message(
            "system",
            "You are an experienced rust programmer, know all the major crates.You should help user and answer the question.",
        );
        ret
    }

    /// 处理用户消息并返回 AI 响应
    pub async fn chat(&mut self, user_message: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.context.add_message("user", user_message);
        let answer = query_openai(&self.context).await?;
        self.context.add_message("assistant", &answer);
        Ok(answer)
    }

    /// 处理带有嵌入信息的用户消息并返回 AI 响应
    pub async fn chat_with_embedding(
        &mut self,
        user_message: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let addition_infomation = get_crate_info_with_embedding(self.client, user_message).await?;
        let user_message = format!(
            "Here are some revelvant crates to refer:{addition_infomation}. Question:{user_message}"
        );
        self.context.add_message("user", user_message.as_str());
        let answer = query_openai(&self.context).await?;
        self.context.add_message("assistant", &answer);
        Ok(answer)
    }
}

/// 查询 OpenAI API 并返回响应
async fn query_openai(context: &ChatContext) -> Result<String, reqwest::Error> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = Client::new();
    let open_ai_chat_url = env::var("OPEN_AI_CHAT_URL").expect("OPEN_AI_CHAT_URL not set");

    let request_body = RequestBody {
        model: "gpt-3.5-turbo".to_string(),
        messages: context.get_messages().clone(),
    };

    let response = client
        .post(&open_ai_chat_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;

    let response_body: ResponseBody = response.json().await?;

    Ok(response_body.choices[0].message.content.clone())
}

/// 获取带有嵌入信息的 crate 信息
async fn get_crate_info_with_embedding(
    client: &PgClient,
    question: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let question_embedding = crate::embedding::get_one_text_embedding(question).await?;
    let top_n = 5;
    let results: Vec<(i32, String, String)> =
        crate::embedding::search_crates_by_embedding(client, &question_embedding, top_n).await?;

    let mut results_text = String::new();
    for (id, name, description) in results {
        results_text.push_str(&format!(
            "ID: {id}, Name: {name}, Description: {description}\n"
        ));
    }

    Ok(results_text)
}
