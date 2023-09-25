//!
//!
//!
//!
//!

use std::path::PathBuf;
use std::str::FromStr;
use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;

use axum::extract::{State};
use axum::response::Response;
use axum::routing::post;
use axum::{Router, Server};
use clap::Args;
use database::driver::ObjectStorage;
use database::DataSource;
use hyper::{Body, Request, StatusCode, Uri};
use jsonwebtoken::EncodingKey;
use octocrab::{models::AppId, Octocrab};
use std::env;
use sync::service;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};


/// Parameters for starting the HTTP service
#[derive(Args, Clone, Debug)]
pub struct WebhookOptions {
    /// Server start hostname
    #[arg(long, default_value_t = String::from("0.0.0.0"))]
    pub host: String,

    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,

    #[arg(short, long, value_name = "FILE")]
    key_path: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    cert_path: Option<PathBuf>,

    #[arg(short, long, default_value_os_t = PathBuf::from("lfs_content"))]
    pub lfs_content_path: PathBuf,

    #[arg(short, long, value_enum, default_value = "postgres")]
    pub data_source: DataSource,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn ObjectStorage>,
    pub options: WebhookOptions,
}

pub fn remove_git_suffix(uri: Uri, git_suffix: &str) -> PathBuf {
    PathBuf::from(uri.path().replace(".git", "").replace(git_suffix, ""))
}

pub async fn webhook_server(options: &WebhookOptions) -> Result<(), Box<dyn std::error::Error>> {
    // Read environment variables
    let github_app_id = env::var("GITHUB_APP_ID").expect("Missing GITHUB_APP_ID");
    let github_private_key = env::var("GITHUB_PRIVATE_KEY").expect("Missing GITHUB_PRIVATE_KEY");
    let _webhook_secret = env::var("GITHUB_WEBHOOK_SECRET").expect("Missing GITHUB_WEBHOOK_SECRET");

    // Create RSA private key from the provided environment variable
    let rsa_key = EncodingKey::from_rsa_pem(github_private_key.as_bytes())
        .expect("Failed to load private key");
    // Create Octocrab instance for GitHub App authentication
    let _octocrab = Octocrab::builder()
        .app(AppId::from(github_app_id.parse::<u64>().unwrap()), rsa_key)
        .build()
        .expect("Failed to create Octocrab instance");

    let WebhookOptions {
        host,
        port,
        key_path: _,
        cert_path: _,
        lfs_content_path: _,
        data_source,
    } = options;
    let server_url = format!("{}:{}", host, port);

    let state = AppState {
        storage: database::init(data_source).await,
        options: options.to_owned(),
    };
    let app = Router::new()
        .route("/", post(post_method_router))
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

async fn post_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {

    println!("{:?}", uri.path());

    //resolve the pull request event
    let pull_request_event = service::resolve_pull_request_event(req).await;
    match pull_request_event.action().as_str(){
        "opened" => {
            state.storage.save_pull_request(pull_request_event.convert_to_model()).await.unwrap();
            let pull_request_ = state.storage.get_pull_request_by_id(pull_request_event.id()).await.unwrap().unwrap();
            println!("{:?}", pull_request_);
        },
        "reopened" | 
        "closed" => {
            state.storage.update_pull_request(pull_request_event.convert_to_model()).await.unwrap();
            let pull_request_ = state.storage.get_pull_request_by_id(pull_request_event.id()).await.unwrap().unwrap();
            println!("{:?}", pull_request_);
        }
        _ => {},
    }

    // resolve the issue event
    // let issue_event = service::resolve_issue_event(req).await;
    // match issue_event.action().as_str(){
    //     "opened" => {
    //         state.storage.save_issue(issue_event.convert_to_model()).await.unwrap();
    //         let issue_ = state.storage.get_issue_by_id(issue_event.id()).await.unwrap().unwrap();
    //         println!("{:?}", issue_);
    //     },
    //     "reopened" | 
    //     "closed" => {
    //         state.storage.update_issue(issue_event.convert_to_model()).await.unwrap();
    //         let issue_ = state.storage.get_issue_by_id(issue_event.id()).await.unwrap().unwrap();
    //         println!("{:?}", issue_);
    //     }
    //     _ => {},
    // }


    let response = Response::builder()
        .status(200)
        .header("X-Custom-Foo", "Bar")
        .body(Body::empty())
        .unwrap();
    Ok(response)
}



#[cfg(test)]
mod tests {}