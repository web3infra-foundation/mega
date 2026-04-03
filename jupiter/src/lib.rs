/// Shared ID generation used with DB / storage paths.
pub use idgenerator;
/// SeaORM — storage layer; dependents may use `jupiter::sea_orm` without a direct `sea-orm` dependency where appropriate.
pub use sea_orm;

pub mod migration;
pub mod model;
pub mod redis;
pub mod service;
pub mod storage;
pub mod utils;
// FIXME: use a global tests module instead
pub mod tests;
