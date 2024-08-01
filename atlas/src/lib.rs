pub mod api;

pub trait AskModel {
    fn ask_model(
        &self,
        question: &str,
    ) -> impl std::future::Future<Output = Result<String, Box<dyn std::error::Error>>> + Send;
}
