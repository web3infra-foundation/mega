//! SeaORM database migrations for Mega (extracted from jupiter for faster `cargo check`).

pub mod migration;

pub use migration::{Migrator, apply_migrations};
