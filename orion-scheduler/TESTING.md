# 测试方法

本文档包含本地调试、API 测试、服务管理和常见问题排查的方法。

## 前提条件

首次使用前需要先生成 SSH 密钥并构建本地镜像：

```bash
# 1. 生成 SSH 密钥对（用于 VM 访问，路径配置在 target_config.json 的 ssh_public_key_path）
ssh-keygen -t ed25519 -f ~/.ssh/orion_vm_access -N "" -C "orion-scheduler"

# 2. 构建自定义镜像（包含 Rust 工具链和 Buck2）
sudo bash ~/mega/orion-scheduler/scripts/build-custom-image.sh

# 3. 复制并编辑配置文件
cp orion-scheduler/target_config.json.template orion-scheduler/target_config.json
# 编辑 target_config.json，填入本机实际路径
```

## 1. 本地调试

### 构建与运行

```bash
# 构建调试版本
cargo build

# 运行服务（调试模式）
sudo env "PATH=$PATH" "RUSTUP_HOME=$RUSTUP_HOME" "CARGO_HOME=$CARGO_HOME" "HOME=$HOME" cargo run
```

### 调试流程

```bash
# 1. 发送 webhook 请求（使用默认 Debian 镜像）
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{"target": "aws-gitmega"}'

# 1b. 发送 webhook 请求（指定本地自定义镜像）
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{
    "target": "aws-gitmega",
    "image_path": "~/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2",
    "image_digest": "sha256:e3219324738ef9492042f021fa13b44dc668507f2bd254b5c3470f1d7cdfcce4",
    "image_disk_gb": 20,
    "image_cpus": 2,
    "image_memory_mb": 4096
  }'

# 1c. 发送 webhook 请求（指定远程镜像）
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{
    "target": "aws-gitmega",
    "image_url": "https://gitmega.s3.ap-southeast-2.amazonaws.com/images/debian-13-buck2.qcow2?response-content-disposition=inline&X-Amz-Content-Sha256=UNSIGNED-PAYLOAD&X-Amz-Security-Token=IQoJb3JpZ2luX2VjEDgaDmFwLXNvdXRoZWFzdC0yIkYwRAIgQmiSRJW4dcJqZ1YlbTo64NAZipaYlDxezUtQoVpn2R4CICJxDmSVTUWvGXxGJxSVCm59TFian4l%2B95P4lRM9X5VLKtYDCAEQABoMNTM1MDAyODcyMDczIgytK%2FEFzG5xpCYPutAqswM2%2FRYbsun0%2FTUNc44myCbtG8Zl9vGxs0zoHA8PUK5yxWVugKy7wE8maQyBRsnRxj97YvDd64HDWJgy%2F6ZJRzRIkn5O4gOjOfACr2RibrAF951%2FnIz8gyiESic8DUVBV8K0xLT%2FOXOIvY9DdhwNXP5O1CG63IRE%2FEoIAEwDrJl4Fr3tW868bdzRUEiYwwclvWQ17i8Gw2xnbJ%2FLUTnuWOcoI3tECZam2VHs1Fi00YyIhTnZRmKcqirxIar8%2BGv7JrXrMd0Nup8s12zGjsZWJ%2FeNxVWzmh4A4K43enJT%2BAwHhQ%2FEfTVZh%2F4CxYXbOHTiSVVjvtpJ4QgiiQR6VyiY5Wp2UkEHguiQC8MIOemYIuZFdSoDWvs1HjofbJ13%2FdySYg1fRvlnmteJyE%2F4J6vJ83Rt3W4GeqTntKFIqC6xlhKbYx0Wektf1p%2F1qMXKFmNI%2BvIDVU5xS9Prm6NKWkUeKDBC6t%2FdxDXjzyRJNVenmGPg%2BHufyUEb2gyKDdgXxxg%2BBYK%2Bwt97tvqRh8s58khV36v8Nt55NlrWMIAi84q8AJFKtL227wDmRXl%2FZGsXBYMR776qrt8wlte50AY63wKNrXwyrKpRVq0LjQ5Rd0nklMtriQ9deJo1NNT6CR8ZJr2x17Cf3JSM1EbQwCRdHpJavob7bVJyrIhVY%2B8zIFp70XsZZWhcnd6P32ymnttMAvCQUZB%2FxcHN%2BDlzB1AtATRWtXihVq0ExD2%2BLbXCx8H0Lkdxpd87EOZ7d095DK9zFXz7FAlXlPiGGVSyQWB%2FUfQzk%2BDv1%2FInTIjjSnzYQD5dmrPwXVtbEBx6zolpTCYGyVLpnasgsFTv1nUR9c7e7POCxVjEy8WO%2FhAZwA%2FP8FDxnmlxUatyD7zTPohPWJVpymprxABhaZrUQLTL9SBuU462Wfc6sv6Mmm7RVKkIqGrDzW3OdnJtVCeYUdEhXUHlis0cbQsFzce9bM0LnynlBd67mqOyfpZdeJ531NPvgdukOzfCE2BITZ%2BjXWMDN%2F3Ndo%2FaSoC%2B9lT%2FwYnhxkMQh4cSKog3pPsvllJHNG4RILs%3D&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=ASIAXZEFH7EE6E2HVVIM%2F20260521%2Fap-southeast-2%2Fs3%2Faws4_request&X-Amz-Date=20260521T082130Z&X-Amz-Expires=14400&X-Amz-SignedHeaders=host&X-Amz-Signature=99471e1c7bab465f0242fc9bb86aabfbdbf271d079ca0db780b9ba29503e3d86",
    "image_digest": "sha256:677ef198bb2a8a30bb3a593b1b70efb9a14f6e06a1193df47d4e028bce0445d6",
    "image_disk_gb": 20,
    "image_cpus": 4,
    "image_memory_mb": 8192
  }'

# 2. 检查服务状态（VM 应保持运行状态）
curl http://localhost:8080/status

# 3. 持续监控日志（SSE 流，每 2 秒刷新，适合终端监控）
curl -N http://localhost:8080/logs/orion/stream
```

## 2. API 测试

### 健康检查

```bash
curl http://localhost:8080/health
# 响应: {"status": "healthy", "service": "orion-scheduler"}
```

### Webhook

```bash
# GET - 健康检查
curl http://localhost:8080/webhook
# 响应: {"status": "ok", "vm_id": null, "error": null, "orion_log_file": null}

# POST - 触发部署（keep-alive 模式，使用默认 Debian 镜像）
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{"target": "gcp-buck2hub"}'
# 响应: {"status": "ok", "vm_id": "orion-vm-xxx", "error": null, "orion_log_file": null}

# POST - 指定本地镜像
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{
    "target": "gcp-buck2hub",
    "image_path": "~/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2",
    "image_digest": "sha256:abcd1234...",
    "image_disk_gb": 20,
    "image_cpus": 4,
    "image_memory_mb": 8192
  }'
```

### VM 状态

```bash
# 获取 VM 状态（keep-alive 模式，VM 持续运行）
curl http://localhost:8080/status
# 响应: {"status": "running", "vm_id": "orion-vm-xxx", "vm_ip": "192.168.221.87", "uptime_secs": 60, "log_file": "/var/log/orion-scheduler/..."}
```

### 日志端点


| 端点                       | 响应格式 | 特点      | 使用场景           |
| ------------------------ | ---- | ------- | -------------- |
| `GET /logs/orion/stream` | SSE  | 每 2 秒推送 | `curl -N` 持续监控 |


```bash
curl -N http://localhost:8080/logs/orion/stream
```

### Scorpio 状态

```bash
curl http://localhost:8080/scorpio/status
# 响应: {"status": "ok", "directories": {...}, "mounts": "...", "orion_process": "...", "scorpio_process": "..."}
```

### 关闭

```bash
# 优雅关闭（停止 VM 并退出）
curl -X POST http://localhost:8080/shutdown
# 响应: {"status": "ok", "message": "Shutdown initiated, VM will be stopped"}
```

## 3. 服务管理

### 启动服务

```bash
cargo run -p orion-scheduler
```

### 停止服务

```bash
# 停止 orion-scheduler 服务进程
pkill -9 -f orion-scheduler

# 停止所有 QEMU 进程（如果有残留的 VM）
sudo pkill -9 -f qemu-system-x86

# 验证进程已停止
ps aux | grep -E "orion-scheduler|qemu-system" | grep -v grep

# 检查端口是否释放
fuser 8080/tcp 2>/dev/null || echo "Port 8080 is free"
```

### 检查服务状态

```bash
# 检查 HTTP API 状态
curl http://localhost:8080/status

# 检查 orion-scheduler 进程
ps aux | grep orion-scheduler | grep -v grep

# 检查 QEMU 进程
ps aux | grep qemu | grep -v grep
```

### 优雅关闭对比


| 操作                            | VM  | 服务器  | 说明                     |
| ----------------------------- | --- | ---- | ---------------------- |
| `Ctrl+C`                      | 停止  | 停止   | 关闭 VM 后退出服务            |
| SIGTERM                       | 停止  | 停止   | 关闭 VM 后退出服务            |
| SIGQUIT                       | 停止  | 停止   | 关闭 VM 后退出服务            |
| `POST /shutdown`              | 停止  | 继续运行 | 仅关闭 VM，服务保持运行          |
| `pkill -9 -f orion-scheduler` | -   | 停止   | **不优雅**：直接杀死进程，不会关闭 VM |


```bash
# 关闭 VM，服务继续运行（推荐）
curl -X POST http://localhost:8080/shutdown

# 发送 SIGTERM 信号（关闭 VM 并停止服务）
kill -TERM <pid>

# 强制杀死进程（不优雅）
pkill -9 -f orion-scheduler
```

## 4. 查看日志

```bash
# 服务端日志
RUST_LOG=debug cargo run 2>&1 | grep -E '\[orion|webhook|vm'

# Orion 实时 SSE 流（持续刷新，Ctrl+C 退出）
curl -N http://localhost:8080/logs/orion/stream

# systemd 日志（如服务以 systemd 运行）
journalctl -u orion-scheduler -f
```

## 5. 构建镜像并上传到 S3

### 5.1 构建本地镜像

```bash
sudo modprobe nbd max_part=8
sudo bash ~/mega/orion-scheduler/scripts/build-custom-image.sh
```

构建完成后会输出：

```
sha256:<镜像SHA256值>
```

### 5.2 上传到 S3

```bash
# 上传镜像到 S3
aws s3 cp ~/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2 \
  s3://gitmega/images/debian-13-buck2.qcow2

# 上传时显示进度
aws s3 cp ~/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2 \
  s3://gitmega/images/debian-13-buck2.qcow2 --progress

```

> **注意**：`image_digest` 使用构建脚本输出的 `sha256:<hex>` 值（上传前本地文件的 hash），上传后内容不变 hash 保持一致。

## 6. 常见问题排查


| 问题                  | 排查方法                                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------------------- |
| KVM 权限错误            | 检查 `/dev/kvm` 权限，确保用户在 `kvm` 组                                                                                |
| QEMU 网络桥接失败         | 检查 `/etc/qemu/bridge.conf` 是否配置 `allow qlbr0`                                                                 |
| VM 启动超时             | 检查 cloud-init 是否正常，SSH 是否可连接                                                                                  |
| Orion 启动失败          | `curl -N http://localhost:8080/logs/orion/stream` 实时查看日志                                                      |
| Scorpio 挂载问题        | `curl http://localhost:8080/scorpio/status` 检查挂载状态                                                            |
| VM 已关闭但状态显示 running | 重启服务或检查 VM 是否异常退出                                                                                             |
| 需要 SSH 进入 VM 调试     | Orion-scheduler 会自动注入 `ssh_public_key_path` 配置的公钥对应的私钥访问权限。使用 `ssh -i ~/.ssh/orion_vm_access root@<vm-ip>` 连接 |


