# Orion Client 部署文档

## 概述

Orion Client 是 Mega 构建系统的 Worker 节点，负责从 Orion Server 领取构建任务并执行。它通过 scorpiofs 库集成 FUSE 文件系统，实现远程仓库的本地挂载。

## 架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              部署架构                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────────┐            ┌──────────────────┐                      │
│   │   deployment     │            │      mega        │                      │
│   │   (仓库)         │            │     (仓库)       │                      │
│   └────────┬─────────┘            └────────┬─────────┘                      │
│            │                               │                                │
│            │ Terraform apply               │ git push (CI 触发)             │
│            │                               │                                │
│            ▼                               ▼                                │
│   ┌──────────────────┐            ┌──────────────────┐                      │
│   │  基础设施准备     │            │   应用部署        │                      │
│   │  - VM 创建       │            │  - 编译 orion     │                      │
│   │  - 依赖安装      │            │  - 打包配置       │                      │
│   │  - systemd       │            │  - rsync 部署     │                      │
│   │  - 目录创建      │            │  - 重启服务       │                      │
│   └────────┬─────────┘            └────────┬─────────┘                      │
│            │                               │                                │
│            └───────────────┬───────────────┘                                │
│                            │                                                │
│                            ▼                                                │
│                   ┌──────────────────┐                                      │
│                   │    GCP VM        │                                      │
│                   │  orion-client    │                                      │
│                   │                  │                                      │
│                   │  systemd:        │                                      │
│                   │  orion-runner    │                                      │
│                   └──────────────────┘                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 仓库职责

### 1. deployment 仓库（基础设施）

**路径**: `envs/gcp/prod/`

负责 GCP 基础设施的管理：

| 资源 | 说明 |
|------|------|
| `google_compute_instance.orion_client_vm` | Worker VM 实例 |
| `startup-orion-client.sh` | VM 启动脚本 |
| 网络/防火墙 | VPC 和安全规则 |
| IAM | 服务账号权限 |

**Terraform 执行内容**：
1. 创建 VM 实例（Debian 13）
2. 执行 startup-orion-client.sh：
   - 安装系统依赖（fuse3, git, rust, buck2 等）
   - 创建运行时目录
   - 创建 systemd 服务单元
   - 启用并尝试启动服务

### 2. mega 仓库（应用代码）

**相关路径**：
- `orion/` - 应用代码
- `orion/runner-config/` - 生产运行时配置
- `.github/workflows/orion-client-deploy.yml` - CI 工作流

**CI 执行内容**：
1. 编译 orion 二进制（`cargo build --release -p orion`）
2. 打包配置文件
3. rsync 部署到 VM
4. 重启 systemd 服务

## 目录结构

### VM 运行时目录

```
/home/orion/orion-runner/       # 应用根目录（orion 用户）
├── orion                       # 主程序
├── .env                        # 环境变量
├── scorpio.toml                # Scorpio 配置
├── run.sh                      # 启动脚本
└── log/
    └── orion.log               # 应用日志

/data/scorpio/                  # Scorpio 数据目录
├── store/                      # Dicfuse 数据存储
├── tmp_build/                  # Buck2 临时构建目录
└── antares/                    # Antares overlay 配置
    ├── upper/                  # Overlay upper 层
    ├── cl/                     # CL 数据
    ├── mnt/                    # Overlay 挂载点
    └── state.toml              # 状态文件

/workspace/mount/               # FUSE 主挂载点（Scorpio daemon 挂载）
```

### 源码配置文件

```
mega/orion/runner-config/       # 生产配置（版本控制）
├── .env.prod                   # 生产环境变量 → 部署时重命名为 .env
├── scorpio.toml                # Scorpio 配置
├── run.sh                      # 启动脚本
└── README.md                   # 说明文档
```

## 配置文件

### scorpio.toml

Scorpio/Dicfuse FUSE 文件系统配置：

```toml
# Mega 服务地址
base_url = "https://git.buck2hub.com"
lfs_url = "https://git.buck2hub.com"

# 数据存储
store_path = "/data/scorpio/store"
workspace = "/workspace/mount"

# Antares overlay 配置
antares_upper_root = "/data/scorpio/antares/upper"
antares_cl_root = "/data/scorpio/antares/cl"
antares_mount_root = "/data/scorpio/antares/mnt"
antares_state_file = "/data/scorpio/antares/state.toml"
```

### .env

环境变量配置：

```bash
# Buck2 项目根目录
BUCK_PROJECT_ROOT="/workspace/mount"

# Orion Server WebSocket
SERVER_WS="wss://orion.buck2hub.com/ws"

# 任务轮询配置
SELECT_TASK_COUNT="30"
INITIAL_POLL_INTERVAL_SECS="2"

# 临时构建目录
TMP_BUCKOUT_DIR="/data/scorpio/tmp_build"
```

## 部署流程

### 首次部署

```
1. Terraform apply (deployment 仓库)
   ├── 创建 VM
   ├── 执行 startup-orion-client.sh
   │   ├── 安装依赖
   │   ├── 创建目录
   │   ├── 创建 systemd 服务
   │   └── 尝试启动（会失败，配置文件尚未部署）
   │
2. CI 首次触发 (mega 仓库)
   ├── 编译 orion
   ├── rsync 部署配置和程序
   └── 重启服务 ✓
```

### 后续更新

```
git push to main (orion/** 或 workflow 改动)
   │
   ├── CI 编译
   ├── CI rsync 部署
   └── CI 重启服务
```

## systemd 服务

### 服务单元 (orion-runner.service)

```ini
[Unit]
Description=Orion Runner and Scorpio Service
After=network.target

[Service]
User=orion
Group=orion
WorkingDirectory=/home/orion/orion-runner
ExecStart=/bin/bash run.sh

Restart=on-failure
RestartSec=5
LimitNOFILE=10485760
LimitNPROC=1048576

StandardOutput=append:/var/log/orion-runner.log
StandardError=append:/var/log/orion-runner.log

[Install]
WantedBy=multi-user.target
```

### 常用命令

```bash
# 查看状态
sudo systemctl status orion-runner

# 查看日志
sudo journalctl -u orion-runner -f
tail -f /var/log/orion-runner.log
tail -f /home/orion/orion-runner/log/orion.log

# 重启服务
sudo systemctl restart orion-runner

# 停止服务
sudo systemctl stop orion-runner
```

## 故障排查

### 常见问题

1. **FUSE 挂载失败**
   ```bash
   # 检查 fuse 配置
   grep user_allow_other /etc/fuse.conf
   
   # 手动卸载
   fusermount -u /workspace/mount
   ```

2. **服务启动失败**
   ```bash
   # 检查配置文件是否存在
   ls -la /home/orion/orion-runner/
   
   # 检查权限
   ls -la /data/scorpio/
   ```

3. **网络连接问题**
   ```bash
   # 测试 Orion Server 连接
   curl -I https://orion.buck2hub.com
   
   # 测试 Mega 仓库连接
   curl -I https://git.buck2hub.com
   ```

### 日志位置

| 日志 | 路径 |
|------|------|
| VM 启动日志 | `/var/log/orion-client-startup.log` |
| systemd 服务日志 | `/var/log/orion-runner.log` |
| Orion 应用日志 | `/home/orion/orion-runner/log/orion.log` |

## 本地开发

参考 [orion/README.md](../README.md) 中的本地开发说明。

本地开发使用独立的配置文件：
- `orion/.env` - 本地环境变量
- `orion/scorpio.toml` - 本地 Scorpio 配置
- `orion/run-dev.sh` - 本地启动脚本

## CI/CD 配置

### 触发条件

```yaml
on:
  push:
    branches: [main]
    paths:
      - ".github/workflows/orion-client-deploy.yml"
      - "orion/**"
  workflow_dispatch:  # 手动触发
```

### Secrets 配置

| Secret | 说明 |
|--------|------|
| `ORION_DEPLOY_HOST` | 旧 VM IP 地址 |
| `ORION_DEPLOY_SSH_KEY` | 旧 VM SSH 私钥 |
| `ORION_GCP_VM_HOST` | GCP VM IP 地址 |
| `ORION_GCP_VM_SSH_KEY` | GCP VM SSH 私钥 |

### 部署目标

| VM | 用户 | 路径 |
|----|------|------|
| orion_vm (旧) | root | `/root/orion-runner/` |
| gcp_vm (新) | orion | `/home/orion/orion-runner/` |

## 配置变更流程

1. 修改 `orion/runner-config/` 下的配置文件
2. 提交并推送到 main 分支
3. CI 自动部署到所有 VM
4. 服务自动重启

## 注意事项

1. **配置单一来源**: 所有生产配置维护在 `mega/orion/runner-config/`
2. **无需手动配置**: Terraform startup script 不再生成配置文件
3. **首次部署**: 需要 Terraform + CI 两步完成
4. **更新部署**: 只需 CI 即可完成
