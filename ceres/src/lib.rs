//! Ceres: monorepo domain library (transport, application, shared models).

pub mod application;
pub mod bus;
pub mod diff;
pub mod infra;
pub mod lfs;
pub mod merge_checker;
pub mod model;
pub mod transport;

pub use application::api_service::{
    ADMIN_FILE, EffectiveResourcePermission, MonoApiService, MonoAppServices, MonoServiceLogic,
    RefUpdate, TreeUpdateResult, cl_merge,
};
pub use bus::{ApplicationEventHandler, TransportEvent, TransportRuntime};
pub use transport::ProtocolApiState;
