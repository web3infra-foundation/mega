## Libra
`libra` is a `Git` Client in `Rust`.

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
- [x] `commit`
- [x] `log`
- [ ] `tag`
- [x] `switch`
- [x] `restore`
- [ ] `reset`
- [x] `branch`
- [ ] `diff`
- [x] `merge`
- [ ] `rebase`
- [x] `index-pack`
- [x] `remote`
- [ ] `config`
- [x] `push`
- [x] `pull`
- [x] `clone`
- [x] `fetch`

### Others
- [ ] `.gitignore` and `.gitattributes`