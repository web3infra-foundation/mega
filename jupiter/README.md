## Jupiter Module - Monorepo and Mega Database Storage Engine

### Migration Guideline
1. Generate new migration

```bash

cd mega/jupiter/src

sea-orm-cli migrate generate "your_migration_name"
```

2. Generate entity files

```bash

cd mega/jupiter/src

sea-orm-cli generate entity -u postgres://postgres:postgres@localhost:5432/mono -o ../callisto/src --with-serde both

```

3. Running Migrator CLI

- Generate a new migration file
    ```sh
    cargo run -- generate MIGRATION_NAME
    ```
- Apply all pending migrations
    ```sh
    cargo run
    ```
    ```sh
    cargo run -- up
    ```
- Apply first 10 pending migrations
    ```sh
    cargo run -- up -n 10
    ```
- Rollback last applied migrations
    ```sh
    cargo run -- down
    ```
- Rollback last 10 applied migrations
    ```sh
    cargo run -- down -n 10
    ```
- Drop all tables from the database, then reapply all migrations
    ```sh
    cargo run -- fresh
    ```
- Rollback all applied migrations, then reapply all migrations
    ```sh
    cargo run -- refresh
    ```
- Rollback all applied migrations
    ```sh
    cargo run -- reset
    ```
- Check the status of all migrations
    ```sh
    cargo run -- status
    ```
