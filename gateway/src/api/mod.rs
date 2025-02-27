use common::model::ZtmOptions;
use mono::api::MonoApiServiceState;

pub mod github_router;
mod model;
pub mod nostr_router;
pub mod ztm_router;

#[derive(Clone)]
pub struct MegaApiServiceState {
    pub inner: MonoApiServiceState,
    pub port: u16,
    pub ztm: ZtmOptions,
}
