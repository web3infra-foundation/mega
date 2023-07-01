# Mega - Monorepo Engine for Enterprise and Individual

Mega is an engine for managing a monorepo. It functions similarly to Google's Piper and helps to streamline Git and trunk-based development for large-scale projects.

## Git Compatible

Git is a version control system that distributes file versions across local machines, allowing for quick access and collaboration. While mid-sized companies can store repositories as large as 20TB, managing such extensive codebases can pose challenges. Mega offers the ability to utilize Git with a monorepo. This allows for easy cloning or pulling of any monorepo folder into local filesystem as a Git repository, and seamless pushing of changes back. Despite Git's widespread use, it does not inherently support monorepo structures, but Mega fills this void.

## Trunk-based Development

When it comes to managing large codebases in a centralized manner, trunk-based development is the way to go. This workflow is particularly well-suited for monorepos. The idea behind trunk-based development is to work on a single codebase, making frequent commits and testing regularly. This approach helps identify issues early on, which ultimately leads to greater code stability. Additionally, trunk-based development enables consistency and integration, making it easier to manage monorepos and collaborate effectively on larger projects.

## Getting Started

Coming soon...

## Contributing

The mega project relies on community contributions and aims to simplify getting started. To develop Mega, clone the repository, then install all dependencies and initialize the database schema, run the test suite and try it out locally. Pick an issue, make changes, and submit a pull request for community review.

More information on contributing to Mega is available in the [Contributing Guide](docs/contributing.md).

## License

Mega is licensed under this Licensed:

- MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)