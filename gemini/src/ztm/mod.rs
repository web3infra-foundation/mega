pub mod agent;
pub mod hub;

pub async fn handle_ztm_response(
    request_result: Result<reqwest::Response, reqwest::Error>,
) -> Result<String, String> {
    match request_result {
        Ok(res) => {
            if res.status().is_success() {
                Ok(res.text().await.unwrap())
            } else {
                Err(res.text().await.unwrap())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
