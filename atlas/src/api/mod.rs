pub mod claude;
pub mod gemini;
pub mod gitee;
pub mod lingyiwanwu;
pub mod openai;
#[cfg(test)]
mod test {
    use std::env;

    pub fn get_gemini_key() -> Option<String> {
        // Some("".to_string())
        env::var("GOOGLE_GEMINI_KEY").unwrap().into()
    }

    pub fn get_01_key() -> Option<String> {
        // Some("".to_string())
        env::var("LINGYI_KEY").unwrap().into()
    }

    pub fn get_giteeai_key() -> Option<String> {
        // Some("".to_string())
        env::var("GITEEAI_KEY").unwrap().into()
    }

    pub fn get_claude_key() -> Option<String> {
        // Some("".to_string())
        env::var("CLAUDE_KEY").unwrap().into()
    }

    pub async fn test_client_with_context(client: impl crate::AskModel) {
        let _context = crate::ChatMessage {
            messages: vec![
                (
                    crate::ChatRole::User,
                    "Resposponse a '0' no matter what you receive".into(),
                ),
                (
                    crate::ChatRole::Model,
                    "Ok, I will response with a number 0.".into(),
                ),
                (crate::ChatRole::User, "who are you".into()),
            ],
        };
        let res = client.ask_model_with_context(_context).await;
        match res {
            Ok(text) => {
                assert!(!text.is_empty());
                println!("model response with  {}", text);
            }
            Err(e) => {
                println!("{}", e);
                panic!();
            }
        }
    }
}
