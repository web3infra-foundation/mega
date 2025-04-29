use crate::api::MegaApiServiceState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use lazy_static::lazy_static;
use reqwest::Client;
use serde_json::Value;

lazy_static! {
    static ref CLIENT: Client = Client::builder()
        .user_agent("Mega/0.0.1") // IMPORTANT, or 403 Forbidden
        .build()
        .unwrap();
}

pub fn routers() -> Router<MegaApiServiceState> {
    Router::new().route("/github/webhook", post(webhook))
}

/// Handle the GitHub webhook event. <br>
/// For more details, see https://docs.github.com/zh/webhooks/webhook-events-and-payloads.
async fn webhook(
    headers: HeaderMap,
    Json(mut payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .expect("Missing X-GitHub-Event header");
    payload["event_type"] = event_type.into();

    // TODO: Handle the webhook event.

    Ok("WebHook OK")
}

/// GitHub API: Get the files change of a pull request. <br>
/// For read-only operation of public repos, no authentication is required.
pub async fn get_pr_files(pr_url: &str) -> Value {
    get_request(&format!("{}/files", pr_url)).await
}

/// GitHub API: Get the commits of a pull request.
pub async fn get_pr_commits(pr_url: &str) -> Value {
    get_request(&format!("{}/commits", pr_url)).await
}

/// Send a GET request to the given URL and return the JSON response.
async fn get_request(url: &str) -> Value {
    let resp = CLIENT.get(url).send().await.unwrap();
    resp.json().await.unwrap()
}
