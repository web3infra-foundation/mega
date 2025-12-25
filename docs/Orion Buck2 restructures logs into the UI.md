# Orion Server Buck2 restructures logs into the UI
## 架构

- CL -> Task (一次提交对应一个任务记录)  
- Task 下包含多个 Build（对应单个 Buck2 target 或按变更自动解析的构建单元）。  
- 日志与状态均以 Build 为粒度隔离，Task 级别通过聚合得到总体状态。

## 后端更新

- 创建任务时 `builds` 数组可提供可选 `target`：

- 提供 `target`：worker 仅构建该目标，日志和状态仅归属该 Build。
- worker 根据变更自动解析目标列表，维持兼容模式。

- 日志 key 结构：`{task_id}/{repo_last_segment}/{build_id}.log`，Build 级别完全隔离。
- Build 状态推导：运行中 -> `Building`；未结束且不在 active -> `Pending`；结束且 `exit_code==0` -> `Completed`；`exit_code` 为其他值 -> `Failed`；`exit_code` 缺失 -> `Interrupted`；预留 `Canceled`。
- Task 聚合：

- 优先级：存在 `Failed/Interrupted/Canceled` -> `Failed`; 其次存在 `Building/Pending` -> `Building`; 否则全为成功 -> `Completed`; 无数据 -> `NotFound`。
- `partial_success=true` 当至少一个 Build 成功且仍有运行中或失败的 Build。

## HTTP 接口

- 创建任务 `POST /task`

- Request Body（摘录）：

- `repo` (string)
- `cl_link` (string)
- `cl` (number)
- `builds`: `BuildRequest[]`

- `buck_hash` / `buckconfig_hash`
- `args?`
- `target?` 可选 Buck2 label（如 `//app:server`）

- Response: `task_id` 与每个 build 的 `build_id`/状态（queued/dispatched/error）。

- 查询 CL 任务列表 `GET /tasks/{cl}`

- Response: `TaskInfoDTO[]`

- `status`: Task 聚合状态
- `partial_success`: 是否部分成功
- `build_list`: `BuildDTO[]`（含 `status`, `target`, `id`, `output_file` 等）

- 实时日志 `GET /task-output/{build_id}` (SSE)

- 事件类型 `log`，数据为该 Build 的行输出；按 Build 隔离。

- 历史日志 `GET /task-history-output`

- Query: `task_id`, `build_id`, `repo`, `start?`, `end?`
- 返回 `data: string[]` 与 `len`。

## WebSocket（Worker）

- 服务器下发 `Task` 消息新增 `target: Option<String>`。
- Worker 回传 `BuildOutput`、`BuildComplete`、`TaskPhaseUpdate` 不变。

## 前端

- 按 CL 查询后，使用 `TaskInfoDTO.status` 显示总体状态，若 `partial_success=true` 显示“部分成功/仍在构建”提示。
- Build 级列表用 `build_list` 渲染；点击某 Build 使用其 `id` 订阅 SSE 或读取历史日志，仅显示该 target 的日志。
- 高亮：`status` 为 `Failed/Interrupted/Canceled` 的 Build，日志中可配合后端已解析的 `cause_by` 字段做折叠/展开。
- 兼容：未提供 `target` 的旧请求依旧有效，UI 继续依赖 `target` 字段作为标签；字段新增均向后兼容。