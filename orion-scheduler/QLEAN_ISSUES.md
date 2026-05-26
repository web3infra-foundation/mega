# Qlean 问题分析

本文档记录了 `orion-scheduler` 开发过程中遇到的，需要修改 `qlean` crate 的问题。

---

## 修复建议汇总

| 问题 | 优先级 | 建议修复方案 |
|------|--------|--------------|
| KVM OnceLock 缓存 | 高 | 移除全局缓存，每次 VM 检查 |
| 目录权限 | 中 | 遵守 XDG_DATA_HOME，使用当前用户目录 |
| Guest CID 冲突 | 低 | 启动时清理或动态 CID |
| KVM 检测时序 | 中 | 在 VM 启动前懒加载检查 |
| SSH 超时硬编码 | 高 | 支持配置化超时，或减少无 KVM 时的 180 秒超时 |
| 优雅关闭信号处理 | 中 | 支持 SIGTERM/SIGQUIT，不只是 SIGINT |
| **qlean API 暴露** | 高 | 暴露目录查询、KVM 状态、MachineConfig 等接口 |
| **VM 删除后资源不回收** | 高 | 添加镜像清理 API 或在 orion-scheduler 中手动清理 |
| **自定义镜像复制瓶颈** | 高 | 使用 overlay 机制避免复制，或添加镜像缓存 |

## 1. KVM 检测警告

### 问题现象

```
WARN qlean::qemu: KVM is not available on this host. QEMU will run without hardware acceleration, which may result in significantly reduced performance.
```

### 根因分析

`qlean` 使用 `kvm-ioctls::Kvm::new()` 检测 KVM 可用性，并通过 `OnceLock` 缓存结果：

```rust
// qlean/src/lib.rs
static KVM_AVAILABLE: OnceLock<bool> = OnceLock::new();

pub async fn with_machine<'a, F, R>(image: &'a Image, config: &'a MachineConfig, f: F) -> Result<R>
{
    // ...
    KVM_AVAILABLE.get_or_init(|| Kvm::new().is_ok());
    // ...
}
```

检测结果被全局缓存。如果首次调用时 `Kvm::new()` 失败（即使是暂时性的），所有后续 VM 创建都会认为 KVM 不可用。

### 观察到的现象

1. **独立测试 KVM 成功**：在隔离环境中测试 `Kvm::new()` 时，它能正常工作并返回 API 版本 12。

2. **权限检查通过**：`/dev/kvm` 权限正确（`crw-rw---- root:kvm`），用户也在 `kvm` 组中。

3. **嵌套虚拟化已启用**：`/sys/module/kvm_intel/parameters/nested = Y`

4. **QEMU 可以使用 KVM**：当 QEMU 直接以 `-enable-kvm -cpu host` 启动时，能正常工作。

### 可能的修复方案

#### 方案 A：延迟重新检测

修改 `qlean`，在每次 VM 启动前重新检查 KVM 可用性，而不是全局缓存：

```rust
fn check_kvm_available() -> bool {
    Kvm::new().is_ok()
}
```

#### 方案 B：移除 OnceLock 缓存

每次调用 `Kvm::new().is_ok()` 时不再缓存，因为检查开销很小：

```rust
let kvm_available = Kvm::new().is_ok();
```

#### 方案 C：提供手动覆盖选项

添加环境变量或配置选项来覆盖 KVM 检测：

```rust
let kvm_available = match std::env::var("QLEAN_FORCE_KVM") {
    Ok(v) => v == "true",
    _ => Kvm::new().is_ok(),
};
```

## 2. 目录权限问题

### 问题现象

`qlean` 将数据存储在 `~/.local/share/qlean/`，当使用 `sudo` 启动时，该路径解析为 `/root/.local/share/qlean`。这导致之后以非 root 用户运行服务时出现权限拒绝错误。

### 根因分析

`qlean` 使用 `directories::ProjectDirs` 确定数据目录：

```rust
// qlean/src/utils.rs
impl QleanDirs {
    pub fn new() -> Result<Self> {
        let project_dir = ProjectDirs::from("", "", "qlean").expect("Couldn't get project dir");
        let data_dir = project_dir.data_dir().to_path_buf();
        // ...
    }
}
```

当使用 `sudo` 启动时，`HOME` 环境变量可能仍指向 `/root`，导致数据存储在 root 的目录中。

### 观察到的问题

1. `/home/ubuntu/.local/share/qlean/images/debian-13-generic-amd64/` 属于 `root:root`
2. `/var/log/orion-scheduler/` 属于 `root:root`

### 可能的修复方案

#### 方案 A：使用 XDG 基础目录规范

遵守 `XDG_DATA_HOME` 环境变量，并回退到用户主目录：

```rust
pub fn get_data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("qlean")
        })
}
```

#### 方案 B：要求显式配置

要求用户通过环境变量显式设置数据目录：

```rust
pub fn new() -> Result<Self> {
    let data_dir = std::env::var("QLEAN_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| panic!("Cannot determine data directory"))
                .join("qlean")
        });
    // ...
}
```

#### 方案 C：创建时设置正确权限

创建目录时，确保以正确的所有者创建：

```rust
pub fn create_dir(purpose: &str, path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path).expect("Failed to create directory");
        // 设置权限以允许当前用户访问
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(path, perms)?;
        }
    }
    Ok(())
}
```

## 3. Guest CID 冲突

### 问题现象

当 VM 未正确清理时，vsock guest CID 仍被占用：

```
ERROR qlean::qemu: qemu-system-x86_64: -device vhost-vsock-pci,id=vhost-vsock-pci0,guest-cid=10: vhost-vsock: unable to set guest cid: Address already in use
```

### 根因分析

QEMU 使用静态 guest CID（默认为 10）进行 vsock 连接。如果之前的 VM 进程未正确终止，CID 会保持预留状态。

### 当前临时解决方案

需要手动清理：

```bash
pkill -9 -f qemu-system-x86
```

### 可能的修复方案

1. **动态 CID 分配**：使用随机 CID 而非固定的 10
2. **启动时清理**：启动时检查并终止残留的 QEMU 进程
3. **更好的错误提示**：检测到 CID 冲突时提供更清晰的说明

## 4. KVM 检测时序问题

### 问题现象

`OnceLock` 模式意味着 KVM 检测发生在首次调用 `with_machine()` 或 `with_pool()` 时。如果进程在初始设置后需要降级权限，缓存的"不可用"结果将持续存在。

### 示例场景

1. 进程以 root 身份启动
2. `qlean` 被初始化（在某处依赖链中）
3. KVM 检测运行并失败（由于任何暂时性原因）
4. 进程降级权限到 ubuntu
5. 所有后续 VM 操作都报告 KVM 不可用

### 可能的修复方案

实现懒加载的、每个 VM 的 KVM 检查，而不是全局缓存：

```rust
pub async fn with_machine<'a, F, R>(image: &'a Image, config: &'a MachineConfig, f: F) -> Result<R>
{
    // 在启动 VM 前立即检查 KVM 可用性
    let kvm_available = Kvm::new().is_ok();
    // 仅为此 VM 实例使用 kvm_available
    // ...
}
```

## 5. VM 启动优化建议

### 问题现象

无 KVM 时 VM 启动非常慢，SSH 连接等待时间过长。

### 当前瓶颈分析

VM 启动耗时分布（无 KVM 情况）：

| 阶段 | 默认超时 | 说明 |
|------|----------|------|
| SSH 连接等待 | **180 秒** | qlean 硬编码，`machine.rs:609-613` |
| Orion 文件上传 | ~10-30 秒 | 477MB 通过 vsock |
| Cloud-init 初始化 | ~10-30 秒 | VM 内部初始化 |

### 优化方案

#### 方案 A：减少无 KVM 时的 SSH 超时

修改 `qlean/src/machine.rs`：

```rust
// 当前：无 KVM 时 180 秒
let ssh_timeout = if kvm_available {
    Duration::from_secs(60)
} else {
    Duration::from_secs(180)  // 太长了！
};

// 优化：减少到 60 秒
let ssh_timeout = if kvm_available {
    Duration::from_secs(60)
} else {
    Duration::from_secs(60)  // 改为 60 秒
};
```

#### 方案 B：添加环境变量配置

```rust
let ssh_timeout = if kvm_available {
    Duration::from_secs(60)
} else {
    std::env::var("QLEAN_SSH_TIMEOUT")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or(60)
};
```

#### 方案 C：优化 Orion 文件传输

考虑使用更快的文件传输方式（如并行传输、压缩）减少 Orion 上传时间。

## 6. 优雅关闭信号处理

### 问题现象

`qlean` 的 `Machine::shutdown()` 方法可能未正确处理 SIGTERM/SIGQUIT 信号。

### 可能的修复方案

在 `qlean` 中增强信号处理，支持多种优雅关闭信号：

```rust
impl Machine {
    pub async fn shutdown(mut self) -> Result<()> {
        // 发送 SIGTERM 而不是 SIGKILL
        self.process.signal(::std::os::unix::signal::SignalKind::terminate())?;
        // 等待进程退出
        tokio::time::timeout(Duration::from_secs(30), self.process.wait()).await??;
        Ok(())
    }
}
```

## 7. 调试建议

如果需要进一步调试 qlean 的 KVM 检测问题，可以使用以下方法：

### 方法 1：独立测试程序

创建一个独立的测试程序来验证 KVM：

```rust
use kvm_ioctls::Kvm;

fn main() {
    match Kvm::new() {
        Ok(kvm) => {
            println!("KVM created successfully");
            println!("KVM API version: {}", kvm.get_api_version());
        }
        Err(e) => {
            println!("KVM new failed: {:?}", e);
        }
    }
}
```

### 方法 2：检查进程 capability

```bash
cat /proc/<pid>/status | grep -i cap
```

确保 `CapBnd` 包含 `CAP_SYS_ADMIN`。

### 方法 3：检查 AppArmor 状态

```bash
aa-status
cat /proc/self/attr/current
```

## 8. qlean API 接口设计

为支持 `orion-scheduler` 多 VM 调度场景，建议在 qlean crate 中暴露以下接口：

### 8.1 状态查询接口

```rust
// qlean/src/lib.rs

/// 查询 KVM 是否可用（每次重新检测，不使用缓存）
pub fn is_kvm_available() -> bool {
    Kvm::new().is_ok()
}

/// 查询当前 KVM 缓存状态（供调试使用）
pub fn is_kvm_available_cached() -> Option<bool> {
    KVM_AVAILABLE.get().copied()
}
```

### 8.2 目录路径查询

```rust
// qlean/src/utils.rs

impl QleanDirs {
    /// 获取数据目录（支持 XDG_DATA_HOME）
    pub fn get_data_dir() -> PathBuf {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("qlean")
            })
    }

    /// 获取镜像目录
    pub fn get_images_dir() -> PathBuf {
        Self::get_data_dir().join("images")
    }

    /// 获取 runs 目录（VM 运行状态）
    pub fn get_runs_dir() -> PathBuf {
        Self::get_data_dir().join("runs")
    }
}
```

### 8.3 可配置的 MachineConfig

```rust
// qlean/src/machine.rs

pub struct MachineConfig {
    /// SSH 连接超时（无 KVM 时默认 60s，而非硬编码 180s）
    pub ssh_timeout: Duration,
    /// 强制启用/禁用 KVM（覆盖自动检测）
    pub kvm_enabled: Option<bool>,
    /// VM 内存大小（MB）
    pub memory_mb: u32,
    /// vCPU 数量
    pub cpus: u32,
    /// Guest CID（None 表示动态分配）
    pub guest_cid: Option<u32>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            ssh_timeout: Duration::from_secs(60),
            kvm_enabled: None,
            memory_mb: 4096,
            cpus: 2,
            guest_cid: None,  // 动态分配
        }
    }
}
```

### 8.4 动态 Guest CID 分配

```rust
// qlean/src/qemu.rs

impl QemuManager {
    /// 分配一个未使用的 guest CID
    pub fn allocate_guest_cid() -> Result<u32> {
        // 扫描现有进程，查找已使用的 CID
        // 返回 3-65535 范围内的随机可用 CID
    }
}
```

### 8.5 环境变量配置支持

```rust
// 支持的配置环境变量
const QLEAN_CONFIG_ENV_VARS: &[&str] = &[
    "QLEAN_SSH_TIMEOUT",      // SSH 超时（秒）
    "QLEAN_FORCE_KVM",        // 强制启用/禁用 KVM ("true"/"false")
    "QLEAN_DATA_DIR",         // 数据目录覆盖
    "QLEAN_GUEST_CID",        // 指定 Guest CID
];

impl MachineConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(timeout) = std::env::var("QLEAN_SSH_TIMEOUT") {
            if let Ok secs) = timeout.parse() {
                config.ssh_timeout = Duration::from_secs(secs);
            }
        }

        if let Ok(force) = std::env::var("QLEAN_FORCE_KVM") {
            config.kvm_enabled = Some(force == "true");
        }

        if let Ok(cid) = std::env::var("QLEAN_GUEST_CID") {
            if let Ok(cid_num) = cid.parse() {
                config.guest_cid = Some(cid_num);
            }
        }

        config
    }
}
```

### 8.6 推荐的最小暴露 API

| 函数/类型 | 文件 | 说明 |
|-----------|------|------|
| `is_kvm_available()` | `lib.rs` | 每次重新检测 KVM |
| `QleanDirs::get_data_dir()` | `utils.rs` | 获取数据目录 |
| `QleanDirs::get_images_dir()` | `utils.rs` | 获取镜像目录 |
| `MachineConfig::from_env()` | `machine.rs` | 从环境变量加载配置 |
| `MachineConfig { ssh_timeout, kvm_enabled, guest_cid }` | `machine.rs` | 可配置字段 |
| `allocate_guest_cid()` | `qemu.rs` | 动态 CID 分配 |

## 9. VM 删除后资源不回收

### 问题现象

`~/.local/share/qlean/images/` 下的自定义镜像在 VM 删除后不会被清理，每次部署都会残留约 2.5GB 的 qcow2 镜像文件：

```
~/.local/share/qlean/images/
├── custom-orion-vm-1778587647/   ← 2.5GB 残留
├── custom-orion-vm-1778588170/   ← 2.5GB 残留
├── custom-orion-vm-1778588655/   ← 2.5GB 残留
├── debian-13-buck2/              ← 官方基础镜像（正常）
└── debian-13-generic-amd64/      ← 官方基础镜像（正常）
```

同时 `~/.local/share/qlean/runs/` 下也有残留的目录和 overlay.img 文件。

### 根因分析

`Machine::shutdown()` 只做了两件事：

1. **关闭 VM** - 通过 `systemctl poweroff` 关闭客户机系统
2. **清理 runs 目录** - 如果 `config.clear = true`，只删除 `runs/{machine_id}/`

```rust
// machine.rs:887-891 (Drop impl)
if self.config.clear {
    let dirs = QleanDirs::new().expect("Failed to get QleanDirs in Drop");
    let run_dir = dirs.runs.join(&self.id);
    let _ = std::fs::remove_dir_all(run_dir);  // 只删 runs，不删 images
}
```

但 `images/` 目录中的自定义镜像（通过 `create_custom_image()` 创建）**从未被清理**。在 `orion-scheduler` 的 `KeepAliveMachine::new()` 中：

```rust
// keep_alive.rs:81
qlean::create_custom_image(&format!("custom-{}", vm_name), image_config).await?
```

这会在 `~/.local/share/qlean/images/custom-{vm_name}/` 下生成完整的 qcow2 镜像（约 2.5GB）。VM 关闭后，这个目录**永久保留**。

### 资源泄漏情况

| 目录 | 内容 | 清理方式 | 状态 |
|------|------|----------|------|
| `runs/{id}/` | overlay.img (COW) | `shutdown()` 时删除 | ✅ 正常 |
| `images/{name}/` | 完整 qcow2 + kernel/initrd | **从不删除** | ❌ 残留 |

### 可能的修复方案

#### 方案 A：在 qlean 中添加镜像清理 API

在 `qlean` 中暴露镜像删除接口：

```rust
// qlean/src/lib.rs
/// Delete a custom image by name
pub async fn delete_custom_image(name: &str) -> Result<()> {
    let dirs = QleanDirs::new()?;
    let image_dir = dirs.images.join(name);
    if image_dir.exists() {
        tokio::fs::remove_dir_all(&image_dir).await?;
    }
    Ok(())
}
```

#### 方案 B：在 orion-scheduler 中手动清理

在 `KeepAliveMachine::shutdown()` 后，根据 `vm_name` 推算镜像目录并删除：

```rust
// keep_alive.rs
pub async fn shutdown_and_cleanup(self) -> Result<()> {
    self.shutdown().await?;

    // 清理对应的 images 目录
    let dirs = QleanDirs::new()?;
    let image_name = format!("custom-{}", self.vm_name);
    let image_dir = dirs.images.join(&image_name);
    let _ = tokio::fs::remove_dir_all(&image_dir).await?;
    Ok(())
}
```

#### 方案 C：区分基础镜像和临时镜像

- 基础镜像（如 `debian-13-generic-amd64`、`debian-13-buck2`）应持久保留
- 临时镜像（如 `custom-orion-vm-*`）在 VM 删除后应自动清理

在配置中区分镜像类型：

```rust
pub struct CustomImageConfig {
    pub image_source: ImageSource,
    pub image_hash: String,
    pub persistent: bool,  // 新增：是否持久化
}
```

## 10. 自定义镜像复制瓶颈

### 问题现象

启动自定义镜像的 VM 时，从日志观察到以下时间线：

```
02:35:40 - 使用自定义镜像
02:36:36 - 镜像 hash 计算完成 (~55s)
02:41:21 - VM 初始化开始 (~4m45s 空档)
02:41:21 - SSH 连接开始
02:42:01 - SSH 连接成功 (~40s)
```

从 hash 计算完成到 VM 初始化开始之间有 **4m45s 空档**。

### 根因分析

`qlean::create_custom_image()` 中会复制镜像文件：

```rust
// image.rs:900-912 (Custom::download)
ImageSource::LocalPath(src) => {
    tokio::fs::copy(src, dest).await?;  // 复制整个 2.5GB 文件！
}
```

每次创建 VM 时，qlean 会把源镜像复制到 `~/.local/share/qlean/images/custom-orion-vm-xxx/` 目录：

```
源: /home/ubuntu/.local/share/qlean/images/debian-13-buck2/debian-13-buck2.qcow2 (2.5GB)
    ↓ copy
目的: ~/.local/share/qlean/images/custom-orion-vm-xxx/custom-orion-vm-xxx.qcow2
```

这就是 4m45s 空档的原因！2.5GB 文件复制约需 4-5 分钟。

### 完整耗时分解

| 阶段 | 耗时 | 原因 |
|------|------|------|
| 镜像 hash 计算 | ~55s (优化后几秒) | 流式读取 2.5GB |
| **文件复制** | **~4m45s** | **复制 2.5GB 到新目录** |
| SSH 连接等待 | ~40s | VM boot + SSH 服务就绪 |

### 可能的修复方案

#### 方案 A：直接使用源镜像 + overlay（推荐）

修改 `Custom::download` 的 `LocalPath` 逻辑，不复制文件，只创建指向源镜像的路径：

```rust
ImageSource::LocalPath(src) => {
    // 不复制，直接使用源文件路径
    // overlay 机制已经实现了 COW，不需要复制整个镜像
}
```

但需要注意 `ImageMeta` 的设计假设文件在 `image_dir` 下，可能需要较大改动。

#### 方案 B：镜像缓存 + 引用计数

检测到源镜像未变化时，跳过复制：

```rust
if let ImageSource::LocalPath(src) = &self.config.image_source {
    let src_hash = compute_sha256_streaming(src).await?;
    let cache_key = format!("{:x}", src_hash);
    // 检查缓存是否存在且有效
    // 如果有效，直接使用缓存路径
}
```

#### 方案 C：延迟复制

只在确实需要修改镜像时才复制，初始只创建 overlay：

```rust
// Machine::new() 中已经创建了 overlay.img
// overlay 引用源镜像，不需要完整复制
```

### 相关问题

- 此问题与 "VM 删除后资源不回收" 关联：每次创建的镜像副本都残留磁盘空间
- 使用 overlay 机制可以同时解决两个问题
