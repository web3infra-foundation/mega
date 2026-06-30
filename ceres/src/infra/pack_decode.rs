//! Isolates `axum-core` stream error mapping required by `git-internal` pack decode.
//!
//! `git-internal` still types decode streams with `axum_core::Error`; keep that
//! dependency confined here until upstream accepts `std::io::Error`.

use std::fmt::Display;

/// Maps a pack decode stream error into the type expected by `Pack::decode_stream`.
pub fn map_decode_stream_error<E: Display>(err: E) -> axum_core::Error {
    axum_core::Error::new(std::io::Error::other(err.to_string()))
}
