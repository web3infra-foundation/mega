use callisto::ztm_nostr_req;
use gemini::nostr::client_message::Filter;
use serde::{Deserialize, Serialize};

pub mod api;
pub mod relay_server;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Req {
    pub subscription_id: String,
    pub filters: Vec<Filter>,
}

impl From<ztm_nostr_req::Model> for Req {
    fn from(n: ztm_nostr_req::Model) -> Self {
        let filters: Vec<Filter> = serde_json::from_str(&n.filters).unwrap();
        Req {
            subscription_id: n.subscription_id,
            filters,
        }
    }
}

impl Req {
    fn filters_json(&self) -> String {
        serde_json::to_string(&self.filters).unwrap()
    }
}

#[cfg(test)]
mod tests {}
