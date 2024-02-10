# Mega - is an unofficial open-source implementation of Google Piper.

Mega is an unofficial open-source implementation of Google Piper. It is a monorepo and monolithic codebase management system that supports Git and Buck2. Mega is designed to manage large-scale codebases, streamline development, and foster collaboration. It is built on top of Rust and PostgreSQL, and is designed to be highly scalable and efficient.

## Motivation

### Monorepo

Monorepo, a unified code repository model, significantly streamlines team development by promoting code sharing and collaboration within a single repository. This approach not only enhances consistency and minimizes duplication but also seamlessly integrates large-scale refactoring and code reviews. By allowing changes across multiple projects to be managed in a single pull request, Monorepo not only bolsters code quality but also expedites the development process. It cultivates a more cohesive and integrated development culture, leading to improved communication and a deeper understanding among team members across various project components. Ultimately, Monorepo boosts efficiency, strengthens consistency, and fosters collaboration in team development.

### Git Compatible

Git is a version control system that distributes file versions across local machines, allowing for quick access and collaboration. While mid-sized companies can store repositories as large as 20TB, managing such extensive codebases can pose challenges. Mega offers the ability to utilize Git with a monorepo. This allows for easy cloning or pulling of any monorepo folder into local filesystem as a Git repository, and seamless pushing of changes back. Despite Git's widespread use, it does not inherently support monorepo structures, but Mega fills this void.

### Trunk-based Development

When it comes to managing large codebases in a centralized manner, trunk-based development is the way to go. This workflow is particularly well-suited for monorepos. The idea behind trunk-based development is to work on a single codebase, making frequent commits and testing regularly. This approach helps identify issues early on, which ultimately leads to greater code stability. Additionally, trunk-based development enables consistency and integration, making it easier to manage monorepos and collaborate effectively on larger projects.

### Decentralized Collaboration

The current open source collaboration landscape, dominated by centralized platforms like GitHub and GitLab, presents a paradox. While Git itself is a decentralized version control system, these platforms bind open-source projects to centralized models. This centralization poses a risk of monopolization in the open-source community, potentially stifling innovation and diversity.

Centralized systems, despite their convenience and popularity, are not without their flaws. One significant concern is the risk of a single point of failure. If a centralized platform experiences downtime or security breaches, it can disrupt the workflow of countless projects and developers relying on it. This vulnerability highlights the need for a more resilient approach to open-source collaboration.

To address these challenges, there's a growing need for a decentralized open-source collaboration model. Such a model would enhance the freedom of communication among developers and strengthen their ownership and control over their code. By moving away from centralized systems, developers can ensure that their contributions and the direction of their projects are not unduly influenced by the policies or stability of a single platform. This shift towards decentralization is not just a technical necessity but a step towards preserving the ethos of open-source: collaboration, freedom, and community-driven development.

## Quick Start for developing and testing

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

   2.  Create database, then find the dump file in the SQL directory of the Mega repository and import it into the database.

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
   
   3. Craeate user and grant privileges.

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

## Contributing

The mega project relies on community contributions and aims to simplify getting started. To develop Mega, clone the repository, then install all dependencies and initialize the database schema, run the test suite and try it out locally. Pick an issue, make changes, and submit a pull request for community review.

More information on contributing to Mega is available in the [Contributing Guide](docs/contributing.md).

## License

Mega is licensed under this Licensed:

- MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
