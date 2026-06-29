//! Stateless monorepo path/tree/ref helpers for [`MonoApiService`](super::service::MonoApiService).

mod path;
mod tree;

pub(crate) use path::path_not_exist_re;

/// Stateless logic helpers for monorepo operations (easy to unit test).
pub struct MonoServiceLogic;
