pub mod api;

pub trait Model: Sync {
    fn as_str(&self) -> &str;
}

pub trait AskModel {
    fn ask_model(
        &self,
        question: &str,
    ) -> impl std::future::Future<Output = Result<String, Box<dyn std::error::Error>>> + Send;
}
