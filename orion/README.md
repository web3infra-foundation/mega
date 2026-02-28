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
