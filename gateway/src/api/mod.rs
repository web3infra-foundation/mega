use common::model::P2pOptions;
use mono::api::MonoApiServiceState;

pub mod commit;
pub mod github_router;
mod model;

#[derive(Clone)]
pub struct MegaApiServiceState {
    pub inner: MonoApiServiceState,
    pub p2p: P2pOptions,
}
