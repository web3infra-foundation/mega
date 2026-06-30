//! Transport-layer shared state (storage, cache, application event handler).

pub use crate::bus::TransportRuntime;

/// Backward-compatible alias for [`TransportRuntime`].
pub type ProtocolApiState = TransportRuntime;
