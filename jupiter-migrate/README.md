# jupiter-migrate

SeaORM database migrations for Mega, extracted from `jupiter` so day-to-day `cargo check` does not compile migration code unless needed.

## Apply migrations

`mono` enables `jupiter/migrate`. On startup, `Storage::new` calls `jupiter_migrate::apply_migrations` automatically (`jupiter/src/storage/init.rs`). No separate `init` CLI step is required.

Crates that need a migrated DB in tests should enable `jupiter/migrate` or `ceres` feature `migrate`.

## Generate a new migration

```bash
cd jupiter-migrate/src/migration
sea-orm-cli migrate generate "your_migration_name"
```

Commit the new file under `jupiter-migrate/src/migration/`.

## Regenerate entities

After schema changes, regenerate callisto entities (adjust connection URL for your DB):

```bash
sea-orm-cli generate entity \
  -u postgres://postgres:postgres@localhost:5432/mono \
  -o jupiter/callisto/src \
  --with-serde both
```

Review generated diffs in `jupiter/callisto/src/` before committing.

## Library API

```rust
use jupiter_migrate::{apply_migrations, Migrator};
```

`apply_migrations(&db, refresh)` runs pending migrations. `Migrator` is the SeaORM migrator trait implementation.
