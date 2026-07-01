# Orion

Orion is a Rust-based Buck build task WebSocket client. It communicates with a server via WebSocket, receives build tasks, and streams build output in real time, making it suitable for distributed or remote build scenarios.

## Features

- Communicates with the server via WebSocket to receive build tasks (repo/target/args).
- Invokes the local `buck2 build` command, collects stdout/stderr in real time, and streams output back via WebSocket.
- Supports task status feedback (started, output, completed).
- Automatically creates required files and directories.

## systemd (GCP VM)

The repository includes a ready-to-install unit file at `orion/systemd/orion-runner.service`.
Create `/etc/orion/orion-runner.env` based on `orion/systemd/orion-runner.env.example` and make
sure at least `SERVER_WS` and `BUCK_PROJECT_ROOT` are set.

## GitHub Release

Orion is released as a native Linux amd64 runner bundle, not as a Docker runtime image. Push an
`orion-vX.Y.Z` tag to create a GitHub Release:

```bash
git tag -a orion-v0.1.1 -m "Release orion v0.1.1"
git push origin orion-v0.1.1
```

The `.github/workflows/orion-release.yml` workflow builds:

```bash
cargo build --release -p orion --bin orion --target x86_64-unknown-linux-gnu
```

and uploads these GitHub Release assets:

```text
orion-vX.Y.Z-linux-amd64.tar.gz
orion-vX.Y.Z-linux-amd64.tar.gz.sha256
```

The tarball layout is:

```text
orion-vX.Y.Z-linux-amd64/
├── orion
├── runner-config/
│   ├── .env.prod
│   ├── cleanup.sh
│   ├── preflight.sh
│   ├── run.sh
│   └── scorpio.toml
├── systemd/
│   ├── orion-runner.env.example
│   └── orion-runner.service
└── VERSION
```

Download and verify:

```bash
VERSION=v0.1.1
BUNDLE=orion-${VERSION}-linux-amd64
REPO=gitmono-dev/mega

curl -LO https://github.com/${REPO}/releases/download/orion-${VERSION}/${BUNDLE}.tar.gz
curl -LO https://github.com/${REPO}/releases/download/orion-${VERSION}/${BUNDLE}.tar.gz.sha256
sha256sum -c ${BUNDLE}.tar.gz.sha256
tar -xzf ${BUNDLE}.tar.gz
```

The bundle shape intentionally matches the path layout expected by `orion-scheduler` artifact
distribution: binary at bundle root, runtime scripts under `runner-config/`, and systemd files under
`systemd/`.
