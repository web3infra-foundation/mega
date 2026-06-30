//! Ceres: monorepo domain library (transport, application, shared models).

pub mod application;
pub mod bus;
pub mod diff;
pub mod infra;
pub mod lfs;
pub mod merge_checker;
pub mod model;
pub mod transport;

// Legacy module paths (internal + `mono` crate compatibility).
pub mod api_service {
    pub use crate::application::api_service::*;
}
pub mod build_trigger {
    pub use crate::application::build_trigger::*;
}
pub mod code_edit {
    pub use crate::application::code_edit::*;
}
pub mod pack {
    pub use crate::transport::pack::*;
}
pub mod protocol {
    pub use crate::transport::protocol::*;
}

pub use application::api_service::{
    ADMIN_FILE, EffectiveResourcePermission, MonoApiService, MonoAppServices, MonoServiceLogic,
    RefUpdate, TreeUpdateResult, cl_merge,
};
pub use bus::{ApplicationEventHandler, TransportEvent, TransportRuntime};
pub use transport::ProtocolApiState;
