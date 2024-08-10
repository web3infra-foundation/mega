use std::fmt::format;
use axum::{Json, Router};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use lazy_static::lazy_static;
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde_json::Value;
use common::model::CommonResult;
use crate::api::ApiServiceState;

// const GITHUB_API_SERVER: &str = "https://api.github.com";
// const OWNER: &str = "web3infra-foundation";
// const REPO: &str = "mega";

lazy_static! {
    static ref CLIENT: Client = Client::builder()
        .user_agent("Mega/0.0.1") // IMPORTANT, or 403 Forbidden
        .build()
        .unwrap();
}

pub fn routers() -> Router<ApiServiceState> {
    Router::new()
        .route("/github/webhook", post(webhook))
}

async fn webhook(
    headers: HeaderMap,
    Json(payload): Json<Value>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .expect("Missing X-GitHub-Event header");

    tracing::debug!("WebHook Event Type: {}", event_type);

    if event_type == "pull_request" {
        let action = payload["action"].as_str().unwrap();
        tracing::debug!("PR action: {}", action);

        if ["opened", "reopened", "synchronize"].contains(&action) {
            let url = payload["pull_request"]["url"].as_str().unwrap();
            let files = get_pr_files(url).await;
            let commits = get_pr_commits(url).await;
            tracing::debug!("PR: {:#?}", files);
            tracing::debug!("Commits: {:#?}", commits);
        } else if action == "edited" { // PR title or body edited
            let _title = payload["pull_request"]["title"].as_str().unwrap();
            let _body = payload["pull_request"]["body"].as_str().unwrap();
        }
    }

    Ok("WebHook OK")
}

async fn get_pr_files(pr_url: &str) -> Value {
    get_request(&format!("{}/files", pr_url)).await
}

async fn get_pr_commits(pr_url: &str) -> Value {
    get_request(&format!("{}/commits", pr_url)).await
}

/// Send a GET request to the given URL and return the JSON response.
async fn get_request(url: &str) -> Value {
    let resp = CLIENT.get(url).send().await.unwrap();
    resp.json().await.unwrap()
}