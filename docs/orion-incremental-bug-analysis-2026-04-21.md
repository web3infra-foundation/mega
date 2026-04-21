# Orion 增量编译问题定位与修复记录（2026-04-21）

## 1. 目标

确保托管仓库变更通过 `changes` 正确传递到 Orion，并被正确识别为：

- 应触发增量编译
- 不应触发增量编译

重点覆盖：

- `BUCK` 范围内源码修改
- 包级关键文件修改（`BUCK`、`Cargo.toml`、`Cargo.lock`、`.buckroot`、`.buckconfig`）
- `toolchains/BUCK`
- 无关文件新增

## 2. 端到端链路分析

### 2.1 Ceres

- 入口：`ceres/src/build_trigger/changes_calculator.rs`
- 作用：从 commit diff 生成 `Vec<Status<ProjectRelativePath>>`
- 行为：
  - repo 内路径归一化为 repo-relative
  - repo 外共享路径保持 monorepo-relative
  - 过滤不安全路径（`..`、`//` 等）

### 2.2 Orion-Server

- 入口：`orion-server/src/service/api_v2_service.rs`
- 作用：任务入队前再次规范化 `changes`（同样保持“repo 内相对、repo 外共享”契约）
- 下发：通过 WebSocket `WSMessage::TaskBuild` 发送 `repo/cl_link/changes`

### 2.3 Orion Worker

- 接收：`orion/src/ws.rs`
- 执行主流程：`orion/src/buck_controller.rs`
  - 挂载 old/new 视图（Antares/Dicfuse）
  - `get_build_targets(...)` 计算 impacted targets
  - 无目标则跳过构建；有目标则执行 `buck2 build`
- 变更解析：`orion/src/repo/changes.rs`
  - `Changes::contains_package(...)` 用于 package-level 触发判断

### 2.4 Mono

- 作用：API 路由与服务组装（`mono/src/api/**`），触发服务最终走 Ceres build_trigger 链路。
- 结论：本次问题核心不在 Mono 路由层。

## 3. 根因定位

### 3.1 现象

在 package-level 文件集合中，`.buckconfig` 已覆盖，但 `.buckroot` 未被识别为 package-level 文件。

### 3.2 影响

当仅修改 `.buckroot` 时，可能出现：

- 变更已进入 `changes`
- 但 `contains_package` 不认为该文件会影响 package
- 导致 impacted target 识别不完整或漏触发

这与预期不一致：`.buckroot` 作为 Buck 根配置关键文件，应参与 package-level 触发。

## 4. 代码修复

### 4.1 修改文件

- `orion/src/repo/changes.rs`
  - 将 `.buckroot` 纳入 `is_package_level_file(...)` 配置文件集合
  - 更新注释说明
  - 新增测试：`test_contains_package_with_buckroot`
  - 扩展测试：`test_is_package_level_file_config` 增加 `.buckroot`

- `orion/src/buck_controller.rs`
  - 新增端到端目标识别测试：
    `test_get_build_targets_detects_buckroot_change_as_package_level_impact`
  - 验证 `.buckroot` 变更会产生 impacted targets（`root//:explicit_main`、`root//:globbed_lib`）

## 5. 本地验证结果

已执行并通过：

1. `cargo test -p orion buckroot`
   - 通过 `test_contains_package_with_buckroot`

2. `cargo test -p orion is_package_level_file_config`
   - 配置文件分类测试通过

3. `cargo test -p orion test_get_build_targets_detects_buckroot_change_as_package_level_impact -- --nocapture`
   - 端到端目标识别测试通过

## 6. 与 FUSE/挂载相关观察

从 Orion 代码与日志关键点看：

- 挂载入口：`orion/src/antares.rs`、`orion/src/buck_controller.rs`
- 已有可观测点：
  - `Antares mount created successfully`
  - `Dicfuse warmup completed`
  - `Build repo root (...) does not exist`（路径异常）
- 建议在实测期继续关注：
  - `build_change_path_prefix_mismatch`
  - `Remapping unresolved repo-local change path`

若出现频繁 remap/prefix mismatch，说明 `changes` 路径契约与挂载视图可能仍有偏差，需要继续收敛。

## 7. 部署后实测步骤（与测试矩阵配合）

1. 编译部署 Orion：

```bash
cd /home/jackie/mega
cargo build --release -p orion && rm -rf ../orion-web/orion && cp target/release/orion ../orion-web/ && cd ../orion-web && sudo ./restart-and-clean.sh && cd ../mega
```

2. 按文档 `docs/orion-incremental-cl-test-matrix-2026-04-21.md` 串行执行 CL。

3. 每条用例结束后记录：
   - `cl_link`
   - 预期/实际（触发、成功/失败）
   - 关键日志片段

4. 一旦首个 FAIL 出现：停止后续批量用例，继续 bug 定位并更新本文件。

## 8. 当前结论

- 已定位并修复一个明确触发缺口：`.buckroot` 未参与 package-level 触发。
- 已补齐单元测试与端到端目标识别测试。
- 下一阶段应进行真实 CL 串行回归，验证完整矩阵是否全部符合预期。

## 9. 新增关键发现（重启 worker 后）

### 9.1 操作

按以下命令完成了完全清理与重启：

```bash
cd /home/jackie/mega && cargo build --release -p orion && rm -rf ../orion-web/orion && cp target/release/orion ../orion-web/  && cd ../orion-web && sudo ./restart-and-clean.sh && cd ../mega
```

服务状态确认：`orion-web-worker.service` 为 active，WebSocket 握手成功。

### 9.2 结果

重启后任务下发恢复正常（CL 到 worker 的 `TaskBuild` 基本秒级可见），但出现更严重的不一致：

1. `src/main.rs` 故意语法错误用例（T02）预期应失败，实际连续两次构建成功。
2. 第二次提交前已强校验 `diff-preview` 明确包含错误行与 marker（`T02MARK_20260421T071848Z`）。
3. 任务完成后读取保留挂载文件：
  `/home/jackie/orion-web/antares/mnt/019daee7-f466-7fe0-a938-af51c1375b91-1/project/buck2_test/src/main.rs`
  其中不存在 marker 和错误行。

### 9.3 结论

当前高优先级问题已从“增量目标识别”转向“CL 内容到 worker 构建输入的数据一致性”：

- `changes` 路径下发是正确的（日志显示 `changes: [Modified(ProjectRelativePath("src/main.rs"))]`）。
- 但 worker 挂载视图中的文件内容与提交内容不一致，导致“故意错误修改”无法被构建捕获。

### 9.4 下一步定位方向

1. 重点排查 Antares 根据 `cl_link` 拉取快照的实现，确认是否拿到了错误 commit/视图（例如回退到基线）。
2. 检查 Orion-Server 下发给 worker 的 `cl_link` 与 commit 关联是否存在漂移。
3. 在 worker 挂载后、buck2 构建前，增加日志：打印关键文件的哈希（如 `src/main.rs`）并与 CL 文件 sha 对照。
4. 在触发测试脚本中增加“提交前后内容校验”：`diff-preview` 与挂载文件内容均需命中 marker，作为构建结果判定前置条件。
