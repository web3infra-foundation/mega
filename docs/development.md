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

   ```bash
   $ cd mega/sql/postgres
   $ psql mega < pg_YYYYMMDD_init.sql
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

   3.Import `mega/sql/postgres/pg_<time>_init.sql` to `mega`.

   ```bash
   $ cd mega/sql/postgres
   $ sudo -u postgres psql mega < pg_YYYYMMDD__init.sql
   ```

   4.Create user and grant privileges.

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

4. Config `confg.toml`. See [Configuration](#configuration).

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
   $ git remote add mega http://localhost:8000/third-part/linux.git
   $ git push --all mega
   $ sudo rm -r /tmp/linux
   $ cd /tmp
   $ git clone http://localhost:8000/third-part/linux.git
   ```

### GitHub Codespace

If you are using GitHub codespaces, you can follow the steps below to set up the Mega project. When you create a new Codespace, the Mega project will be cloned automatically. You can then follow the steps below to set up the project.

You can skip this step (PostgreSQL setup) if using SQLite in `config.toml`.

When the codespace is ready, the PostgreSQL will be installed and started automatically. You can then follow the steps below to set up the database with below steps.

```bash
## Start PostgreSQL
/etc/init.d/postgresql start

sudo -u postgres psql mega -c "CREATE DATABASE mega;"
sudo -u postgres psql mega < /workspaces/mega/sql/pg_YYYYMMDD__init.sql
sudo -u postgres psql mega -c "CREATE USER mega WITH ENCRYPTED PASSWORD 'mega';"
sudo -u postgres psql mega -c "GRANT ALL PRIVILEGES ON DATABASE mega TO mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL FUNCTIONS IN SCHEMA public to mega;"
```

Config `confg.toml` file for the Mega project.

---
## Configuration
Config `confg.toml` file for the Mega project.

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

```toml
# The directory where the data files is located, such as logs, database, etc.
# can be overrided by environment variable `MEGA_BASE_DIR`
base_dir = "/tmp/.mega"

# Filling the following environment variables with values you set
## Logging Configuration
[log]
# The path which log file is saved
log_path = "${base_dir}/logs"

# log level
level = "debug"

# print std log in console, disable it on production for performance
print_std = true


[database]
# "sqlite" | "postgres"
# "sqlite" will use `db_path` and ignore `db_url`
db_type = "sqlite"

# used for sqlite
db_path = "${base_dir}/mega.db"

# database connection url
db_url = "postgres://mega:mega@localhost:5432/mega"

# db max connection, setting it to twice the number of CPU cores would be appropriate.
max_connection = 32

# db min connection, setting it to the number of CPU cores would be appropriate.
min_connection = 16

# Whether to disabling SQLx Log
sqlx_logging = false


[storage]

obs_access_key = ""
obs_secret_key = ""

# cloud storage region
obs_region = "cn-east-3"

# Override the endpoint URL used for remote storage services
obs_endpoint = "https://obs.cn-east-3.myhuaweicloud.com"


[monorepo]
## Only import directory support multi-branch commit and tag, monorepo only support main branch
## Mega treats files under this directory as import repo and other directories as monorepo
import_dir = "/third-part"

disable_http_push = false

[pack]
# The maximum memory used by decode
# Support the following units/notations: K, M, G, T, KB, MB, GB, TB, KiB, MiB, GiB, TiB, `%` and decimal percentages
# Capacity units are case-insensitive and can also be spelled as mb or Mb
# Abbreviated units are treated as binary byte units, for example M is treated as MiB
pack_decode_mem_size = "4G"
pack_decode_disk_size = "20%"

# The location where the object stored when the memory used by decode exceeds the limit
pack_decode_cache_path = "${base_dir}/cache"

clean_cache_after_decode = true

# The maximum meesage size in channel buffer while decode
channel_message_size = 1_000_000


[ztm]
ca = "http://127.0.0.1:9999"
hub = "http://127.0.0.1:8888"
agent = "http://127.0.0.1:7777"

[lfs]
# LFS Server url
url = "https://git.gitmono.com"

# set the local path of the lfs storage
lfs_obj_local_path = "${base_dir}/lfs"

## IMPORTANT: The 'enable_split' feature can only be enabled for new databases. Existing databases do not support this feature.
# Enable or disable splitting large files into smaller chunks
enable_split = false  # Default is disabled. Set to true to enable file splitting.

# Size of each file chunk when splitting is enabled, in bytes. Ignored if splitting is disabled.
split_size = 20971520 # Default size is 20MB (20971520 bytes)
```

---
## Database maintenance
Currently, the tables of database are created by `.sql` file. 

If you want to add a new table or modify the existing table, you need to update the `.sql` files which are located in the `sql` directory.

### Attention
- Each database corresponds to one `.sql` file, you must modify all of them if you want to update the tables in order to keep the consistency of the database.
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
