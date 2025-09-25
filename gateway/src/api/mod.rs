use mono::api::MonoApiServiceState;

pub mod github_router;
mod model;

#[derive(Clone)]
pub struct MegaApiServiceState {
    pub inner: MonoApiServiceState,
}
