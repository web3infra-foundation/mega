use crate::api::MegaApiServiceState;
use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use reqwest::Client;
use serde_json::Value;
use std::sync::LazyLock;
use utoipa_axum::router::OpenApiRouter;

pub fn routers() -> OpenApiRouter<MegaApiServiceState> {
    OpenApiRouter::new().route("/github/webhook", post(webhook))
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
    get_request(&format!("{pr_url}/files")).await
}

/// GitHub API: Get the commits of a pull request.
pub async fn get_pr_commits(pr_url: &str) -> Value {
    get_request(&format!("{pr_url}/commits")).await
}
/// Send a GET request to the given URL and return the JSON response.
async fn get_request(url: &str) -> Value {
    static CLIENT: LazyLock<Client> = LazyLock::new(|| {
        Client::builder()
            .user_agent("Mega/0.0.1") // IMPORTANT, or 403 Forbidden
            .build()
            .unwrap()
    });
    let resp = CLIENT.get(url).send().await.unwrap();
    resp.json().await.unwrap()
}
