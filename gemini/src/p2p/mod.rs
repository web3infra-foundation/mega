use std::fmt;

use callisto::import_refs;
use serde::{Deserialize, Serialize};

pub mod client;
pub mod relay;

pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"h3"];

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    Ping,
    Send,
    Call,
    Callback,
    RepoShare,
    Nostr,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::Ping => {
                write!(f, "Ping")
            }
            Action::Send => {
                write!(f, "Send")
            }
            Action::Call => {
                write!(f, "Call")
            }
            Action::Callback => {
                write!(f, "Callback")
            }
            Action::RepoShare => {
                write!(f, "RepoShare")
            }
            Action::Nostr => {
                write!(f, "Nostr")
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestData {
    pub from: String,
    pub data: Vec<u8>,
    pub func: String,
    pub action: Action,
    pub to: String,
    pub req_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseData {
    pub from: String,
    pub data: Vec<u8>,
    pub func: String,
    pub err: String,
    pub to: String,
    pub req_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitCloneHeader {
    pub from: String,
    pub target: String,
    pub git_path: String,
    pub branches: Vec<import_refs::Model>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LFSHeader {
    pub from: String,
    pub target: String,
    pub oid: String,
    pub size: i64,
}

#[cfg(test)]
mod tests {
    use crate::nostr::GitEventReq;
    use crate::p2p::client;
    use crate::util::repo_path_to_identifier;
    use common::config::Config;
    use jupiter::context::Context;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_repo_share() {
        test_with_logs();
        let config = Config::new("E:\\code\\mega\\config.toml").unwrap();
        let context = Context::new(Arc::from(config)).await;
        let context_clone = context.clone();
        tokio::spawn(async move {
            client::run(context_clone.clone(), "127.0.0.1:8001".to_string())
                .await
                .unwrap();
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let context_clone = context.clone();
        let i = client::repo_share(
            context_clone.clone(),
            "/third-part/git_inner_net".to_string(),
        )
        .await
        .unwrap();
        println!("{:?}", i);
    }

    #[tokio::test]
    async fn test_subscribe_repo() {
        test_with_logs();
        let config = Config::new("E:\\code\\mega\\config.toml").unwrap();
        let context = Context::new(Arc::from(config)).await;
        let context_clone = context.clone();
        tokio::spawn(async move {
            client::run(context_clone.clone(), "127.0.0.1:8001".to_string())
                .await
                .unwrap();
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        client::subscribe_repo(
            "p2p://24ok5kQFeJ3wTKYM31e75LwUzZv1e3xsGG7SE9XpdAtvX/third-part/git_inner_net.git"
                .to_string(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_send_repo_event() {
        test_with_logs();
        let config = Config::new("E:\\code\\mega\\config.toml").unwrap();
        let context = Context::new(Arc::from(config)).await;
        let context_clone = context.clone();
        tokio::spawn(async move {
            client::run(context_clone.clone(), "127.0.0.1:8001".to_string())
                .await
                .unwrap();
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let req = GitEventReq {
            path: "/third-part/git_inner_net".to_string(),
            action: "update".to_string(),
            title: "Feature:Nostr Test".to_string(),
            content: "Feature:Nostr Test".to_string(),
        };
        let git_db_storage = context.services.git_db_storage.clone();
        let git_model = git_db_storage
            .find_git_repo_exact_match(&req.path)
            .await
            .unwrap()
            .unwrap();
        let git_ref = git_db_storage
            .get_default_ref(git_model.id)
            .await
            .unwrap()
            .unwrap();
        let identifier = repo_path_to_identifier(git_model.repo_path).await;
        let git_event = req.to_git_event(identifier, git_ref.ref_git_id).await;

        client::send_git_event(git_event).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    fn test_with_logs() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();
    }
}
