# 测试方法

本地调试、API 测试、服务管理和常见问题排查。

## 前提条件

```bash
# SSH 密钥（路径写入 target_config.json 的 ssh_public_key_path）
ssh-keygen -t ed25519 -f ~/.ssh/orion_vm_access -N "" -C "orion-scheduler"

# 配置文件
cp orion-scheduler/target_config.json.template orion-scheduler/target_config.json
# 编辑 target_config.json，填入本机路径
```

自定义 VM 镜像的构建与上传见 [§4 构建镜像并上传到 S3](#4-构建镜像并上传到-s3)。

---

## 1. 快速开始

```bash
# 构建并启动 scheduler（需 root/KVM）
cargo build -p orion-scheduler
sudo env "PATH=$PATH" "RUSTUP_HOME=$RUSTUP_HOME" "CARGO_HOME=$CARGO_HOME" "HOME=$HOME" \
  cargo run -p orion-scheduler

# 触发 VM + Orion worker（推荐：本地 debian-13-buck2 镜像）
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{
    "target": "k3s-buck2hub",
    "image_path": "~/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2",
    "image_digest": "sha256:753c28888c9d30fe4baef55c1d1dfa9a39431595eca940b7ad85d78d84f3d7a5",
    "image_disk_gb": 30,
    "image_cpus": 8,
    "image_memory_mb": 16000
  }'

# VM 与日志
curl http://localhost:8080/status
curl -N http://localhost:8080/logs/orion/stream
```

`image_path` 支持 `~/...` 或绝对路径。其他 webhook 变体（默认镜像、远程 `image_url`）见 [§2 Webhook](#webhook)。

---

## 2. API 参考

### 健康检查

```bash
curl http://localhost:8080/health
# {"status": "healthy", "service": "orion-scheduler"}
```

### Webhook

```bash
# GET
curl http://localhost:8080/webhook

# POST — 默认 Debian 镜像
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{"target": "aws-gitmega"}'

# POST — 本地自定义镜像（字段同 §1 快速开始）
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{"target": "gcp-buck2hub", "image_path": "~/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2", "image_digest": "sha256:...", "image_disk_gb": 20, "image_cpus": 4, "image_memory_mb": 8192}'

# POST — 远程镜像（image_url + image_digest + 可选 disk/cpus/memory）
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{"target": "aws-gitmega", "image_url": "https://...", "image_digest": "sha256:...", "image_disk_gb": 20, "image_cpus": 4, "image_memory_mb": 8192}'
```

### VM 状态

```bash
curl http://localhost:8080/status
# {"status": "running", "vm_id": "orion-vm-xxx", "vm_ip": "192.168.221.x", "uptime_secs": 60, ...}
```

### SSH 进入 VM

```bash
VM_IP=$(curl -s http://localhost:8080/status | jq -r .vm_ip)
ssh -i ~/.ssh/orion_vm_access root@$VM_IP
```

### 日志

| 端点 | 格式 | 说明 |
|------|------|------|
| `GET /logs/orion/stream` | SSE | 每 2 秒推送；`curl -N` 持续监控 |

```bash
curl -N http://localhost:8080/logs/orion/stream
```

服务端调试日志：`RUST_LOG=debug cargo run -p orion-scheduler`；systemd 部署用 `journalctl -u orion-scheduler -f`。

### Scorpio 状态

```bash
curl http://localhost:8080/scorpio/status
```

### 关闭

```bash
curl -X POST http://localhost:8080/shutdown
# 仅停 VM，scheduler 继续运行
```

---

## 3. 服务管理

### 停止与检查

```bash
# 优雅：先关 VM（见上），再停 scheduler
kill -TERM <orion-scheduler-pid>

# 强制（不关闭 VM）
pkill -9 -f orion-scheduler
sudo pkill -9 -f qemu-system-x86   # 清理残留 QEMU

ps aux | grep -E "orion-scheduler|qemu-system" | grep -v grep
fuser 8080/tcp 2>/dev/null || echo "Port 8080 is free"
```

### 信号与关闭方式

| 操作 | VM | scheduler | 说明 |
|------|-----|-----------|------|
| `Ctrl+C` / SIGTERM / SIGQUIT | 停止 | 停止 | 先关 VM 再退出 |
| `POST /shutdown` | 停止 | **继续** | 仅关 VM，适合换 CL 重跑 |
| `pkill -9 -f orion-scheduler` | 可能残留 | 停止 | 不优雅 |

---

## 4. 构建镜像并上传到 S3

```bash
sudo modprobe nbd max_part=8
sudo bash ~/mega/orion-scheduler/scripts/build-custom-image.sh
# 输出 sha256:<hex>，用作 webhook 的 image_digest
```

```bash
aws s3 cp ~/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2 \
  s3://gitmega/images/debian-13-buck2.qcow2 --progress
```

`image_digest` 使用构建脚本输出的本地文件 hash；上传前后内容不变则 hash 一致。

---

## 5. 常见问题排查

| 问题 | 排查 |
|------|------|
| KVM 权限错误 | `/dev/kvm` 权限；用户是否在 `kvm` 组 |
| QEMU 桥接失败 | `/etc/qemu/bridge.conf` 是否 `allow qlbr0` |
| VM 启动超时 | cloud-init、SSH 是否可达 |
| Orion 启动失败 | `curl -N http://localhost:8080/logs/orion/stream` |
| Scorpio 挂载问题 | `curl http://localhost:8080/scorpio/status` |
| 状态仍 running 但 VM 已死 | 重启 scheduler 或查 QEMU 进程 |
| 进 VM 调试 | [SSH 进入 VM](#ssh-进入-vm) |
