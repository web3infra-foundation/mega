use common::model::ZtmOptions;
use mono::api::MonoApiServiceState;

pub mod github_router;
pub mod nostr_router;
pub mod ztm_router;
mod model;

#[derive(Clone)]
pub struct MegaApiServiceState {
    pub inner: MonoApiServiceState,
    pub port: u16,
    pub ztm: ZtmOptions,
}
