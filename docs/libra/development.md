# Libra Development

In theory, libra should run on all platforms that support rust and sqlite.

## storage

Libra store project's data in `.libra` directory in the root of the project.

Libra use sqlite to store some information, such as config, HEAD, refs, which are files in git. However, we keep `index file` and `objects` in the file system, which is the same as git.

The structure of `.libra` directory:

```bash
.libra/
├── libra.db
└── objects
    ├── xx
    │   └── 9dda22e9f8a653838120287d1813305be6cfb3
    ├── info
    └── pack
```

## Data Model

### Database

Libra use `sea-orm` to interact with sqlite database. The data model is defined in `libra/src/internal/model`, with two tables: `config` and `reference`.

The `config` table is used to store the configuration of the project, which corresponds to the `config` file in git. The `reference` table is used to store the reference of the project, which corresponds to the `HEAD` and `refs/*` files in git.

The relationship between the git file and the sqlite table is as follows:

-   **reference**:

| Category                              | Description                                                    | Database format                                                                                                                                         |
| ------------------------------------- | :------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.git/HEAD `                          | Current head pointer (branch or commit hash)                   | - Reference(name=<branch>; kind=HEAD;commit=null;remote=null) <br />- Reference(name=null; kind=HEAD;commit=<commit hash>;remote=null)                  |
| `.git/refs/heads/<branch>`            | Branch name, can’t be “HEAD”                                   | - Reference(name=<branch>, kind=Branch; commit=<commit hash>; remote=null)                                                                              |
| `.git/refs/tags/<tag> `               | Similar to branch                                              | - Reference(name=<tag>, kind=Tag; commit=<commit hash>; remote=null)                                                                                    |
| `.git/refs/remotes/<remote>/<branch>` | Contains branch heads and HEAD*Remote HEAD can’t be detached.* | - Reference(name=<branch>, kind=Branch; commit=<commit hash>; remote=<remote>)<br />- Reference(name=<branch>; type=HEAD; commit=null; remote=<remote>) |

-   **config**:

```ini
# config Example
[core]
        filemode = true
        ignorecase = false
[remote "origin"]
        url = url.git
        fetch = +refs/heads……
```

| Category     | Description              | Database format                                                                                                                                                                                                                                                                                                            |
| ------------ | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.gitconfig` | Ini format configuration | - Config(configuration=”core”; name=null; key=”filemode”;value=”true” )<br />- Config(configuration=”core”; name=null; key=”ignorecase”;value=”false” ) <br />- Config(configuration=”remote”; name=”origin”; key=”url”;value=”url.git” )<br />- Config(configuration=”remote”; name=”null”; key=”fetch”;value=”+refs……” ) |

### Business Model Design

For decoupling, the `sea-orm` model is not directly used in the code, and the business model is redefined (located in `libra/src/internal`), and common CRUD operations are implemented.

Currently, the `config`, `head` and `reference` models are implemented.

### Git Model

Libra use mega's shared model to interact with git objects. The model is defined in `mercury/src/internal`.

The model is used to interact with the git object, such as `commit`, `tree`, `blob`, `tag`, `index`, `pack`, `pack-index`, etc.

Libar use interface in `libra/src/command/mod.rs` to process git object's read and write.

## Add new command

Libra use `clap` to parse the command line arguments. And each subcommand define it's own struct to parse the arguments. The arguments match is defined in `libra/src/main.rs`.

Use `libra push` as an example.

1. Add a new file in `libra/src/command` named `push.rs`, and add it to the `mod.rs` file.

2. Define the subcommand struct and implement the `Parse` trait.

```rust
#[derive(Parser, Debug)]
pub struct PushArgs {
    #[clap(requires("refspec"))]
    repository: Option<String>,
    #[clap(requires("repository"))]
    refspec: Option<String>,
}
```

3. Define the funtion to handle the subcommand, usually named `execute`

```rust
pub async fn execute(args: PushArgs){
    unimplemented!()
}
```

4. Add the subcommand to the `Command` enum in `libra/src/main.rs`

```rust
#[derive(Subcommand, Debug)]
enum Commands {
    // ......
    #[command(about = "Update remote refs along with associated objects")]
    Push(command::push::PushArgs),
}
```

5. Add the subcommand to the `match` in `libra/src/main.rs`

```rust
    // parse the command and execute the corresponding function with it's args
    match args.command {
        // ......
        Commands::Push(args) => command::push::execute(args).await,
    }
```
