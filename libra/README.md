## Libra

`Libra` is a partial implementation of a **Git** client, developed using **Rust**. Our goal is not to create a 100% replica of Git (for those interested in such a project, please refer to the [gitoxide](https://github.com/Byron/gitoxide)). Instead, `libra` focus on implementing the basic functionalities of Git for learning **Git** and **Rust**. A key feature of `libra` is the replacement of the original **Git** internal storage architecture with **SQLite**.

## Example
```
$ libra --help
Simulates git commands

Usage: libra <COMMAND>

Commands:
  init     Initialize a new repository
  clone    Clone a repository into a new directory
  add      Add file contents to the index
  rm       Remove files from the working tree and from the index
  restore  Restore working tree files
  status   Show the working tree status
  log      Show commit logs
  diff    Show changes between commits, commit and working tree, etc
  branch   List, create, or delete branches
  commit   Record changes to the repository
  switch   Switch branches
  merge    Merge changes
  push     Update remote refs along with associated objects
  fetch    Download objects and refs from another repository
  pull     Fetch from and integrate with another repository or a local branch
  remote   Manage set of tracked repositories
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
## Features
### Clean Code
Our code is designed to be clean and easy to read, 
ensuring that it is both maintainable and understandable for developers of all skill levels.

### Cross-Platform
- [x] Windows
- [x] Linux
- [x] MacOS

### Compatibility with Git
Our implementation is essentially fully compatible with `Git` 
(developed with reference to the `Git` documentation), 
including formats such as `objects`, `index`, `pack`, and `pack-index`. 
Therefore, it can interact seamlessly with `Git` servers (like `push` and `pull`).

### Differences from Git:
While maintaining compatibility with `Git`, we have made some innovations and changes:
we use an `SQLite` database to manage loosely structured files such as `config`, `HEAD`, and `refs`, 
achieving unified management.

## Functions
### Commands
- [x] `init`
- [x] `add`
- [x] `rm`
- [x] `status`
- [x] `stash`
- [x] `commit`
- [x] `log`
- [x] `tag`
- [x] `switch`
- [x] `restore`
- [x] `reset`
- [x] `revert`
- [x] `branch`
- [x] `diff`
- [x] `merge`
- [x] `rebase`
- [x] `reflog`
- [x] `index-pack`
- [x] `remote`
- [x] `lfs`
- [x] `config`
- [x] `checkout`
- [x] `cherry_pick`
#### Remote
- [x] `push`
- [x] `pull`
- [x] `clone`
- [x] `fetch`

### Others
- [x] `.gitignore`
- [x] `.gitattributes` (only for `lfs` now)
- [x] `LFS` (embedded, with p2p feature)
- [ ] `ssh`

## Development
Refs to [Development](../docs/libra/development.md)