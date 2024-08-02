pub mod gemini;
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
        env::var("GOOGLE_GEMINI_KEY").unwrap().into()
    }
}
