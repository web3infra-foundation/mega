# Antares FUSE 挂载问题

Orion Worker 长构建（Buck2 + Antares overlay）在 `/data/scorpio/antares/mnt/<task_id>-N/` 上曾出现 **FUSE ENOENT**，导致全量 `rustc` 构建失败（exit 3）。Build **#45** 在 discovery 修复后首次在同一 CL 上 **0× ENOENT、exit 0** 完成全量编译；本文档记录现象、与 discovery 的交互、已落地补丁与验收结果。

Target discovery 收窄与 all-added 子项目逻辑见 [TARGET_DISCOVERY_SCOPE.md](./TARGET_DISCOVERY_SCOPE.md)。

---

## 构建记录

### 早期（discovery 未收窄 + JVM toolchain）

| 构建 | 特点 | 结果 |
|------|------|------|
| #21–#23 | ~35–49 min；`linker_wrapper` ENOENT 5–7% | FAILED |

### CL `UYXIYYNJ`（`rk8s/**` 子项目）

| 构建 | Task ID（前缀） | Action / Commands | `os error 2` | 结果 |
|------|----------------|-------------------|--------------|------|
| #39 | `019ef246` | ~39k | — | 失败（discovery 范围过大） |
| #40 | `019ef252` | ~18k，真 `rustc` | 有 | **FAILED**（FUSE ENOENT） |
| #42 | `019ef325` | ~113，仅 `:vendor` | 0 | exit 0，**未编译**（discovery 误选，已修） |
| **#45** | `019ef333` | **23,270** action；**8,035** local commands | **0** | **exit 0，全量 `rustc` 通过** |

#### Build #45 摘要（`019ef333-db97-70e2-8960-40bcc5cc2496`，VM `192.168.221.108`）

- **总时长** ~73 min（其中 buck2 build ~72 min）；discovery 28 targets，自 `rk8s/` 子项目根构建。
- **FUSE 构建期**：`os error 2` **0**、`Action failed` **0**、`<unspecified>` **0**（日志共 17,697 行含该 task）。
- **编译证据**：日志中 **3,025** 处 `rustc` 相关 action；收尾为 `root//project/rkl:rkl`、`rkforge`、`slayerfs` 的 `rustc link [pic]`。
- **缓存**：`Cache hits: 0%`；`Commands: 8035 (cached: 0, remote: 0, local: 8035)`（`--no-remote-cache` + 新 isolation-dir）。
- **收尾**：构建成功后 `fusermount -u` 报 **Device or resource busy**，fallback 后 unmount 成功（不影响 exit 0）。

#23 指标（历史参考）：`os error 2` **80**、`Action failed` **80**。

---

## 问题描述

### 现象 A：写后立刻 stat ENOENT

buck2 写出 `linker_wrapper.sh` 后，紧接 `metadata()` / `set_permissions()` 报 ENOENT：

```
`write_file` setting executable `.../buck-out/buck-isolation-.../linker_wrapper.sh`:
    metadata(.../linker_wrapper.sh): No such file or directory (os error 2)
```

- 路径：`buck-out/buck-isolation-.../art/root/third-party/rust/crates/**` 或 `project/**`（upper passthrough）
- 多出现在高并发 action burst 初期

### 现象 B：延迟访问 shim ENOENT

`write __cc_shim.sh` / `__cxx_shim.sh` 在数分钟至数十分钟后，cc-rs 执行时报：

```
error occurred in cc-rs: failed to find tool ".../__cc_shim.sh": No such file or directory (os error 2)
```

典型 crate：`libsqlite3-sys`、`ring`、`zstd-sys` 等。写出与执行间隔可达 **~40 min**。可用 `ORION_RETAIN_ANTARES_MOUNTS=1` 对比 upper 与 mnt 是否「磁盘有、FUSE 无」。

### 根因假设（未完全证实）

`/data/scorpio/antares/mnt/...` = dicfuse lower + passthrough upper。FORGET 后内存 inode 与 upper 磁盘可能脱节；补丁将早期 ~72 次 ENOENT/构建降至个位数百分比，但未清零：

1. 高并发 `linker_wrapper` burst 下 overlay lookup / dentry 仍偶发 ENOENT。
2. shim 长间隔 `open`/`access`，`materialize_node_by_path` 可能未覆盖实际路径。
3. 隔离压测（仅写 buck-out）0 失败；全量构建含 dicfuse 读压 + buck2 daemon + 大量 action。

---

## Discovery 与 FUSE 的交互

| Discovery 行为 | 对 FUSE 压力的影响 |
|----------------|-------------------|
| 全 cell + `SelectAll`（#39） | 数万 vendor unpack action，放大写 burst |
| `ScopedNew` / 真 `rustc`（#40） | action 减少但仍大量编译，ENOENT 暴露 |
| owner → `:vendor` only（#42） | action 极少，**绕过**编译，不能验证 FUSE + rustc |
| `project/` only + rust 映射（#45） | 28 个 `rust_*` 根 + 传递 `third-party` 依赖；23k action、~72 min；**本次 0× ENOENT** |

收窄 discovery **不能**单独解决 ENOENT，但可减少无关 action。#45 表明在 discovery 修复 + 现有 unionfs 补丁下，**单次全量 `rustc` 构建可以不在 FUSE 上触发 ENOENT**；仍须更多构建观察是否偶发回归。

---

## 已落地补丁

| 改动 | 位置 | 说明 |
|------|------|------|
| `materialize_child_from_layers` | `rk8s/.../unionfs/mod.rs` | ENOENT 部分缓解 |
| `do_lookup` 在 `stat64` 前 pin | 同上 | 同上 |
| `materialize_node_by_path` + `inode_paths` | `unionfs/mod.rs` + `inode_store.rs` | 延迟 shim 仍可能失败 |
| discovery 后卸载 `old` mount | `orion/src/buck_controller.rs` | 释放双挂载；不消除 ENOENT |
| `writeback=false`、TTL 5s / dicfuse 60s | passthrough + `scorpio.toml` | EBADF 已解决 |
| Scheme C2 platform | `orion/buck/platform.rs` `--config` | `<unspecified>` 已解决 |
| Target discovery A/B/子项目 | `discovery_scope.rs`、`buck_controller.rs` | 减少无关 action；见姊妹文档 |

---

## 待验证方向

1. `ORION_RETAIN_ANTARES_MOUNTS=1` 在**失败**构建后对比 upper 与 mnt 上 shim / `linker_wrapper`（#45 已成功，暂无失败样本）。
2. unionfs/passthrough 在 `open`/`getattr` 路径强制回落 upper backing layer。
3. 多次复跑 `UYXIYYNJ` / 其他 CL，观察 ENOENT 是否偶发回归（#45 为单点成功）。
4. `ORION_BUCK_REMOTE_CACHE=1` 对重复构建耗时与 FUSE 读压的影响（#45 为 0% cache，~73 min）。
5. 构建后 `fusermount` **EBUSY** 与 fuse task 5s 超时（#45 出现，unmount 仍成功）。

---

## 构建验收

Worker 日志：`/home/orion/orion-runner/log/orion.log`（或 `GET /logs/orion/stream`）。

```bash
TASK=<build_id>
LOG=/home/orion/orion-runner/log/orion.log

# ENOENT / 失败 action
grep "$TASK" "$LOG" | grep -c "os error 2"
grep "$TASK" "$LOG" | grep -c "Action failed"

# 是否真在编译（不应只有 :vendor symlink）
grep "$TASK" "$LOG" | grep "buck2 stderr" | grep -E "rustc|rust_library|rust_binary" | head

# discovery 目标（应为 rust_* 而非 :vendor）
grep "$TASK" "$LOG" | grep -E "Target discovery|owner_seed|in-graph rdeps"
```

---

## 回归压测（`scorpiofs/tests/antares_test.rs`）

需 root/`/dev/fuse`；VM 上 `systemctl stop orion-runner` 后运行。

| 测试 | 说明 |
|------|------|
| `test_fuse_write_then_metadata` | 写后立即 stat |

---

## 相关文件

| 路径 | 说明 |
|------|------|
| `rk8s/project/libfuse-fs/src/unionfs/mod.rs` | materialize、`do_lookup` |
| `rk8s/project/libfuse-fs/src/unionfs/inode_store.rs` | `inode_paths` |
| `scorpiofs/src/antares/fuse.rs` | Antares FUSE 集成 |
| `orion/src/buck_controller.rs` | 双挂载、卸 old、`ORION_RETAIN_ANTARES_MOUNTS` |
| `scorpiofs/tests/antares_test.rs` | FUSE 压测 |
| [orion/docs/TARGET_DISCOVERY_SCOPE.md](./TARGET_DISCOVERY_SCOPE.md) | discovery 收窄与 #39–#42 |

---

## 已关闭问题

| 问题 | 验证 |
|------|------|
| `event.jsonl` EBADF | #21–#23：0× `os error 9` |
| `manifest_parse (<unspecified>)` | C2 platform 注入后：0× `<unspecified>` |
| `buck2 build --config` CLI | 已修 |
| TTL=0 吞吐劣化 | 勿将 TTL 设为 0 |
| 从 monorepo 根跑 rk8s buck2 | 子项目 `.buckconfig` 检测（#37） |
| #42 浅层成功（仅 `:vendor`） | `normalize_owner_targets_to_rust` |
| #45 全量 `rustc` on FUSE | `019ef333`：0× ENOENT，exit 0（单点验证） |

---

## 修订历史

| 日期 | 说明 |
|------|------|
| 2026-06-15 | 初稿 |
| 2026-06-17–18 | ENOENT 根因、补丁、压测；#22/#23 |
| 2026-06-23 | 补充 #39–#42、discovery 与 FUSE 关系、验收命令 |
| 2026-06-23 | **#45**：全量 rustc 首次 exit 0、0× ENOENT；fusermount EBUSY 收尾 |
