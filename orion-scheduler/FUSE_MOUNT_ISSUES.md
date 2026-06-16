# Antares FUSE 挂载问题分析

本文档记录在 `orion-scheduler` 驱动的 buck2 构建过程中，构建工作目录所在的 antares/scorpio FUSE 挂载（`/data/scorpio/antares/mnt/<build_id>-N/`）出现的稳定性问题。这些问题不会每次都直接中断构建，但会放大失败、干扰诊断，长构建下尤其明显。

> 首次记录来源：线上测试服务 `git.xuanwu.openatom.cn`（k3s 集群 `buck2hub` 命名空间），CL `UYXIYYNJ` 第三次构建 `019ec954-e2b3-7520-b780-a14382f6ed24`（2026-06-15 03:30 → 04:07，exit_code=1）。

---

## 问题汇总


| 现象                          | errno                 | 出现次数（该次构建） | 影响                    | 优先级 |
| --------------------------- | --------------------- | ---------- | --------------------- | --- |
| 刚写出的文件立即 `metadata()` 报不存在  | `os error 2` (ENOENT) | 45         | 本地 action 失败、可能误判构建失败 | 高   |
| 收尾 flush `event.jsonl` 句柄失效 | `os error 9` (EBADF)  | 2          | buck2 无法写自身事件日志，收尾报错  | 中   |
| `Action failed` 总数          | —                     | 47         | 干扰诊断、放大失败面            | 高   |


> 注意：本次构建的**决定性失败**是工具链问题（worker 缺少 `jlink`，见文末「与构建失败的关系」），FUSE 问题是次要但需要单独跟踪的稳定性隐患。

---

## 1. 写后立即读取返回 ENOENT（一致性问题）

### 问题现象

构建过程中大量本地 action（如 `write linker_wrapper.sh`）刚写出文件，buck2 紧接着对同一路径做 `metadata()`（设置可执行权限）时却报“文件不存在”：

```
Action failed: root//third-party/rust/crates/thiserror/2.0.18:thiserror-build-script-build (write linker_wrapper.sh)
`write_file` setting executable `/data/scorpio/antares/mnt/019ec954-.../.../linker_wrapper.sh`:
    metadata(/data/scorpio/antares/mnt/019ec954-.../.../linker_wrapper.sh): No such file or directory (os error 2)
```

同类还有 `remove_file(...): No such file or directory (os error 2)`。

### 根因分析

`/data/scorpio/antares/mnt/...` 是 antares/scorpio 提供的 FUSE 工作目录。错误模式是典型的 **“写后读不一致”（read-after-write inconsistency）**：

- buck2 在挂载点 `write_file` 成功后，立刻 `stat`/`metadata` 同一路径；
- FUSE 层因元数据缓存、异步落盘或目录项尚未可见，返回 ENOENT；
- 高并发（buck2 同时跑成百上千个 action）下概率显著上升。

### 影响

- 这些是本地 `write_file` action，buck2 会记为 `Action failed`；
- 即使部分被重试，也会拖慢构建、污染日志，难以与“真实的构建错误”区分。

### 建议排查 / 修复方向

- 检查 antares/scorpio FUSE 实现的元数据缓存策略，确认 `write` 后 `lookup`/`getattr` 的一致性保证；必要时关闭/缩短 attr/entry cache 的 TTL。
- 确认是否存在异步写入（writeback）导致目录项延迟可见；写关键控制文件时考虑同步语义。
- 评估并发压力下的表现，复现“写后立即 stat”的竞态。

---

## 2. 收尾 flush `event.jsonl` 报 Bad file descriptor

### 问题现象

构建末尾 buck2 flush 自身事件日志失败，随后整体 `BUILD FAILED`：

```
BUILD FAILED
 WARN buck2_event_log::writer: Failed to flush log file at
   `/data/scorpio/antares/mnt/019ec954-.../event.jsonl`: Bad file descriptor (os error 9)
Command failed: Error flushing log file at /data/scorpio/antares/mnt/019ec954-.../event.jsonl

Caused by:
    Bad file descriptor (os error 9)
```

### 根因分析

`Bad file descriptor (EBADF)` 表示文件句柄在 flush 时已失效。在 FUSE 挂载上通常意味着：

- 挂载在构建收尾阶段被卸载 / 重新挂载，或连接中断（transport endpoint 变化）；
- 句柄被底层提前关闭，而上层仍持有并写入。

该 `event.jsonl` 是 buck2 自己的事件日志，位于挂载工作目录内，因此挂载的句柄稳定性直接影响 buck2 收尾。

### 影响

- 即使所有目标都已构建，buck2 也会因无法落盘事件日志而以失败收尾；
- 句柄失效往往与挂载生命周期管理（卸载时机）相关。

### 建议排查 / 修复方向

- 检查 antares 挂载的生命周期：是否在构建进程仍在写入时就触发了卸载 / 清理。
- 确认 `fusermount` 卸载时机晚于构建进程完全退出（参考 `orion` 中 antares 卸载相关逻辑）。
- 评估把 buck2 的 event-log 输出目录放到挂载之外的本地稳定路径，避免受挂载卸载影响。

---

## 与构建失败的关系（区分主因）

需要明确区分：CL `UYXIYYNJ` 该次构建的**决定性失败原因不是 FUSE**，而是 worker 的 Java 17 运行时缺少 `jlink`，导致 `create_jdk_system_image` 工具链 action 失败：

```
Action failed: root//project/buck2_test/toolchains:jdk_system_image (create_jdk_system_image)
FileNotFoundError: [Errno 2] No such file or directory:
    PosixPath('/usr/local/java-runtime/impl/17/bin/jlink')
```

本文档聚焦的是**次要但需独立跟踪**的 FUSE 挂载稳定性问题：它会产生大量噪声 action 失败、并在收尾阶段以 EBADF 破坏事件日志，从而放大整体失败面、增加定位成本。即便修复了 `jlink`，FUSE 问题仍可能在长构建中独立导致失败，建议单独跟进。

---

## 复现与取证

- 构建本地日志（orion-server `mix` 模式）：`<build_log_dir>/<task_id>/<repo_leaf>/<build_id>.log`
  - 示例：`/tmp/megadir/buck2ctl/019ebaad-2fa4-7310-a027-13a1616483af/019ec954-e2b3-7520-b780-a14382f6ed24.log`
- 关键字检索：

```bash
grep -niE "bad file descriptor|os error 9|os error 2|fuse|transport endpoint|input/output error|scorpio|antares|Action failed|BUILD FAILED" <build_log>
```

- 统计错误规模：

```bash
grep -c "os error 2" <build_log>   # ENOENT 写后读不一致
grep -c "os error 9" <build_log>   # EBADF  句柄失效
grep -c "Action failed" <build_log>
```

