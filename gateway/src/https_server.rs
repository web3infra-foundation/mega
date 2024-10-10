use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::{thread, time};

use axum::routing::get;
use axum::{http, Router};
use axum_server::tls_rustls::RustlsConfig;
use clap::Args;

use gemini::http::cache_repo::cache_public_repository;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use common::config::Config;
use common::model::{CommonOptions, ZtmOptions};
use gemini::ztm::agent::{run_ztm_client, LocalZTMAgent};
use jupiter::context::Context;
use mono::api::lfs::lfs_router;
use mono::api::MonoApiServiceState;
use mono::server::https_server::{get_method_router, post_method_router, AppState};

use crate::api::{github_router, nostr_router, ztm_router, MegaApiServiceState};

#[derive(Args, Clone, Debug)]
pub struct HttpOptions {
    #[clap(flatten)]
    pub common: CommonOptions,

    #[clap(flatten)]
    pub ztm: ZtmOptions,

    #[arg(long, default_value_t = 8000)]
    pub http_port: u16,
}

#[derive(Args, Clone, Debug)]
pub struct HttpsOptions {
    #[clap(flatten)]
    pub common: CommonOptions,

    #[clap(flatten)]
    pub ztm: ZtmOptions,

    #[arg(long, default_value_t = 443)]
    pub https_port: u16,

    #[arg(long, value_name = "FILE")]
    pub https_key_path: PathBuf,

    #[arg(long, value_name = "FILE")]
    pub https_cert_path: PathBuf,
}

pub async fn https_server(config: Config, options: HttpsOptions) {
    let HttpsOptions {
        common: CommonOptions { host, .. },
        https_key_path,
        https_cert_path,
        https_port,
        ztm,
    } = options.clone();

    check_run_with_ztm(config.clone(), options.ztm.clone(), https_port);

    let app = app(
        config,
        host.clone(),
        https_port,
        options.common.clone(),
        ztm.clone(),
    )
    .await;

    let server_url = format!("{}:{}", host, https_port);
    let addr = SocketAddr::from_str(&server_url).unwrap();
    let config = RustlsConfig::from_pem_file(https_cert_path.to_owned(), https_key_path.to_owned())
        .await
        .unwrap();
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn http_server(config: Config, options: HttpOptions) {
    let HttpOptions {
        common: CommonOptions { host, .. },
        http_port,
        ztm,
    } = options.clone();

    check_run_with_ztm(config.clone(), options.ztm.clone(), http_port);

    let app = app(
        config,
        host.clone(),
        http_port,
        options.common.clone(),
        ztm.clone(),
    )
    .await;

    let server_url = format!("{}:{}", host, http_port);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(
    config: Config,
    host: String,
    port: u16,
    common: CommonOptions,
    ztm: ZtmOptions,
) -> Router {
    let context = Context::new(config.clone()).await;
    context.services.mono_storage.init_monorepo().await;
    let state = AppState {
        host,
        port,
        context: context.clone(),
        common: common.clone(),
    };

    let mega_api_state = MegaApiServiceState {
        inner: MonoApiServiceState {
            context: context.clone(),
            common: common.clone(),
            oauth_client: None,
            store: None,
        },
        ztm,
        port,
    };

    let mono_api_state = MonoApiServiceState {
        context: context.clone(),
        common: common.clone(),
        oauth_client: None,
        store: None,
    };

    pub fn mega_routers() -> Router<MegaApiServiceState> {
        Router::new()
            .merge(ztm_router::routers())
            .merge(nostr_router::routers())
            .merge(github_router::routers())
    }

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    Router::new()
        .nest(
            "/",
            lfs_router::routers().with_state(mono_api_state.clone()),
        )
        .nest(
            "/api/v1/mono",
            mono::api::api_router::routers().with_state(mono_api_state.clone()),
        )
        .nest(
            "/api/v1/mega",
            mega_routers().with_state(mega_api_state.clone()),
        )
        // Using Regular Expressions for Path Matching in Protocol
        .route("/*path", get(get_method_router).post(post_method_router))
        .layer(
            ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any).allow_headers(vec![
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
            ])),
        )
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(state)
}

pub fn check_run_with_ztm(config: Config, ztm: ZtmOptions, http_port: u16) {
    //Mega server join a ztm mesh
    match ztm.bootstrap_node {
        Some(bootstrap_node) => {
            tracing::info!(
                "The bootstrap node is {}, prepare to join ztm network",
                bootstrap_node.clone()
            );
            let (peer_id, _) = vault::init();
            let ztm_agent: LocalZTMAgent = LocalZTMAgent {
                agent_port: ztm.ztm_agent_port,
            };
            ztm_agent.clone().start_ztm_agent();
            thread::sleep(time::Duration::from_secs(3));

            let bootstrap_node_clone = bootstrap_node.clone();
            let config_clone = config.clone();
            let ztm_agent_clone = ztm_agent.clone();
            tokio::spawn(async move {
                run_ztm_client(
                    bootstrap_node_clone,
                    config_clone,
                    peer_id,
                    ztm_agent_clone,
                    http_port,
                )
                .await
            });

            if ztm.cache_repo {
                thread::sleep(time::Duration::from_secs(3));
                tokio::spawn(async move {
                    let context = Context::new(config.clone()).await;
                    context.services.mono_storage.init_monorepo().await;
                    cache_public_repository(bootstrap_node, context, ztm_agent).await
                });
            }
        }
        None => {
            tracing::info!("The bootstrap node is not set, prepare to start mega server locally");
        }
    };
}

#[cfg(test)]
mod tests {}
