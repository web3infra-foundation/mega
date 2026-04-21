# Orion 增量编译 CL 测试分析矩阵（阶段一）

日期：2026-04-21  
适用范围：`https://app.gitmega.com/mega/code` 上的 `/project/buck2_test` 及其相关文件  
目标：在真实 CL 流程中，稳定验证“哪些变更应该触发增量编译、哪些不应该触发”，并能通过本地 Orion Worker 最新日志快速判定。

## 1. 测试执行纪律（必须遵守）

1. 串行执行：一次只提交 1 个 CL。
2. 必须等待当前 CL 构建完成并判定“符合预期”后，才开始下一个 CL。
3. 若当前 CL 结果不符合预期：立即停止后续用例，进入 bug 定位阶段。
4. 日志分析只看最新日志：每次提交前记录时间戳；提交后仅过滤该时间戳之后日志。

## 2. CL 提交最小闭环（已验证 API）

基于 `gitmega-cl-code-api-guide.md`：

1. `GET /blob` 读取原文件
2. `POST /edit/diff-preview`
3. `POST /edit/save`（拿到 `cl_link`）
4. `GET /cl/{link}/files-list`
5. `GET /cl/{link}/merge-box`

注意：必须使用 `edit/save` 返回的 `cl_link` 做后续关联，不要从 `cl/list` 猜测。

## 3. 日志观察方法（本地 orion worker）

日志文件：`/home/jackie/orion-web/log/orion.log`

建议命令（仅看最新）：

```bash
# 1) 提交前记录时间点
TS="$(date -u +%Y-%m-%dT%H:%M:%S)"

# 2) 提交 CL 并等待一段时间后，查看 TS 之后与任务判定有关的关键日志
awk -v ts="$TS" '$0 >= ts' /home/jackie/orion-web/log/orion.log \
  | grep -E "TaskBuild|Analyzing changes|Found impacted targets|No impacted Buck targets|Starting buck2 build|buck2 stderr|Buck2 build completed successfully|Buck2 build failed|build_change_path_prefix_mismatch|Remapping unresolved"

# 3) 持续跟随最新日志
 tail -f /home/jackie/orion-web/log/orion.log
```

推荐关键词（DEBUG 已开启）：

- 触发链路：`Received task` `Analyzing changes` `Changes`
- 增量识别：`Found impacted targets` `No impacted Buck targets detected`
- 路径异常：`build_change_path_prefix_mismatch` `Remapping unresolved repo-local change path`
- 构建结果：`Starting buck2 build` `Buck2 build completed successfully` `Buck2 build failed`
- 挂载/数据面：`Antares mount created successfully` `Dicfuse`

## 4. 判定规则

- 应触发增量：应出现 `Found impacted targets`，且进入 `Starting buck2 build`。
- 不应触发增量：应出现 `No impacted Buck targets detected for the provided changes.`，且跳过构建。
- 预期成功：最终出现 `Buck2 build completed successfully`。
- 预期失败：最终出现 `Buck2 build failed`，且失败原因与本次故意错误一致（语法/配置错误）。

## 5. 测试矩阵（第一批必测）

说明：
- “正确修改”=语义安全（例如注释、格式、合法配置）。
- “错误修改”=故意引入语法/格式错误。
- “应触发增量”强调触发行为正确性；“预期构建结果”强调成功/失败。

| ID | 文件 | 修改类型 | 应触发增量 | 预期构建结果 | 说明 |
|---|---|---|---|---|---|
| T01 | `src/main.rs` | 正确修改（加注释） | 是 | 成功 | 典型 repo 内源码变更 |
| T02 | `src/main.rs` | 错误修改（Rust 语法错） | 是 | 失败 | 典型源码错误应直接暴露 |
| T03 | `src/generated/new_module.rs`（新增，相关） | 正确新增 | 是 | 成功 | 新文件位于 BUCK 规则/glob 范围内 |
| T04 | `src/generated/new_module.rs`（新增，相关） | 错误新增（语法错） | 是 | 失败 | 覆盖“新增文件+语法错” |
| T05 | `notes/added.txt`（新增，无关） | 正确新增 | 否 | 成功（跳过构建） | 不在 BUCK 影响范围内 |
| T06 | `BUCK` | 正确修改 | 是 | 成功 | 包级规则变更应触发 |
| T07 | `BUCK` | 错误修改（语法/规则错） | 是 | 失败 | 规则错误应可见 |
| T08 | `Cargo.toml` | 正确修改 | 是 | 成功 | 包级配置变更应触发 |
| T09 | `Cargo.toml` | 错误修改（TOML 错） | 是 | 失败（或明确配置解析报错） | 若未失败需重点排查 |
| T10 | `Cargo.lock` | 正确修改 | 是 | 成功 | 依赖锁文件变更应触发 |
| T11 | `Cargo.lock` | 错误修改（格式错） | 是 | 失败或成功（取决于是否被解析） | 关键是“应触发”；结果需记录 |
| T12 | `.buckconfig` | 正确修改 | 是 | 成功 | Buck 配置应触发 |
| T13 | `.buckconfig` | 错误修改 | 是 | 失败 | Buck 初始化/解析应报错 |
| T14 | `.buckroot` | 正确修改 | 是 | 成功 | 仓库根标识变更应触发 |
| T15 | `.buckroot` | 错误修改 | 是 | 失败或显式异常 | 关键是应触发并可观测异常 |
| T16 | `.buckconfig` | 正确修改（仅注释） | 是 | 成功 | 验证最小配置改动 |
| T17 | `toolchains/BUCK` | 正确修改 | 是 | 成功 | 子 cell/toolchain 包规则应触发 |
| T18 | `toolchains/BUCK` | 错误修改 | 是 | 失败 | 覆盖 toolchains cell |

## 6. 每次 CL 的记录模板（建议）

```markdown
### Case: Txx
- cl_link:
- 提交时间(UTC):
- 文件与改动摘要:
- 预期: 触发=是/否, 构建=成功/失败
- 实际: 触发=是/否, 构建=成功/失败
- 关键日志片段:
- 结论: PASS / FAIL
- 若 FAIL: 立即停止批量测试并进入 bug 分析
```

## 7. 失败即转入 Bug 分析的条件

出现以下任一项即停止后续测试：

1. 应触发却未触发（错误地 `No impacted`）。
2. 不应触发却触发了构建（误触发）。
3. 故意语法错误却构建成功（漏检）。
4. 正确修改却构建失败（误报）。
5. 日志显示路径映射异常（例如重复 `build_change_path_prefix_mismatch`）并导致判定偏差。

## 8. 与本轮代码分析的直接关联

- 重点验证 `changes` 路径契约：repo 内应 repo-relative，repo 外应 monorepo-relative。
- 重点验证 Buck 关键文件变更（`BUCK`/`Cargo.toml`/`Cargo.lock`/`.buckroot`/`.buckconfig`/`toolchains/BUCK`）是否稳定触发。
- 若发现偏差，优先检查：
  - Ceres 侧 `changes` 计算与归一化
  - Orion-Server 侧二次归一化
  - Orion 侧 `Changes::contains_package` 与 remap 逻辑
  - Antares/Dicfuse 挂载路径及可见性

## 9. 已执行用例结果（持续追加）

### Case: T14
- cl_link: `WOH2VX7P`
- 提交时间(UTC): `2026-04-21T07:03:42`
- 文件与改动摘要: 修改 `.buckroot`，新增注释行（正确修改）
- 预期: 触发=是, 构建=成功
- 实际: 触发=是, 构建=成功
- 关键日志片段:
  - `Received message from server: TaskBuild { ... cl_link: "WOH2VX7P", changes: [Modified(ProjectRelativePath(".buckroot"))] }`
  - `[Task 019daeda-21ee-7f10-88a2-66756ec023ec] Target discovery succeeded: 1 targets`
  - `[Task 019daeda-21ee-7f10-88a2-66756ec023ec] Starting buck2 build`
  - `BUILD SUCCEEDED`
  - `[Task 019daeda-21ee-7f10-88a2-66756ec023ec] Buck2 build completed successfully`
- 结论: PASS

### Case: T01
- cl_link: `WOHOJHS8`
- 提交时间(UTC): `2026-04-21T07:09:41`
- 文件与改动摘要: 修改 `src/main.rs`，新增注释行（正确修改）
- 预期: 触发=是, 构建=成功
- 实际: 触发=是, 构建=成功
- 关键日志片段:
  - `Received message from server: TaskBuild { ... cl_link: "WOHOJHS8", changes: [Modified(ProjectRelativePath("src/main.rs"))] }`
  - `[Task 019daedf-9651-76b1-a22c-03c37fd6cf6e] Target discovery succeeded: 1 targets`
  - `[Task 019daedf-9651-76b1-a22c-03c37fd6cf6e] Starting buck2 build`
  - `[Task 019daedf-9651-76b1-a22c-03c37fd6cf6e] Buck2 build completed successfully`
  - `[Task 019daedf-9651-76b1-a22c-03c37fd6cf6e] Build succeeded; Exit code: Some(0)`
- 结论: PASS

### Case: T02（重启后第一次）
- cl_link: `0GRUQH94`
- 提交时间(UTC): `2026-04-21T07:16:05`
- 文件与改动摘要: 修改 `src/main.rs`，尝试追加 Rust 语法错误行
- 预期: 触发=是, 构建=失败
- 实际: 触发=是, 构建=成功
- 关键日志片段:
  - `Received message from server: TaskBuild { ... cl_link: "0GRUQH94", changes: [Modified(ProjectRelativePath("src/main.rs"))] }`
  - `[Task 019daee5-71ac-79f2-b5dd-4cb8f5776840] Target discovery succeeded: 1 targets`
  - `[Task 019daee5-71ac-79f2-b5dd-4cb8f5776840] Starting buck2 build`
  - `[Task 019daee5-71ac-79f2-b5dd-4cb8f5776840] Buck2 build completed successfully`
  - `[Task 019daee5-71ac-79f2-b5dd-4cb8f5776840] Build succeeded; Exit code: Some(0)`
- 结论: FAIL（预期失败但实际成功）

### Case: T02（重启后第二次，带 diff-preview 内容校验）
- cl_link: `XIPMHGLH`
- 提交时间(UTC): `2026-04-21T07:18:49`
- 文件与改动摘要: 修改 `src/main.rs`，追加 `// T02MARK_20260421T071848Z` 和语法错误函数头；提交前已确认 diff-preview 包含该标记与错误行
- 预期: 触发=是, 构建=失败
- 实际: 触发=是, 构建=成功
- 关键日志片段:
  - `Received message from server: TaskBuild { ... cl_link: "XIPMHGLH", changes: [Modified(ProjectRelativePath("src/main.rs"))] }`
  - `[Task 019daee7-f466-7fe0-a938-af51c1375b91] Target discovery succeeded: 1 targets`
  - `[Task 019daee7-f466-7fe0-a938-af51c1375b91] Starting buck2 build`
  - `[Task 019daee7-f466-7fe0-a938-af51c1375b91] Buck2 build completed successfully`
  - `[Task 019daee7-f466-7fe0-a938-af51c1375b91] Build succeeded; Exit code: Some(0)`
- 额外验证:
  - 任务保留挂载文件 `/home/jackie/orion-web/antares/mnt/019daee7-f466-7fe0-a938-af51c1375b91-1/project/buck2_test/src/main.rs` 中不存在 `T02MARK_20260421T071848Z` 和 `__restart_t02_broken_20260421T071848Z`
  - 说明 worker 实际看到的构建输入未体现本次 CL 提交内容
- 结论: FAIL（确认进入 bug 定位阶段，暂停后续矩阵用例）

补充：后续日志检索统一改为“先从 `cl_link` 找 `task_id`，再按 `task_id` 过滤”，避免 DEBUG FUSE 噪声导致等待时间过长。
