# Mono & Mega - is an monorepo & monolithic codebase management system and decentralized open source collaboration network application

Mono is a monorepo & monolithic codebase management system that supports Git, it is designed to manage large-scale codebases, streamline development, and foster collaboration. Mono is an unofficial open source implementation of Google Piper. 

Mega is a standalone version of Mono for individual developers on their local machines, enabling them to connect with each other to build a decentralized open source collaboration network. 

## What's the Piper?

Google Piper is a massive, centralized version control system that Google uses internally to manage their vast codebase. It is a monorepo, and a monolithic which mean is a single repository that contains all the source code for Google's software. It is designed to manage large-scale codebases, streamline development, and foster collaboration. It is built on top of Google's internal infrastructure and is designed to be highly scalable and efficient. More information on the [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository/fulltext).

**Google Piper is not open source**

## Mono features

Mono is an unofficial open source implementation of Google Piper. And it has the following features:

### Git compatible

Mono offers the ability to utilize Git with a monorepo. This allows for easy cloning or pulling of any monorepo folder into local filesystem as a Git repository, and seamless pushing of changes back.

### Trunk-based Development

When it comes to managing large codebases in a centralized manner, trunk-based development is the way to go. More trunk-based Development information on the [Trunk-Based Development](https://trunkbaseddevelopment.com/).

### Conventional Commits

Mono will support conventional commits, which are a set of rules for creating clear and concise commit messages.  More information on the [Conventional Commits](https://www.conventionalcommits.org/).

### Code Owners

Mono will support code owners, which are a set of rules for defining who owns a particular piece of code. More information on the [Code Owners](https://help.github.com/en/github/creating-cloning-and-archiving-repositories/about-code-owners).

## Mega features

### Decentralized Open Source Collaboration

For now, the entire open source community base on Git and GitHub. It's centralized model, and it's not suitable for growing speed of open source world. Mega is working on build a decentralized open source collaboration model with [ZTM](https://github.com/flomesh-io/ztm)(Zero Trust Model) and decentralized social network like [Nostr](https://nostr.com), [Matrix](https://matrix.org) and [Mastodon](https://joinmastodon.org).

## Quick Try Monorepo Engine with Docker

For now, the monorepo engine could be deployed on your host machine or insulated into containers. For deploying through docker, follow the steps below:

1. Pull docker images from Docker Hub

```bash
$ docker pull genedna/mega:mono-pg-latest
$ docker pull genedna/mega:mono-engine-latest
$ docker pull genedna/mega:mono-ui-latest
```

2. Initialize for mono-engine and PostgreSQL

```bash
$ git clone https://github.com/web3infra-foundation/mega.git
$ cd mega
# Linux or MacOS
$ ./docker/init-volume.sh /tmp/data ./config/config.toml
```

3. Run the mono-engine and PostgreSQL with docker, and open the mono-ui in your browser with `http://localhost:3000`.

```bash
# create network
$ docker network create mono-network

# run postgres
$ docker run --rm -it -d --name mono-pg --network mono-network -v /tmp/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 genedna/mega:mono-pg-latest
$ docker run --rm -it -d --name mono-engine --network mono-network -v /tmp/data/mono/mono-data:/opt/mega -p 8000:8000 genedna/mega:mono-engine-latest
$ docker run --rm -it -d --name mono-ui --network mono-network -e MEGA_INTERNAL_HOST=http://mono-engine:8000 -e MEGA_HOST=http://localhost:8000 -p 3000:3000 genedna/mega:mono-ui-latest
```

4. Try to upload a repository to mono-engine

```bash
$ git clone http://localhost:8000/project.git
$ cd project
$ git clone https://github.com/dagrs-dev/dagrs.git
$ sudo rm -r dagrs/.git
$ git add .
$ git commit -a -m"Initial the dagrs project"
$ git push
```

5. Check the repository in UI
Open the mono-ui in your browser with `http://localhost:3000`, and you will see the `project` folder.

## Contributing

The mega project relies on community contributions and aims to simplify getting started. To develop Mega, clone the repository, then install all dependencies and initialize the database schema, run the test suite and try it out locally. Pick an issue, make changes, and submit a pull request for community review.

More information on contributing to Mega is available in the [Contributing Guide](docs/contributing.md).

## Chat with us

If you interested in Mega, you can make an appointment with us on [Google Calendar](https://calendar.app.google/QuBf2sdmf68wVYWL7) to discuss your ideas, questions or problems, and we will share our vision and roadmap with you.

## License

Mega is licensed under this Licensed:

- MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
