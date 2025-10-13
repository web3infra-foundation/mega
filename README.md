# Mega - is an monorepo & monolithic codebase management system

Mega is a monorepo & monolithic codebase management system that supports Git, it is designed to manage large-scale codebases, streamline development, and foster collaboration. Mega is an unofficial open source implementation of Google Piper. 

## What's the Piper?

Google Piper is a massive, centralized version control system that Google uses internally to manage their vast codebase. It is a monorepo, and a monolithic which mean is a single repository that contains all the source code for Google's software. It is designed to manage large-scale codebases, streamline development, and foster collaboration. It is built on top of Google's internal infrastructure and is designed to be highly scalable and efficient. More information on the [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository/fulltext).

**Google Piper is not open source**

## Mega features

Mega is an unofficial open source implementation of Google Piper. And it has the following features:

### Git compatible

Mega offers the ability to utilize Git with a monorepo. This allows for easy cloning or pulling of any monorepo folder into local filesystem as a Git repository, and seamless pushing of changes back.

### Trunk-based Development

When it comes to managing large codebases in a centralized manner, trunk-based development is the way to go. More trunk-based Development information on the [Trunk-Based Development](https://trunkbaseddevelopment.com/).

### Conventional Commits

Mono will support conventional commits, which are a set of rules for creating clear and concise commit messages.  More information on the [Conventional Commits](https://www.conventionalcommits.org/).

### FUSE File System of Monorepo

Mega has a component named [scorpio](https://github.com/web3infra-foundation/mega/tree/main/scorpio), which is a FUSE file system for monorepo. Scorpio allows users to mount a monorepo folder as a local filesystem. This enables users to work with their codebase as if it were a local filesystem, while still benefiting from the features of a monorepo. 

### Buck2 Integration

Mega intergrates Buck2 as default, which is a build system developed by Meta with Rust. Buck2 allows users to build their codebase in a declarative manner, and provides a number of features that are not available in other build systems. More information on Buck2 is available in the [Buck2 Documentation](https://buck2.build/).

## Contributing

The mega project relies on community contributions and aims to simplify getting started. To develop Mega, clone the repository, then install all dependencies and initialize the database schema, run the test suite and try it out locally. Pick an issue, make changes, and submit a pull request for community review.

More information on contributing to Mega is available in the [Contributing Guide](docs/contributing.md).

## Chat with us

If you interested in Mega, you can make an appointment with us on [Google Calendar](https://calendar.app.google/QuBf2sdmf68wVYWL7) to discuss your ideas, questions or problems, and we will share our vision and roadmap with you.

## License

Mega is licensed under this Licensed:

- MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
