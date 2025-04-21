pub mod claude;
pub mod deepseek;
pub mod gemini;
pub mod gitee;
pub mod lingyiwanwu;
pub mod openai;

#[cfg(test)]
mod test {
    pub async fn test_client_with_context(client: impl crate::AskModel) {
        let context = crate::ChatMessage {
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
        let res = client.ask_model_with_context(context).await;
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
