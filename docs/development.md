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
   $ cargo build
   ```

3. Install PostgreSQL and init database.

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
   $ psql mega < pg_20240205_init.sql
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

4. Install redis.

   ```bash
   $ brew install redis
   $ brew services start redis
   ```

5. Update config file for local test. For local testing, Mega uses the `confile.toml` file to configure the required parameters.

   ```ini
    # Fillin the following environment variables with values you set

    ## Database Configuration
    DB = "postgres" # {postgres}
    DB_USERNAME = "mega"
    DB_PASSWORD = "mega"
    DB_HOST = "localhost"

    MEGA_DB_POSTGRESQL_URL = "${DB}://${DB_USERNAME}:${DB_PASSWORD}@${DB_HOST}/mega"
    MEGA_DB_MAX_CONNECTIONS = 32
    MEGA_DB_MIN_CONNECTIONS = 16

    MEGA_DB_SQLX_LOGGING = false # Whether to disabling SQLx Log
    ## End Database Configuration


    ## SSH/HTTPS Key Configruation
    MEGA_SSH_KEY = "/tmp/.mega/ssh"
    MEGA_HTTPS_PUBLIC_KEY = ""
    MEGA_HTTPS_PRIVATE_KEY = ""
    ## End SSH/HTTPS Key Configruation

    ## File Storage Configuration
    MEGA_RAW_STORAGE = "LOCAL" # LOCAL or REMOTE

    ### This configuration is used to set the local path of the project storage
    MEGA_OBJ_LOCAL_PATH = "/tmp/.mega/objects"
    MEGA_LFS_OBJ_LOCAL_PATH = "/tmp/.mega/lfs"

    ### This configuration is used to set the object storage service like S3
    MEGA_OBS_ACCESS_KEY = ""
    MEGA_OBS_SECRET_KEY = ""
    MEGA_OBJ_REMOTE_REGION = "cn-east-3" # Remote cloud storage region
    MEGA_OBJ_REMOTE_ENDPOINT = "https://obs.cn-east-3.myhuaweicloud.com" # Override the endpoint URL used for remote storage services

    ## If the object file size exceeds the threshold value, it will be handled by file storage instead of the database
    MEGA_BIG_OBJ_THRESHOLD_SIZE = 1024 # Unit KB.

    ## Only import directory support multi-branch commit and tag, repo under regular directory only support main branch only
    MEGA_IMPORT_DIRS = "/third-part"

    ## Decode cache configuration
    MEGA_PACK_DECODE_MEM_SIZE = 4 # Unit GB.
    MEGA_PACK_DECODE_CACHE_PATH = "/tmp/.mega/cache"
    CLEAN_CACHE_AFTER_DECODE = true

   ```

6. Init the Mega

   ```bash
   $ cd mega
   $ cargo run init -d postgres
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

   1. Install PostgreSQL.

   ```bash
   $ pacman -S postgresql
   # Switch to `postgres` user
   $ sudo -i -u postgres
   postgres $ initdb -D /var/lib/postgres/data -E utf8 # /Volumes/Data is where data will be stored
   postgres $ exit
   $ systemctl enable --now postgresql
   ```

   2. Create database.

   ```bash
   $ sudo -u postgres psql postgres
   ```

   ```sql
   postgres=# \l
   postgres=# DROP DATABASE IF EXISTS mega;
   postgres=# CREATE DATABASE mega;
   postgres=# \q
   ```

   3. Import `mega/sql/postgres/pg_<time>_init.sql` to `mega`.

   ```bash
   $ cd mega/sql/postgres
   $ sudo -u postgres psql mega < pg_20240205__init.sql
   ```

   4. Create user and grant privileges.

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

4. Install redis.

   ```bash
   $ pacman -S redis
   $ systemctl enable --now redis
   ```

5. Config `confile.toml`.

   ```ini
    # Fillin the following environment variables with values you set

      ## Logging Configuration
      [log]
      # The path which log file is saved
      log_path = "/tmp/.mega/logs"

      # log level
      level = "debug"

      # print std log in console, disable it on production for performance
      print_std = true


      [database]
      # database connection url
      db_url = "postgres://postgres:postgres@localhost:5432/mega"

      # db max connection, setting it to twice the number of CPU cores would be appropriate.
      max_connection = 32

      # db min connection, setting it to the number of CPU cores would be appropriate.
      min_connection = 16

      # Whether to disabling SQLx Log
      sqlx_logging = false


      [ssh]
      ssh_key_path = "/tmp/.mega/ssh"

      [storage]
      # raw object stroage type, can be `local` or `remote`
      raw_obj_storage_type = "LOCAL"

      ## If the object file size exceeds the threshold value, it will be handled by file storage instead of the database, Unit is KB
      big_obj_threshold = 1024

      # set the local path of the project storage
      raw_obj_local_path = "/tmp/.mega/objects"

      lfs_obj_local_path = "/tmp/.mega/lfs"

      obs_access_key = ""
      obs_secret_key = ""

      # cloud storage region
      obs_region = "cn-east-3"

      # Override the endpoint URL used for remote storage services
      obs_endpoint = "https://obs.cn-east-3.myhuaweicloud.com"


      [monorepo]
      ## Only import directory support multi-branch commit and tag, repo under regular directory only support main branch only
      ## Mega treats files in that directory as import repo and other directories as monorepo
      import_dir = "/third-part"


      # The maximum memory used by decode, Unit is GB
      pack_decode_mem_size = 4

      # The location where the object stored when the memory used by decode exceeds the limit
      pack_decode_cache_path = "/tmp/.mega/cache"

      clean_cache_after_decode = true


   ```

6. Init Mega.

   ```bash
   $ cd mega
   $ cargo run init -d postgres
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
   $ cd /tmp
   $ git clone https://github.com/Rust-for-Linux/linux.git
   $ cd linux
   $ git remote add mega http://localhost:8000/third-part/linux.git
   $ git push --all mega
   $ sudo rm -r /tmp/linux
   $ cd /tmp
   $ git clone http://localhost:8000/third-part/linux.git
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
