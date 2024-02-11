# Development

## Architect

![Mega Architect](images/mega-architect.png)

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
   $ cargo build
   ```

3. Install PostgreSQL and init database.

    1.  Install PostgreSQL 16 with `brew` command.

   ```bash
   $ brew install postgresql@16
   $ echo 'export PATH="/opt/homebrew/opt/postgresql@16/bin:$PATH"' >> ~/.zshrc
   $ brew services start postgresql@16
   $ initdb /Volumes/Data/postgres -E utf8 # /Volumes/Data is path store data
   ```

    2.  Create a database, then find the dump file in the SQL directory of the Mega repository and import it into the database.

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
   $ psql mega < pg_20231106__init.sql
   ```

    3. Create user and grant privileges.

   ```sql
   postgres=# DROP USER IF EXISTS mega;
   postgres=# CREATE USER mega WITH ENCRYPTED PASSWORD 'rustgit';
   postgres=# GRANT ALL PRIVILEGES ON DATABASE mega TO mega;
   ```

   ```bash
   $ psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
   $ psql mega -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public to mega;"
   $ psql mega -c "GRANT ALL ON ALL FUNCTIONS IN SCHEMA public to mega;"
   ```

4. Install redis.

   ```bash
   $ brew install redis
   $ brew services start redis
   ```

5. Config environment variables for local test. For local testing, Mega uses the .env file to configure the required parameters. However, before starting the project, you also need to configure the environment variables such as `DB_USERNAME`, `DB_SECRET`, and `DB_HOST`.

   ```ini
   MEGA_DB_POSTGRESQL_URL = "postgres://mega:rustgit@127.0.0.1/mega"
   MEGA_DB_MAX_CONNECTIONS = 32
   MEGA_DB_MIN_CONNECTIONS = 16

   MEGA_DB_SQLX_LOGGING = false # Whether to disabling SQLx Log

   ## file storage configuration
   MEGA_OBJ_STORAGR_TYPE = "LOCAL" # LOCAL or REMOTE
   MEGA_OBJ_LOCAL_PATH = "/tmp/.mega" # This configuration is used to set the local path of the project storage

   MEGA_BIG_OBJ_THRESHOLD_SIZE = 1024 # Unit KB. If the object file size exceeds the threshold value, it will be handled by file storage instead of the database.

   ## Init directory configuration
   MEGA_INIT_DIRS = "projects,docs,third_parts" # init these repo directories in mega init command
   MEGA_IMPORT_DIRS = "third_parts" # Only import directory support multi-branch commit and tag, repo under regular directory only support main branch only


   GIT_INTERNAL_DECODE_CACHE_SIZE = 100 # Maximum number of git objects in LRU cache
   GIT_INTERNAL_DECODE_STORAGE_BATCH_SIZE = 1000 # The maximum number of git object in a "INSERT" SQL database operation
   GIT_INTERNAL_DECODE_STORAGE_TQUEUE_SIZE = 1 # The maximum number of parallel insertion threads in the database operation queue
   GIT_INTERNAL_DECODE_CACHE_TYEP = "redis" # {lru,redis}
   REDIS_CONFIG = "redis://127.0.0.1:6379"

   ## Bazel build config, you can use service like buildfarm to enable RBE(remote build execution)
   # you can refer to https://bazelbuild.github.io/bazel-buildfarm/docs/quick_start/ for more details about remote executor
   BAZEL_BUILD_ENABLE = false # leave true if you want to trigger bazel build in each push process
   BAZEL_BUILDP_PATH = "/tmp/.mega/bazel_build_projects" # Specify a temporary directory to build the project with bazel
   BAZEL_REMOTE_EXECUTOR = "grpc://localhost:8980" # If enable the remote executor, please fillin the remote executor address, or else leave empty if you want to build by localhost. 
   BAZEL_GIT_CLONE_URL = "http://localhost:8000" # Tell bazel to clone the project from the specified git url
   ```

6. Init the Mega

   ```bash
   $ cd mega
   $ cargo run init
   ```

7. Start the Mega server for testing.

   ```bash
   # Starting a single https server
   $ cargo run service https 
   # Or Starting multiple server
   $ cargo run service start http ssh p2p
   ```

8. Test the `git push` and `git clone`

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
   $ cargo build
   ```

3. Install PostgreSQL and initialize database.

    1.  Install PostgreSQL.

   ```bash
   $ pacman -S postgresql
   # Switch to `postgres` user
   $ sudo -i -u postgres
   postgres $ initdb -D /var/lib/postgres/data -E utf8 # /Volumes/Data is where data will be stored
   postgres $ exit
   $ systemctl enable --now postgresql
   ```

    2.  Create database.

   ```bash
   $ sudo -u postgres psql postgres
   ```

   ```sql
   postgres=# \l
   postgres=# DROP DATABASE IF EXISTS mega;
   postgres=# CREATE DATABASE mega;
   postgres=# \q
   ```

    3.  Import `mega/sql/postgres/pg_<time>_init.sql` to `mega`.

   ```bash
   $ cd mega/sql/postgres
   $ psql mega < pg_<time>__init.sql
   ```

    4. Craeate user and grant privileges.

   ```sql
   postgres=# DROP USER IF EXISTS mega;
   postgres=# CREATE USER mega WITH ENCRYPTED PASSWORD 'rustgit';
   postgres=# GRANT ALL PRIVILEGES ON DATABASE mega TO mega;
   ```

   ```bash
   $ psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
   $ psql mega -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public to mega;"
   $ psql mega -c "GRANT ALL ON ALL FUNCTIONS IN SCHEMA public to mega;"
   ```

4. Install redis.

   ```bash
   $ pacman -S redis
   $ systemctl enable --now redis
   ```

5. Config `.env`.

   ```ini
   # If you followed the installation guide, you can use below URL directly, comment it the otherwise.
   MEGA_DB_POSTGRESQL_URL = "postgres://mega:rustgit@127.0.0.1/mega"
   # If you changed any of the username, password or host, you will need to uncomment the following line and replace the placeholders manually.
   #MEGA_DB_POSTGRESQL_URL = "postgres://<username>:<password>@127.0.0.1/<db_name (or host)>"
   MEGA_DB_MAX_CONNECTIONS = 32
   MEGA_DB_MIN_CONNECTIONS = 16

   MEGA_DB_SQLX_LOGGING = false # Whether to disabling SQLx Log

   ## file storage configuration
   MEGA_OBJ_STORAGR_TYPE = "LOCAL" # LOCAL or REMOTE
   MEGA_OBJ_LOCAL_PATH = "/tmp/.mega" # This configuration is used to set the local path of the project storage

   MEGA_BIG_OBJ_THRESHOLD_SIZE = 1024 # Unit KB. If the object file size exceeds the threshold value, it will be handled by file storage instead of the database.

   ## Init directory configuration
   MEGA_INIT_DIRS = "projects,docs,third_parts" # init these repo directories in mega init command
   MEGA_IMPORT_DIRS = "third_parts" # Only import directory support multi-branch commit and tag, repo under regular directory only support main branch only


   GIT_INTERNAL_DECODE_CACHE_SIZE = 100 # Maximum number of git objects in LRU cache
   GIT_INTERNAL_DECODE_STORAGE_BATCH_SIZE = 1000 # The maximum number of git object in a "INSERT" SQL database operation
   GIT_INTERNAL_DECODE_STORAGE_TQUEUE_SIZE = 1 # The maximum number of parallel insertion threads in the database operation queue
   GIT_INTERNAL_DECODE_CACHE_TYEP = "redis" # {lru,redis}
   REDIS_CONFIG = "redis://127.0.0.1:6379"

   ## Bazel build config, you can use service like buildfarm to enable RBE(remote build execution)
   # you can refer to https://bazelbuild.github.io/bazel-buildfarm/docs/quick_start/ for more details about remote executor
   BAZEL_BUILD_ENABLE = false # leave true if you want to trigger bazel build in each push process
   BAZEL_BUILDP_PATH = "/tmp/.mega/bazel_build_projects" # Specify a temporary directory to build the project with bazel
   BAZEL_REMOTE_EXECUTOR = "grpc://localhost:8980" # If enable the remote executor, please fillin the remote executor address, or else leave empty if you want to build by localhost. 
   BAZEL_GIT_CLONE_URL = "http://localhost:8000" # Tell bazel to clone the project from the specified git url
   ```

6. Init Mega.

   ```bash
   $ cd mega
   $ cargo run init
   ```

7. Start Mega server.

   ```bash
   # Start a single https server
   $ cargo run service https 
   # Or Start multiple server
   $ cargo run service start http ssh p2p
   ```

8. Test `git push` and `git clone`

   ```bash
   $ cd mega
   $ git remote add local http://localhost:8000/projects/mega.git
   $ git push local main
   $ cd /tmp
   $ git clone http://localhost:8000/projects/mega.git
   ```

## Comment Guideline

This guide outlines the recommended order for importing dependencies in Rust projects.

### File Header Comments (//!)

### Struct Comments (///)

### Function Comments (///)


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
