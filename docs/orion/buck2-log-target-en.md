# Orion Server Buck2 Log Architecture

## Architecture

This design uses independent Buck2 builds for each target, generating dedicated log files to achieve natural multi-target log isolation. The scheduler aggregates states and results at the Task level for CL-level status summary. The solution directly reuses existing REST/WebSocket interfaces, enabling the frontend to access real-time target-level status and log information.

- **CL (Change List)** -> **Task** (1:N): One code change can submit multiple tasks (retry, add new targets, etc.)
- **Task** -> **Build** (1:N): One task submission can contain multiple Builds, each corresponding to a Buck2 target
- **Log Isolation**: Isolated at Build granularity, with Task-level aggregated overall status
- **Target Distinction**: Identified by Build's `target` field for specific build targets

```
CL (cl_id: 123)
│
├── Task 1 (task_id: 01JN1111-...) ← First submission
│   ├── Build A (build_id: 01JN2222-..., target: "//app:libA")
│   │   ├── Status: Completed, exit_code: 0
│   │   ├── Log: {task_id}/repo/{build_id}.log
│   │   └── API: /task-output/{build_id}, /task-history-output
│   ├── Build B (build_id: 01JN3333-..., target: "//app:libB")
│   │   ├── Status: Building, exit_code: null
│   │   ├── Log: {task_id}/repo/{build_id}.log
│   │   └── API: same as above
│   └── Build C (build_id: 01JN4444-..., target: "//app:libC")
│       ├── Status: Failed, exit_code: 1
│       ├── Log: {task_id}/repo/{build_id}.log
│       └── API: same as above
│
└── Task 2 (task_id: 01JN5555-...) ← Retry libC or add new target
    └── Build C' (build_id: 01JN6666-..., target: "//app:libC")
        ├── Status: Completed, exit_code: 0
        ├── Log: {task_id}/repo/{build_id}.log (new log file)
        └── API: same as above
```

Notes:

- CL/Task Aggregation: Any Build fails/interrupted/canceled → Task fails; any running → Building; all successful → Completed; partial_success is true when "some succeeded and some are running or failed"
- Fallback: When target is not provided, falls back to `//...`, does not interrupt the build, and logs a warning

## Backend Updates

- ~~Task creation `builds` array can provide optional `target`: With `target`: worker builds only that target, logs/status belong to that Build. Without: automatically parses target list (backward compatible), and logs fallback. (Already implemented in Orion module)~~
- Log key structure: `{task_id}/{repo_last_segment}/{build_id}.log`, Build-level complete isolation; `BuildDTO.log_path` returns this path for frontend display/download
- Build status derivation: running -> `Building`; not finished and not in active -> `Pending`; finished and `exit_code==0` -> `Completed`; `exit_code` other values -> `Failed`; `exit_code` missing -> `Interrupted`; `Canceled` reserved for support
- Task Aggregation:
- Priority: exists `Failed/Interrupted/Canceled` -> `Failed`; then exists `Building/Pending` -> `Building`; otherwise all successful -> `Completed`; no data -> `NotFound`
- `partial_success=true` when at least one Build succeeded and there are still running or failed Builds

## HTTP API

- Create Task `POST /task`
- Request Body (excerpt):
- `repo` (string)
- `cl_link` (string)
- `cl` (number)
- `builds`: `BuildRequest[]`
- `buck_hash` / `buckconfig_hash`
- `args?`
- `target?` optional Buck2 label (e.g., `//app:server`)
- Response: `task_id` and each build's `build_id`/status (queued/dispatched/error)
- Query CL Task List `GET /tasks/{cl}`
- Response: `TaskInfoDTO[]`
- `status`: Task aggregated status
- `partial_success`: whether partially successful
- `build_list`: `BuildDTO[]` (contains `status`, `target`, `id`, `output_file`, `log_path`, etc.)
- Real-time Log `GET /task-output/{build_id}` (SSE)
- Event type `log`, data is line output for that Build; isolated by Build
- Historical Log `GET /task-history-output`
- Query: `task_id`, `build_id`, `repo`, `start?`, `end?`
- Returns `data: string[]` and `len`

## WebSocket (Worker)

- Server sends `Task` message containing `target: Option<String>` (passed after backend parsing/fallback)
- Worker sends back `BuildOutput`, `BuildComplete`, `TaskPhaseUpdate` unchanged

## Frontend

- After querying by CL, use `TaskInfoDTO.status` to display overall status; if `partial_success=true`, display "Partially successful/still building" hint
- Build-level list rendered with `build_list`; clicking a Build uses its `id` to subscribe to SSE or read historical logs, displaying only that target's logs
- Highlight: Builds with `status` as `Failed/Interrupted/Canceled`; logs can collapse/expand with backend-parsed `cause_by` field
- Compatibility: Old requests without `target` still work; UI continues to rely on `target` field as label; new fields are all backward compatible

#### Details

- `/tasks/{cl}`: Returns aggregated status and target-split `build_list` (containing `target`, `status`, `log_path`)
- `/task-output/{build_id}` (SSE) and `/task-history-output`: Read by build_id/target, logs naturally isolated
- UI only needs to use `build_id`/`target` for filtering and highlighting failed entries; `partial_success` for "partially successful/still in progress" hint

## Testing
1) Create Task: `POST /task`, put multiple targets in `builds` array (example: `//app:libA`, `//app:libB`). Missing target will fall back to `//...` with warning but won't interrupt
2) View Aggregation: `GET /tasks/{cl}`, should see target-split `build_list` and aggregated `status`/`partial_success`
3) Log Isolation: Subscribe to `/task-output/{build_id_of_libA}` and `/task-output/{build_id_of_libB}` separately, should only see respective target logs; or use `/task-history-output` to read separately
4) Fallback Compatibility: Submit a build without target, confirm it can still return build_id and read logs

