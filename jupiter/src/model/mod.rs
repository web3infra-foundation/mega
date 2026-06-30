//! Storage-layer assembly DTOs (bundles of `callisto` entities).
//!
//! These types are internal to Jupiter storage/services. Ceres application code may
//! construct them when calling storage; mono must not import this module.

pub(crate) mod bot_token_dto;
pub mod cl_dto;
pub mod code_review_dto;
pub mod common;
pub(crate) mod conv_dto;
pub mod group_dto;
pub mod issue_dto;
pub mod merge_queue_dto;
pub mod sidebar_dto;
