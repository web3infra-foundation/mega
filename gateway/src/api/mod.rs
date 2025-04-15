use common::model::P2pOptions;
use mono::api::MonoApiServiceState;

pub mod github_router;
mod model;

#[derive(Clone)]
pub struct MegaApiServiceState {
    pub inner: MonoApiServiceState,
    pub port: u16,
    pub p2p: P2pOptions,
}
