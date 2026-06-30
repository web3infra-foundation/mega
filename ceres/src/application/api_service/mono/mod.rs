//! Monorepo API implementation (`MonoApiService` and domain modules).

pub mod admin;
pub mod logic;
pub mod service;
pub mod types;

pub mod buck;
pub mod cl;
pub mod cla;
pub mod edit;
pub mod label_assignee;
pub mod sync;
pub mod tag;

pub use admin::{ADMIN_FILE, EffectiveResourcePermission};
pub use cl::merge_strategy as cl_merge;
pub use logic::MonoServiceLogic;
pub use service::MonoApiService;
pub use types::{RefUpdate, TreeUpdateResult};
