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
