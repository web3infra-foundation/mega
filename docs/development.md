# Development

## Architect

![Mega Architect](images/architect.svg)

## Quick start manuel to developing or testing

### MacOS

1. Install Rust on your macOS machine.

   ```bash
   $ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone mega repository and build it.

   ```bash
   $ git clone https://github.com/web3infra-foundation/mega.git
   $ cd mega
   $ git submodule update --init --recursive
   $ cargo build
   ```

3. Install PostgreSQL and init database. (You can skip this step if using SQLite in `config.toml`)

   1. Install PostgreSQL 16 with `brew` command.

   ```bash
   $ brew install postgresql@16
   $ echo 'export PATH="/opt/homebrew/opt/postgresql@16/bin:$PATH"' >> ~/.zshrc
   $ brew services start postgresql@16
   $ initdb /Volumes/Data/postgres -E utf8 # /Volumes/Data is path store data
   ```

   2. Create a database, then find the dump file in the SQL directory of the Mega repository and import it into the database.

   ```bash
   $ psql postgres
   ```

   ```sql
   postgres=# \l
   postgres=# DROP DATABASE IF EXISTS mega;
   postgres=# CREATE DATABASE mega;
   postgres=# \q
   ```


   3. Create user and grant privileges.

   ```sql
   postgres=# DROP USER IF EXISTS mega;
   postgres=# CREATE USER mega WITH ENCRYPTED PASSWORD 'mega';
   postgres=# GRANT ALL PRIVILEGES ON DATABASE mega TO mega;
   ```

   ```bash
   $ psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
   $ psql mega -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public to mega;"
   $ psql mega -c "GRANT ALL ON ALL FUNCTIONS IN SCHEMA public to mega;"
   ```

4. Update config file for local test. For local testing, Mega uses the `config.toml` file to configure the required parameters. See [Configuration](#configuration).

5. Init the Mega

   ```bash
   $ cd mega
   $ cargo run init
   ```

6. Start the Mega server for testing.

   ```bash
   # Starting a single http server
   $ cargo run service http
   # Or Starting multiple server
   $ cargo run service multi http ssh
   ```

7. Test the `git push` and `git clone`

   ```bash
   $ cd mega
   $ git remote add local http://localhost:8000/projects/mega.git
   $ git push local main
   $ cd /tmp
   $ git clone http://localhost:8000/projects/mega.git
   ```

### Arch Linux

1. Install Rust.

   ```bash
   $ pacman -S rustup
   $ rustup default stable
   ```

2. Clone mega repository and build.

   ```bash
   $ git clone https://github.com/web3infra-foundation/mega.git
   $ cd mega
   $ git submodule update --init --recursive
   $ cargo build
   ```

3. Install PostgreSQL and initialize database. (You can skip this step if using SQLite in `config.toml`)

   1.Install PostgreSQL.

   ```bash
   $ pacman -S postgresql
   # Switch to `postgres` user
   $ sudo -i -u postgres
   postgres $ initdb -D /var/lib/postgres/data -E utf8 # /Volumes/Data is where data will be stored
   postgres $ exit
   $ systemctl enable --now postgresql
   ```

   2.Create database.

   ```bash
   $ sudo -u postgres psql postgres
   ```

   ```sql
   postgres=# \l
   postgres=# DROP DATABASE IF EXISTS mega;
   postgres=# CREATE DATABASE mega;
   postgres=# \q
   ```

   3.Create user and grant privileges.

   ```sql
   $ sudo -u postgres psql postgres
   postgres=# DROP USER IF EXISTS mega;
   postgres=# CREATE USER mega WITH ENCRYPTED PASSWORD 'mega';
   postgres=# GRANT ALL PRIVILEGES ON DATABASE mega TO mega;
   ```

   ```bash
   $ sudo -u postgres psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
   $ sudo -u postgres psql mega -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public to mega;"
   $ sudo -u postgres psql mega -c "GRANT ALL ON ALL FUNCTIONS IN SCHEMA public to mega;"
   ```

4. Config `config.toml`. See [Configuration](#configuration).

5. Init Mega.

   ```bash
   $ cd mega
   $ cargo run init
   ```

6. Start Mega server.

   ```bash
   # Start a single https server
   $ cargo run service http
   # Or Start multiple server
   $ cargo run service multi http ssh
   ```

7. Test `git push` and `git clone`

   ```bash
   $ cd /tmp
   $ git clone https://github.com/Rust-for-Linux/linux.git
   $ cd linux
   $ git remote add mega http://localhost:8000/third-party/linux.git
   $ git push --all mega
   $ sudo rm -r /tmp/linux
   $ cd /tmp
   $ git clone http://localhost:8000/third-party/linux.git
   ```

### GitHub Codespace

If you are using GitHub codespaces, you can follow the steps below to set up the Mega project. When you create a new Codespace, the Mega project will be cloned automatically. You can then follow the steps below to set up the project.

You can skip this step (PostgreSQL setup) if using SQLite in `config.toml`.

When the codespace is ready, the PostgreSQL will be installed and started automatically. You can then follow the steps below to set up the database with below steps.

```bash
## Start PostgreSQL
/etc/init.d/postgresql start

sudo -u postgres psql mega -c "CREATE DATABASE mega;"
sudo -u postgres psql mega -c "CREATE USER mega WITH ENCRYPTED PASSWORD 'mega';"
sudo -u postgres psql mega -c "GRANT ALL PRIVILEGES ON DATABASE mega TO mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL FUNCTIONS IN SCHEMA public to mega;"
```

---
## Configuration
Setting `config.toml` file for the Mega project, default config file can be found under [config directory](/config/config.toml).

Currently, the mono bin and mega bin use two different files, each with a different default database type: mono uses `Postgres`, while mega uses `SQLite`.

### Path
- Default: automatically load `config.toml` in current directory.
- Specify manually: use `--config "/path/to/config.toml"`

### Enhance
- You can use environment variables starting with `MEGA_` to override the configuration in `config.toml`.
  - like `MEGA_BASE_DIR` to override `base_dir`. // with `env::set_var()`
  - use separator `__` (2 \* `_`) for nested keys, like `MEGA_LOG__LOG_PATH` for `log.log_path`.
- Support `${}` syntax to reference other keys in the same file.
  - like `log_path = "${base_dir}/logs"`, `${base_dir}` will be replaced by the value of `base_dir`
  - or `key = "${xxx.yyy}/zzz"` (prefix `xxx.` can't be omitted)
  - only support `String` type
  - substitute from up to down
  - see codes in [config.rs](/common/src/config.rs)

---

### Attention
- DO NOT use `Array` Type in PostgreSQL but use `JSON` instead, for compatibility with SQLite & MySQL. (`JSON` <==> `serde_json::Value`)
---
## Tests
> Keep in mind that it's impossible to find all bugs.
> 
> Tests are the last line of defense.

### Unit Tests

Unit tests are small, focused tests that verify the behavior of a single function or module in isolation.

#### Example:

```rust
// ...Other Codes

#[cfg(test)] // indicates this block will only be compiled when running tests
mod tests {
   use super::*;

   #[test] // indicates that this function is a test, which will be run by `cargo test`
   fn test_add() {
      let result = add(1, 1);
      assert_eq!(result, 2); // assert is important to tests
   }
}
```

### Integration Tests
Integration tests verify that different parts of your **library** work correctly together.
They are **external** to your crate and use your code in the same way any other code would.

#### Steps
You can refer to the implementation of the mega **module**. ([mega/tests](/mega/tests))
1. Create a `tests` directory at the **same level** as your `src` directory (e.g. `libra/tests`).
2. Add `*.rs` files in this directory. // Each file will be compiled as a separate **crate**.

#### Attention
- The `tests` in **root** directory (workspace) is NOT integration tests, but some `data` for other tests.
- If you need a common module, use `tests/common/mod.rs` rather than `tests/common.rs`, to declare it's not a test file.
- There is no need to add `#[cfg(test)]` to the `tests` directory. `tests` will be compiled only when running tests.

#### Run integration tests
The following command will be executed in `GitHub Actions`.

This command DOES NOT run **Unit Tests** (which could be very messy).
```bash
cargo test --workspace --test '*' -- --nocapture
```
- `--workspace` : Run tests for **all packages** in the workspace.
- `--test` : Test the specified **integration test**.
- `--` : Pass the following arguments to the test binary.
- `--nocapture` : DO NOT capture the output (e.g. `println!`) of the test.

If you want to run tests in a specific package, you can use `--package`.

For more information, please refer to the [rust wiki](https://rustwiki.org/zh-CN/cargo/commands/cargo-test.html).

---
## Comment Guideline

This guide outlines the recommended order for importing dependencies in Rust projects.

### File Header Comments (//!)

### Struct Comments (///)

### Function Comments (///)

---
## Rust Dependency Import Order Guideline

This guide outlines the recommended order for importing dependencies in Rust projects.

#### 1. Rust Standard Library

Import dependencies from the Rust standard library.

#### 2. Third-Party Crates

Import dependencies from third-party crates.

#### 3. Other Modules in Workspace

Import dependencies from other modules within the project workspace.

#### 4. Within Modules

Import functions and structs from within modules.

Example:

```rust

// 1. Rust Standard Library
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

// 2. Third-Party Crates
use bytes::{BufMut, Bytes, BytesMut};
use russh::server::{self, Auth, Msg, Session};
use russh::{Channel, ChannelId};
use russh_keys::key;
use tokio::io::{AsyncReadExt, BufReader};

// 3. Other Modules in Workspace
use storage::driver::database::storage::ObjectStorage;

// 4. Other Files in the Same Module
use crate::protocol::pack::{self};
use crate::protocol::ServiceType;
use crate::protocol::{PackProtocol, Protocol};
```

### Additional Notes:

- Always group imports with an empty line between different sections for better readability.
- Alphabetize imports within each section to maintain consistency.
- Avoid using extern crate syntax for Rust 2018 edition and later; prefer using use with crates.
- Do not use `super::` and `self::` in imports. It can lead to ambiguity and hinder code readability. Instead, use crate to reference the current crate's modules.
