//! Webhook orchestration: CL event delivery and HTTP admin CRUD.

pub mod admin;
pub mod delivery;

pub use delivery::{
    ClPayload, RepositoryPayload, WebhookDispatcher, WebhookEvent, WebhookPayload,
    dispatch_cl_webhook,
};
