# Mega - is an unofficial open source implementation of Google Piper.

Mega is an unofficial open source implementation of Google Piper. It is a monorepo & monolithic codebase management system that supports Git. Mega is designed to manage large-scale codebases, streamline development, and foster collaboration.

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

Mega will support conventional commits, which are a set of rules for creating clear and concise commit messages.  More information on the [Conventional Commits](https://www.conventionalcommits.org/).

### Code Owners

Mega will support code owners, which are a set of rules for defining who owns a particular piece of code. More information on the [Code Owners](https://help.github.com/en/github/creating-cloning-and-archiving-repositories/about-code-owners).

### Decentralized Open Source Collaboration

For now, the entire open source community base on Git and GitHub. It's centralized model, and it's not suitable for growing speed of open source world. Mega is working on build a decentralized open source collaboration model with [ZTM](https://github.com/flomesh-io/ztm)(Zero Trust Model) and decentralized social network like [Nostr](https://nostr.com), [Matrix](https://matrix.org) and [Mastodon](https://joinmastodon.org).

## Quick Try Monorepo Engine with Docker

For now, the monorepo engine could be deployed on your host machine or insulated into containers. For deploying through docker, follow the steps below:

1. Clone the project and build the docker images
```bash
git clone https://github.com/web3infra-foundation/mega.git
cd mega
docker buildx build -t mono-pg:0.1-pre-release -f ./docker/mono-pg-dockerfile .
docker buildx build -t mono-engine:0.1-pre-release -f ./docker/mono-engine-dockerfile .
docker buildx build -t mono-ui:0.1-pre-release -f ./docker/mono-ui-dockerfile .
```

2. Expand abbreviations, use initialize for mono-engine and PostgreSQL
```bash
# Linux or MacOS
./docker/init-volume.sh /mnt/data ./docker/config.toml
```

3. Run the mono-engine and PostgreSQL with docker, and open the mono-ui in your browser with `http://localhost:3000`.
```bash
# create network
docker network create mono-network

# run postgres
docker run --rm -it -d --name mono-pg --network mono-network -v /tmp/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mono-pg:0.1-pre-release
docker run --rm -it -d --name mono-engine --network mono-network -v /tmp/data/mono/mono-data:/opt/mega -p 8000:8000 mono-engine:0.1-pre-release
docker run --rm -it -d --name mono-ui --network mono-network -e MEGA_INTERNAL_HOST=http://mono-engine:8000 -e MEGA_HOST=http://localhost:8000 -e MOON_HOST=http://localhost:3000 -p 3000:3000 mono-ui:0.1-pre-release
```

## Quick Review of Architecture

![Mega Architect](docs/images/Mega_Bin_and_Modules.png)

1. **mono** - Mega's api endpoint, responsible for handling `git`, `git-lfs` and web UI requests through the HTTP and SSH protocol.
2. **mega** - The main entry of Mega, parse cli command with clap.
3. **gateway** - The Gateway extend **mono** and contains ztm agent, which is responsible for ztm network capabilities. More information on the [Gateway](gateway/README.md).
4. **libra** - The Libra is a `git` program that rewrite in Rust. More information on the [Libra](libra/README.md).
5. **gemini** - The Gemini is a decentralized module of Mega. More information on the [Gemini](gemini/README.md).
6. **scorpio** - The Scorpio is a FUSE filesystem that allow you to mount a Mega repository as a local filesystem. More information on the [Scorpio](scorpio/README.md).
7. **mercury** - The Mercury module is the core module of Mega, which rewrites Git internal object like Blob, Tree, etc. More information on the [Mercury Module](mercury/README.md).
8.  **ceres** - The Ceres is Mega's Monorepo Engine and implement the git transport protocol. More information on the [Ceres](ceres/README.md).
9. **jupiter** - The Jupiter is storage engine for Mega. More information on the [Jupiter](jupiter/README.md).
10. **saturn** - The Saturn module implement user permissions with Cedar policy [Saturn](saturn/README.md).
11. **vault** -  The Vault module is used for key and certificate management, implemented with RustyVault
12. **lunar** - The Lunar is a tauri app with Mega. More information on the [Lunar](lunar/README.md).
13. **aries** - The Mega's ztm Relay Server. More information on the [Aries](aries/README.md).
14. **altas** - AI API Module. More information on the [Altas](altas/README.md).
15. **moon** - The Moon is a web UI for Mono Engine. More information on the [Moon](moon/README.md).
16. **mars** - The Mars is website for Mega. More information on the [Mars](mars/README.md).



## Contributing

The mega project relies on community contributions and aims to simplify getting started. To develop Mega, clone the repository, then install all dependencies and initialize the database schema, run the test suite and try it out locally. Pick an issue, make changes, and submit a pull request for community review.

More information on contributing to Mega is available in the [Contributing Guide](docs/contributing.md).

## Talk and Share

If you interested in Mega, you can make an appointment with us on [Google Calendar](https://calendar.app.google/QuBf2sdmf68wVYWL7) to discuss your ideas, questions or problems, and we will share our vision and roadmap with you.

## License

Mega is licensed under this Licensed:

- MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
