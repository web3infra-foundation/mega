# Update Module

This module syncs crates from crates.io, pushes them to a Git server, and emits events to Kafka while recording results in Postgres.

## Features
- Clones/updates `crates.io-index`
- Downloads new crate versions and verifies checksums
- Unpacks crates and pushes them to a remote Git server
- Sends Kafka messages for downstream processing
- Persists sync status to Postgres via SQLAlchemy

## Requirements
- Network access to:
  - GitHub (crates.io index)
  - Your Git server (MEGA)
  - Kafka broker
  - Postgres database
- Persistent volume for `/opt/data` if you want data to survive container restarts

## Build the Docker Image
```bash
docker build -f extensions/update/Dockerfile -t mega-update:latest .
```

## Run (one-shot)
One execution of a full sync:

```bash
docker run --rm -d \
  --name mega-update \
  --add-host=git.gitmega.nju:172.17.0.1 \
  --env-file ./extensions/update/.env \
  -v /mnt/data:/opt/data \
  mega-update:latest
```

- Mount `/opt/data` to persist data and avoid re-downloading.


## Local Testing
You can run the module locally as well (Python 3.11+):

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r extensions/update/requirements.txt
PYTHONPATH=extensions python -m update.sync
```

Ensure your local environment can reach GitHub, MEGA, Kafka, and Postgres.