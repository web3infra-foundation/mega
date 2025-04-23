pub mod api;

pub trait Model: Sync {
    fn as_str(&self) -> &str;
}

pub trait AskModel {
    fn ask_model(
        &self,
        question: &str,
    ) -> impl std::future::Future<Output = Result<String, Box<dyn std::error::Error>>> + Send {
        self.ask_model_with_context(ChatMessage {
            messages: vec![(ChatRole::User, question.to_string())],
        })
    }

    /// ask model with context messages, the last message should be user's current message
    /// see [openai docs](https://platform.openai.com/docs/api-reference/chat/create) for more details
    fn ask_model_with_context(
        &self,
        context: ChatMessage,
    ) -> impl std::future::Future<Output = Result<String, Box<dyn std::error::Error>>> + Send;
}
pub enum ChatRole {
    User,
    Model,
}

/// ChatMessage is a vector of (role, content), and role must be `user` or `model`.
pub struct ChatMessage {
    pub messages: Vec<(ChatRole, String)>,
}
