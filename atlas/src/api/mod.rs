pub mod gemini;

#[cfg(test)]
mod test {
    use std::env;

    pub fn get_gemini_key() -> Option<String> {
        env::var("GOOGLE_GEMINI_KEY").unwrap().into()
    }
}
