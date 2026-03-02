# Orion Client Deployment Guide

## Overview

Orion Client is the worker node for the Mega build system. It fetches build tasks from the Orion Server and executes them. It integrates with the FUSE filesystem via the `scorpiofs` library to mount remote repositories locally.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Deployment Architecture                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────────┐            ┌──────────────────┐                      │
│   │   deployment     │            │      mega        │                      │
│   │   (Repo)         │            │     (Repo)       │                      │
│   └────────┬─────────┘            └────────┬─────────┘                      │
│            │                               │                                │
│            │ Terraform apply               │ git push (Triggers CI)         │
│            │                               │                                │
│            ▼                               ▼                                │
│   ┌──────────────────┐            ┌──────────────────┐                      │
│   │  Infrastructure  │            │  App Deployment  │                      │
│   │  - Provision VM  │            │  - Build rust bin│                      │
│   │  - Install deps  │            │  - Package config│                      │
│   │  - Setup systemd │            │  - SCP transfer  │                      │
│   │  - Create dirs   │            │  - Restart svc   │                      │
│   └────────┬─────────┘            └────────┬─────────┘                      │
│            │                               │                                │
│            └───────────────┬───────────────┘                                │
│                            │                                                │
│                            ▼                                                │
│                   ┌──────────────────┐                                      │
│                   │      GCP VM      │                                      │
│                   │   orion-client   │                                      │
│                   │                  │                                      │
│                   │     systemd:     │                                      │
│                   │   orion-runner   │                                      │
│                   └──────────────────┘                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Directory Structure

### VM Runtime Environment
All files are owned by the `orion` user.

```
/home/orion/orion-runner/       # Application root
├── orion                       # Main executable
├── .env                        # Environment variables
├── scorpio.toml                # Scorpio config
├── run.sh                      # Startup script
└── cleanup.sh                  # Pre-start cleanup script

/data/scorpio/                  # Scorpio data directory
├── store/                      # Dicfuse data store
├── tmp_build/                  # Temporary Buck2 build dir
└── antares/                    # Antares overlay config

/workspace/mount/               # FUSE main mount point
```

## VM Initialization (One-time Setup)

Before the first deployment via CI, the target VM must be initialized with the following steps. (This is generally handled by the Terraform startup script in the `deployment` repository).

### 1. Create User & Directories

```bash
sudo useradd -m -s /bin/bash orion

sudo mkdir -p /data/scorpio/{store,antares/{upper,cl,mnt}}
sudo mkdir -p /workspace/mount
sudo mkdir -p /home/orion/orion-runner
sudo chown -R orion:orion /data/scorpio /workspace/mount /home/orion/orion-runner
```

### 2. Configure FUSE

```bash
# Add orion to fuse group
sudo usermod -aG fuse orion

# Allow non-root users to mount with allow_other
echo "user_allow_other" | sudo tee -a /etc/fuse.conf
```

### 3. Setup Passwordless Sudo

The deployment CI requires passwordless execution of specific commands.

```bash
cat <<'EOF' | sudo tee /etc/sudoers.d/orion-runner
orion ALL=(ALL) NOPASSWD: /usr/bin/systemctl start orion-runner.service
orion ALL=(ALL) NOPASSWD: /usr/bin/systemctl stop orion-runner.service
orion ALL=(ALL) NOPASSWD: /usr/bin/systemctl restart orion-runner.service
orion ALL=(ALL) NOPASSWD: /usr/bin/systemctl daemon-reload
orion ALL=(ALL) NOPASSWD: /usr/bin/pkill
orion ALL=(ALL) NOPASSWD: /usr/bin/mkdir
orion ALL=(ALL) NOPASSWD: /usr/bin/chown
orion ALL=(ALL) NOPASSWD: /bin/umount
orion ALL=(ALL) NOPASSWD: /usr/bin/cp /home/orion/orion-runner/orion-runner.service /etc/systemd/system/
EOF
sudo chmod 440 /etc/sudoers.d/orion-runner
```

### 4. Adjust SSH Limits

To prevent CI from timing out due to multiple rapid SSH connections:

```bash
sudo sed -i 's/^#\?MaxStartups.*/MaxStartups 10:30:60/' /etc/ssh/sshd_config
sudo systemctl restart sshd
```

## CI/CD Pipeline

Deployment is automated via GitHub Actions (`.github/workflows/orion-client-deploy.yml`).

### Trigger
Automatically triggered by pushes to the `main` branch when files in `orion/**` or the workflow file itself change.

### Deployment Targets

| VM | User | Target Path |
|----|------|-------------|
| Legacy VM | root (legacy compatibility) | `/home/orion/orion-runner/` |
| GCP VM | orion (best practice) | `/home/orion/orion-runner/` |

**Deployment Secrets required in GitHub:**
- `ORION_DEPLOY_HOST` / `ORION_DEPLOY_SSH_KEY` (Legacy VM)
- `ORION_GCP_VM_HOST` / `ORION_GCP_VM_SSH_KEY` (GCP VM)

### Workflow Steps
1. Compiles the `orion` binary (`cargo build --release -p orion`).
2. Packages configuration files from `orion/runner-config/`.
3. Connects to VMs and stops the existing `orion-runner.service`.
4. Uploads files using `scp` to the `/home/orion/orion-runner/` directory.
5. Updates the systemd service file if necessary.
6. Restarts the service asynchronously using `nohup systemctl start`.

## Troubleshooting

### Useful Commands

```bash
# Check service status
sudo systemctl status orion-runner

# Follow service logs
sudo journalctl -u orion-runner -f

# Force unmount if FUSE is stuck
sudo umount -lf /workspace/mount
```

### Common Issues

1. **`status=217/USER`**: The `orion` user was not created on the target machine. Run the VM Initialization steps.
2. **`unexplained error (code 255) at io.c`**: SSH connection rejected by the server. Ensure `MaxStartups 10:30:60` is set in `/etc/ssh/sshd_config` and `sshd` is restarted.
3. **FUSE Mount Fails**: Ensure `user_allow_other` is enabled in `/etc/fuse.conf`.
