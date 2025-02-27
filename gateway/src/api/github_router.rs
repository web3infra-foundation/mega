use crate::api::MegaApiServiceState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use lazy_static::lazy_static;
use reqwest::Client;
use serde_json::Value;
use taurus::event::github_webhook::{GithubWebhookEvent, WebhookType};

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

    let event_type = WebhookType::from(event_type);
    match event_type {
        WebhookType::PullRequest => {
            let action = payload["action"].as_str().unwrap();
            tracing::debug!("PR action: {}", action);

            if ["opened", "reopened", "synchronize"].contains(&action) {
                // contents changed
                let url = payload["pull_request"]["url"].as_str().unwrap();
                let files = get_pr_files(url).await;
                let commits = get_pr_commits(url).await;
                // Add details to the payload
                payload["files"] = files;
                payload["commits"] = commits;
            } else if action == "edited" {
                // PR title or body edited
                let _ = payload["pull_request"]["title"].as_str().unwrap();
                let _ = payload["pull_request"]["body"].as_str().unwrap();
            }

            GithubWebhookEvent::notify(WebhookType::PullRequest, payload);
        }
        WebhookType::Issues => {
            GithubWebhookEvent::notify(WebhookType::Issues, payload);
        }
        WebhookType::Unknown(_type) => {
            tracing::warn!("Unknown event type: {}", _type);
            GithubWebhookEvent::notify(WebhookType::Unknown(_type), payload);
        }
    }

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
