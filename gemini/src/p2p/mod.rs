use std::fmt;

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
pub enum ConnectionType {
    MSG,
    FILE,
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConnectionType::MSG => {
                write!(f, "MSG")
            }
            ConnectionType::FILE => {
                write!(f, "FILE")
            }
        }
    }
}
