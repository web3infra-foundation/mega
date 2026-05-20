# Agent.md - Mega Monorepo 的 AI 落地方案

> 本文档基于 Mega 当前仓库状态、Claude Code 官方能力边界，以及 Anthropic 2026-05-14 发布的《How Claude Code works in large codebases》整理。目标不是一次性建设一套“大而全”的 AI 平台，而是把 Mega 现有工程规范逐步转成可被 AI Agent 稳定消费、执行和审计的仓库资产。

---

## 0. 结论

**总体可落地，但必须拆成两条节奏不同的路线。**

| 路线 | 可落地性 | 原因 | 首个可验收结果 |
|---|---:|---|---|
| Agent harness：`CLAUDE.md` / `AGENTS.md` / `.claude/settings.json` / skills / hooks / subagents | 高 | Claude Code 已原生支持项目级 memory、settings、skills、hooks、subagents；Mega 已有 README、development、contributing 等规范来源 | 从 `ceres/`、`jupiter/`、`moon/apps/web/` 启动 Agent 时能自动说出本目录约束和正确检查命令 |
| `mega-mcp`：把 Mega 自身能力暴露为 MCP tools | 中 | Claude Code 支持项目级 `.mcp.json`，Mega 已有 tree、commit、policy、LFS、Buck upload 等 API/模块；但当前没有跨语言符号索引或稳定 Buck target 覆盖 | 先实现 file tree、commit metadata、policy check、workspace map 等已有能力的 MCP 包装 |
| 跨语言 symbol/cross-ref 搜索、5x 快于 `rg`、完整 Code Attribution | 低到中 | 这些依赖额外索引、LSP/Buck2 集成和归因数据模型，不应放在第一阶段承诺 | 作为 Phase 4 研究项，先写基准和数据模型再实现 |

**关键调整**：当前方案可以做，但不能把所有内容都放进 Phase 1。Phase 1 应只交付低风险、可版本化、可手工回滚的上下文和护栏；MCP 和归因能力应独立成产品化 track。

---

## 1. 当前证据与硬约束

### 1.1 仓库事实

- 当前这个本地工作副本不是标准 Git checkout，而是 Libra 工作区；`git status` 会失败，`libra status --short` 才能看到变更状态。公开贡献流程仍需兼容 `docs/contributing.md` 中的 Git/DCO/PGP 要求。
- 当前 `.gitignore` 第 63-64 行整体忽略 `.claude`，但方案要求 `.claude/settings.json`、`.claude/agents/**`、`.claude/skills/**` 被团队共享。这是 Phase 1 的第一个阻塞项。
- 当前存在 `.claude/settings.local.json`，只放了个人权限覆盖；它应继续作为本地文件，不进入共享配置。
- Rust workspace 当前有 12 个成员，而不是 11 个：`api-model`、`ceres`、`common`、`context`、`io-orbit`、`jupiter`、`jupiter/callisto`、`mono`、`orion`、`orion-server`、`saturn`、`vault`。
- README 已声明 PR 前置检查：`cargo clippy --all-targets --all-features -- -D warnings`、`cargo +nightly fmt --all --check`、`cargo buckal build`，并要求依赖变更后运行 `cargo buckal migrate`。
- `docs/contributing.md` 要求提交包含 `Signed-off-by`，并说明 PGP 签名要求；AI 提交规范不能只写 Conventional Commits 和 `Co-Authored-By`。
- `jupiter/README.md` 已定义 SeaORM migration 和 entity 生成流程，entity 输出目录是 `jupiter/callisto/src`。
- `saturn/` 已有 Cedar schema、policy 和解析/授权代码，是策略审查 agent 的合理边界。
- `mono/src/api/api_router.rs` 已聚合 file tree、commit、Buck、artifacts、permission、reviewer 等 API；MCP MVP 应优先包装这些已有能力，而不是先承诺新的代码索引能力。

### 1.2 Claude Code 能力边界

- Claude Code 官方建议通过 `CLAUDE.md`、settings、skills、MCP servers 组成 harness；大仓库并不要求先建 embedding/RAG 索引。
- 项目级 subagents 放在 `.claude/agents/`，项目级 skills 放在 `.claude/skills/`，都可以随仓库共享。
- 项目级 MCP 配置应使用仓库根目录 `.mcp.json`，不是 `.claude/mcp/*.json`。
- Hooks 可以阻止工具调用或让 Stop 事件继续工作，但 command hooks 以当前用户权限运行，必须非常保守；不要在 Stop hook 里默认跑长时间全量构建。
- Read-only subagent 如果只授予 Read/Grep/Glob/Bash，就不能同时要求它写 `.agents-scratch/<task>.md`。要么返回结构化摘要，要么明确给它受保护的写权限并用 hook 限制路径。

---

## 2. 设计原则

1. **先上下文，后自动化**  
   先让 Agent 读到正确入口、模块边界和检查命令，再逐步增加 hook、skill、MCP。不要一开始就把重型质量门禁塞进 Stop hook。

2. **共享配置必须能进版本控制**  
   `.claude/settings.json`、agents、skills、commands、hooks 如果是团队资产，就必须解除 `.gitignore` 中对 `.claude` 的整体忽略；个人 override 继续放 `.claude/settings.local.json`。

3. **`permissions.deny` 只用于安全边界，不用于普通噪声控制**  
   不要 deny `Cargo.lock`、`pnpm-lock.yaml`、`tests/**` 这类开发中经常需要读取的文件。噪声控制靠 `CLAUDE.md` 指引、文件建议、命令作用域和 agent 习惯，安全控制才用 deny。

4. **重命令用显式 skill/command，轻检查用 hook**  
   `fmt --check`、路径保护、配置变更审计适合 hook；`clippy --all-targets`、`cargo buckal build`、`pnpm -C moon lint` 应放到 `/pre-pr-check` 或 skill，由用户或 Agent 在收尾阶段显式运行。

5. **MCP MVP 只包装已有事实，不承诺不存在的索引**  
   第一版 `mega-mcp` 包装 tree、commit、LFS、policy、workspace map、Buck upload/session 等已有接口。`search_symbol`、`cross_ref`、“比 `rg` 快 5x”必须等索引方案和基准存在后再进入验收。

6. **Agent 输出必须可审计**  
   AI 提交至少保留 Conventional Commit、`Signed-off-by`、PGP 签名要求、`Co-Authored-By`；后续再把这些 trailer 与 Mega 的 commit binding / attribution 模型打通。

---

## 3. 改进后的目标架构

```text
mega/
├── CLAUDE.md                         # Claude Code 根 memory：指针 + 跨模块硬规则
├── AGENTS.md                         # Codex/Cursor 等通用 Agent 入口，内容与 CLAUDE.md 同步但不必逐字相同
├── .mcp.json                         # 项目级 MCP 配置，Claude Code 官方共享位置
├── .claude/
│   ├── settings.json                 # 团队共享设置：权限、env、轻量 hooks
│   ├── settings.local.json           # 个人覆盖，必须继续忽略
│   ├── agents/
│   │   ├── rust-explorer.md
│   │   ├── frontend-explorer.md
│   │   ├── schema-policy-reviewer.md
│   │   └── monorepo-impact-analyzer.md
│   ├── skills/
│   │   ├── pre-pr-check/
│   │   ├── rust-workspace-change/
│   │   ├── sea-orm-migration/
│   │   ├── cedar-policy-edit/
│   │   └── conventional-commit/
│   ├── commands/
│   │   └── impact.md                 # 保留少量手动命令；复杂流程优先做 skill
│   └── hooks/
│       ├── deny-protected-paths.sh
│       ├── post-edit-rustfmt.sh
│       └── stop-preflight.sh
├── .agents-scratch/                  # 可选，临时报告目录，必须忽略
├── api-model/CLAUDE.md
├── ceres/CLAUDE.md
├── common/CLAUDE.md
├── context/CLAUDE.md
├── io-orbit/CLAUDE.md
├── jupiter/CLAUDE.md
├── jupiter/callisto/CLAUDE.md
├── mono/CLAUDE.md
├── moon/CLAUDE.md
├── moon/apps/web/CLAUDE.md
├── moon/apps/sync-server/CLAUDE.md
├── orion/CLAUDE.md
├── orion-server/CLAUDE.md
├── saturn/CLAUDE.md
├── vault/CLAUDE.md
└── docs/agent.md
```

### 3.1 `.gitignore` 必须同步调整

把当前整体忽略 `.claude` 改成“共享配置可提交、个人/临时内容忽略”：

```gitignore
# Claude Code shared project assets
!.claude/
!.claude/settings.json
!.claude/agents/
!.claude/agents/**
!.claude/skills/
!.claude/skills/**
!.claude/commands/
!.claude/commands/**
!.claude/hooks/
!.claude/hooks/**

# Claude Code local/private state
.claude/settings.local.json
.claude/**/local/**
.agents-scratch/
```

如果保留 `.claude` 的全局忽略规则，必须把上面的 unignore 规则放在它之后，否则共享配置不会进入版本控制。

---

## 4. 关键交付物

### 4.1 根 `CLAUDE.md` 与 `AGENTS.md`

根文件只放跨仓库规则，不复制长文档。目标 `< 180` 行。

必须包含：

| 节 | 内容 |
|---|---|
| `Repository Map` | 12 个 Rust workspace 成员、`moon` 前端 workspace、`docker`、`config`、`scripts`、`docs` 的一行说明 |
| `Start Here` | 先读根文件，再读当前目录最近的 `CLAUDE.md`；如果没有本地文件，回到对应 README |
| `Required Checks` | Rust：`cargo +nightly fmt --all --check`、`cargo clippy --all-targets --all-features -- -D warnings`；Buck：`cargo buckal build`；前端：`pnpm -C moon lint` |
| `Dependency Changes` | 改 `Cargo.toml` 后运行 `cargo buckal migrate` 并检查生成文件；改 `moon/package.json` 或 package 依赖后使用 `pnpm` |
| `Gotchas` | 根 `tests/` 是测试数据；Postgres 不用 Array 类型；`mono` 默认 Postgres、`mega` 默认 SQLite；Rust import 禁止 `super::` / `self::` |
| `Commit Requirements` | Conventional Commits、`Signed-off-by`、PGP 签名要求、AI 参与时加 `Co-Authored-By` |
| `When To Use Subagents` | 大量探索、影响面分析、schema/policy 复核；小改动留在主会话 |
| `MCP` | 当前阶段只说明 `.mcp.json` 和可用工具，不声称 symbol index 已存在 |

`AGENTS.md` 应保持工具中立，避免只写 Claude Code 私有概念；可以指向 `.claude/**` 作为 Claude Code 专用实现。

### 4.2 子目录 `CLAUDE.md`

所有模块使用同一骨架，避免上下文质量漂移：

```markdown
# <module>

## Purpose
1-3 sentences.

## Key Files
- `src/...` - ...

## Local Rules
Only rules that differ from root.

## Local Commands
- Check: `...`
- Test: `...`

## High-Risk Changes
- When changing X, also inspect Y.
```

首批必须覆盖：

| 模块 | 额外要求 |
|---|---|
| `jupiter/` | SeaORM migration 固定流程；schema 变更必须检查 migration、storage、entity 三面 |
| `jupiter/callisto/` | 说明 entity 多数来自生成流程，手改前必须确认来源 |
| `saturn/` | Cedar schema/policy 变更必须运行策略测试并由 `schema-policy-reviewer` 复核 |
| `moon/` | 固定 `pnpm`，共享依赖走 `pnpm-workspace.yaml` `catalog:`，禁止 `npm install` |
| `mono/` | API/router/storage 的改动要同步检查 OpenAPI 注解、权限 guard、对应 storage/service |
| `orion/`、`orion-server/` | Buck/runner/build API 相关变更要跑对应 crate 测试和影响分析 |

### 4.3 `.claude/settings.json`

第一版 settings 只做低风险默认值：

```json
{
  "permissions": {
    "deny": [
      "Read(./.env)",
      "Read(./.env.*)",
      "Read(./**/.env)",
      "Read(./**/.env.*)",
      "Read(./target/**)",
      "Read(./**/target/**)",
      "Read(./moon/**/node_modules/**)",
      "Read(./moon/**/.next/**)",
      "Read(./buck-out/**)",
      "Read(./.git/objects/**)",
      "Read(./.libra/objects/**)"
    ],
    "ask": [
      "Bash(git push*)",
      "Bash(libra push*)",
      "Bash(git reset --hard*)",
      "Bash(rm -rf*)",
      "Bash(cargo install*)"
    ]
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PROJECT_DIR}/.claude/hooks/deny-protected-paths.sh"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PROJECT_DIR}/.claude/hooks/post-edit-rustfmt.sh"
          }
        ]
      }
    ]
  },
  "env": {
    "CARGO_TERM_COLOR": "never",
    "MEGA_LOG__LEVEL": "warn"
  }
}
```

不要在第一版里 deny：

- `Cargo.lock` / `pnpm-lock.yaml`：依赖变更必须读取。
- 根 `tests/**`：虽然默认不是 integration tests，但里面可能是 fixtures。
- `docs/**`：Agent 需要读文档作为上下文来源。

### 4.4 Hooks

| Hook | 阶段 | 必须满足的实现方式 |
|---|---|---|
| `deny-protected-paths.sh` | Phase 1 | 只检查路径，不运行格式化/测试；拒绝写入 `target/`、`node_modules/`、`.next/`、`.git/objects/`、`.libra/objects/`、`jupiter/callisto/src` 的未授权生成文件 |
| `post-edit-rustfmt.sh` | Phase 1 | 只对本次编辑的 `.rs` 文件运行快速格式检查；失败时返回可读错误，不自动改文件 |
| `stop-preflight.sh` | Phase 2 | 只在 Agent 最终消息声称“done/完成/ready”时检查是否运行过必要命令；不要默认每次 Stop 都跑 full clippy/build |

重型检查进入 `pre-pr-check` skill：

```bash
cargo +nightly fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo buckal build
pnpm -C moon lint
```

### 4.5 Subagents

先做 4 个，等有真实重复需求再扩展到 6 个以上。

| Subagent | 工具权限 | 触发条件 | 输出 |
|---|---|---|---|
| `rust-explorer` | Read/Grep/Glob/Bash，只读 | Rust 符号定位、调用链、crate 影响面 | 结构化摘要，必须带文件路径和行号 |
| `frontend-explorer` | Read/Grep/Glob/Bash，只读 | `moon` 中组件、hook、package 依赖定位 | 结构化摘要，必须带文件路径和行号 |
| `schema-policy-reviewer` | Read/Grep/Glob/Bash，只读 | `jupiter` migration/entity 或 `saturn` Cedar 变更 | 风险清单：兼容性、索引、权限、测试缺口 |
| `monorepo-impact-analyzer` | Read/Grep/Glob/Bash，只读 | 大改动前后评估 Rust/Buck/frontend 影响面 | 受影响 crate/app/target/test 列表 |

注意：只读 subagent 不写 `.agents-scratch`。如果确实需要临时报告文件，单独建立 `report-writer` skill 或给特定 agent 加写权限，并用 hook 限制只能写 `.agents-scratch/**`。

### 4.6 Skills 与 Commands

优先用 skills，因为它们支持支持文件、脚本、示例和自动发现；commands 只保留交互入口。

| Skill | 作用域 | 行为 |
|---|---|---|
| `pre-pr-check` | 根 | 按顺序运行 fmt、clippy、buckal build、pnpm lint；失败即停止并总结失败项 |
| `rust-workspace-change` | 根 / Rust crate | 改 workspace、依赖或 crate 边界时提示 `cargo buckal migrate` 和影响面检查 |
| `sea-orm-migration` | `jupiter/**` | 引导 migration、entity 生成、storage/test 检查 |
| `cedar-policy-edit` | `saturn/**` | 引导 schema/policy/test/reviewer 复核 |
| `conventional-commit` | 根 | 生成 Conventional Commit + `Signed-off-by` + AI trailer 检查清单 |

Commands：

| Command | 行为 |
|---|---|
| `/impact <path>` | 调度 `monorepo-impact-analyzer` |
| `/pre-pr-check` | 可以保留为 command，但内部应只调用 `pre-pr-check` skill |

---

## 5. `mega-mcp` 改进方案

### 5.1 MVP 边界

第一版不要叫 “symbol search server”。它应是 **Mega context server**，包装当前已经存在或容易稳定实现的能力。

| MCP Tool | 来源 | Phase | 说明 |
|---|---|---:|---|
| `mega.workspace_map` | `Cargo.toml`、`moon/pnpm-workspace.yaml`、`BUCK` | 3A | 返回 workspace members、frontend packages、Buck 文件位置 |
| `mega.file_tree` | `mono` tree API 或本地文件系统 | 3A | 返回指定 path/depth 的轻量树 |
| `mega.commit_history` | `mono` commit router | 3A | 包装 commit history + path filter |
| `mega.commit_files_changed` | `mono` commit router | 3A | 返回某 commit 的 changed files |
| `mega.commit_binding` | `mono` commit binding API | 3A | 连接 attribution 的最小现有入口 |
| `mega.policy_check` | `saturn` crate 或 `mono` permission API | 3B | 对 principal/action/resource 做策略评估 |
| `mega.lfs_info` | LFS API/storage | 3B | 返回 LFS metadata，不下载大对象 |
| `mega.buck_targets_for` | `buck2 uquery` 本地命令 | 3B | 先做 CLI wrapper；等 Buck graph 存储稳定后再走服务端 |

### 5.2 延后项

这些不进入 MVP 验收：

- `mega.search_symbol`
- `mega.cross_ref`
- 跨语言同名符号消歧
- “比 `rg` 快 5x”
- 需要服务端持久索引的任何工具

进入 Phase 4 前必须先补一份设计文档，明确：

- 索引来源：LSP、tree-sitter、Buck graph、ctags，或 Mega 自身对象库。
- 增量更新策略：push/merge 时更新，还是本地临时构建。
- stale index 处理：结果必须暴露索引 revision / commit sha。
- 基准：数据集、冷启动/热启动、准确率、与 `rg` 的对照。

### 5.3 配置位置

项目级 MCP 配置应放根目录 `.mcp.json`：

```json
{
  "mcpServers": {
    "mega": {
      "command": "cargo",
      "args": ["run", "-p", "mega-mcp", "--"],
      "env": {
        "MEGA_MCP_MODE": "local"
      }
    }
  }
}
```

如果 `mega-mcp` 先作为 `mono --feature mcp` 实现，配置里的 command/args 再对应调整。

---

## 6. 分阶段落地路线图

### Phase 0 - 方案硬化与版本控制边界（0.5-1 天）

交付物：

- [ ] 更新本文件，明确可落地边界。
- [ ] 修改 `.gitignore` / `.libraignore`，允许共享 `.claude/**` 资产进入版本控制，同时继续忽略 `.claude/settings.local.json` 和 `.agents-scratch/`。
- [ ] 确认 `libra status --short` 能看见将要提交的 agent 资产。

验收：

- `jq . .claude/settings.json` 能通过。
- `libra status --short` 不出现 `.claude/settings.local.json`。

### Phase 1 - Context Harness（1 周）

交付物：

- [ ] 根 `CLAUDE.md` 与 `AGENTS.md`。
- [ ] 12 个 Rust workspace 成员的 `CLAUDE.md`，加 `moon/`、`moon/apps/web/`、`moon/apps/sync-server/`。
- [ ] `.claude/settings.json` 第一版。
- [ ] `deny-protected-paths.sh`、`post-edit-rustfmt.sh`。

验收：

- 从 `ceres/`、`jupiter/`、`saturn/`、`moon/apps/web/` 启动 Claude Code，询问“本目录完成任务前必须跑什么检查”，回答必须命中本地规则。
- 尝试编辑 `target/` 或 `moon/**/node_modules/**` 被 hook 阻止。
- 尝试读取 `Cargo.lock`、`docs/development.md`、根 `tests/` 不被 settings deny。
- `post-edit-rustfmt.sh` 对 `.rs` 文件返回可读结果，且不会自动修改源文件。

### Phase 2 - Reusable Workflows（2 周）

交付物：

- [ ] 4 个 subagent 定义。
- [ ] 5 个 skills。
- [ ] `/impact` 和 `/pre-pr-check` command。
- [ ] `stop-preflight.sh`，只做“声称完成前是否运行必要检查”的轻量 gate。

验收：

- `/impact jupiter/src/storage` 能输出受影响 crate、storage/service、migration/entity 检查点。
- 修改 `saturn/mega_policies.cedar` 后，`schema-policy-reviewer` 能给出带文件行号的风险摘要。
- `/pre-pr-check` 任一命令失败时，Agent 不继续声明完成，而是报告失败命令和下一步。

### Phase 3A - `mega-mcp` Local MVP（2-3 周）

交付物：

- [ ] 新 crate `mega-mcp`，或 `mono --feature mcp`。
- [ ] 根 `.mcp.json`。
- [ ] `mega.workspace_map`、`mega.file_tree`、`mega.commit_history`、`mega.commit_files_changed`。
- [ ] `docs/mega-mcp.md`。

验收：

- Claude Code `/mcp` 能看到项目级 `mega` server。
- 每个 tool 返回结构化 JSON，并带 `source` 字段说明来自本地文件、mono API 或命令。
- 大输出分页或限量，避免一次返回超过上下文可消费范围。

### Phase 3B - Policy / LFS / Buck Tools（2-4 周）

交付物：

- [ ] `mega.policy_check`
- [ ] `mega.lfs_info`
- [ ] `mega.buck_targets_for`

验收：

- policy tool 对至少 5 个 saturn 测试场景返回与 crate 测试一致的结论。
- lfs tool 只返回 metadata，不下载大对象。
- buck tool 对已有 `jupiter/callisto/BUCK` 能返回相关 target；如果根 `BUCK` 为空，必须显式说明 coverage gap。

### Phase 4 - Advanced Agent-Native Features（持续）

候选项：

- 代码符号索引与 `mega.search_symbol`。
- `mega.cross_ref`。
- IntentSpec 与 `.claude/skills/**` 的映射。
- Code Attribution 数据模型：commit trailer、commit binding、line-level attribution 的关系。
- Agent DRI 与季度复核机制。

验收前置：

- 每项必须先有设计文档、基准、stale-data 策略和回滚路径。

---

## 7. 风险与缓解

| 风险 | 具体表现 | 缓解 |
|---|---|---|
| `.claude` 资产未进入版本控制 | 本地能用，团队不可复现 | Phase 0 先改 ignore 规则，并用 `libra status --short` 验证 |
| settings 误 deny 正常开发文件 | Agent 不能读 lockfile、fixture、docs | deny 只覆盖 secrets/build artifacts/object store |
| Stop hook 太重 | 每次对话结束都跑 full clippy/build，体验崩坏 | Stop 只做声明完成前的轻量 preflight；全量检查放 skill |
| subagent 过度拆分 | 小任务延迟变高，主上下文反复等待 | `CLAUDE.md` 明确：小改动主会话完成，3+ 步探索或高输出任务才调度 |
| MCP 过早承诺索引能力 | 工具结果慢、不准或 stale | MVP 只包装已有 API；索引能力必须单独设计和基准 |
| AI commit 不满足项目贡献要求 | 只有 Conventional Commit，没有 DCO/PGP | commit skill 同时检查 `Signed-off-by`、PGP、`Co-Authored-By` |
| `jupiter/callisto` 手改污染生成实体 | entity 与 migration/schema 漂移 | path hook + `jupiter/callisto/CLAUDE.md` 明确生成流程和例外审批 |

---

## 8. Day 1 可执行清单

1. 修改 ignore 规则，让共享 `.claude/**` 可提交，继续忽略 `.claude/settings.local.json`。
2. 创建根 `CLAUDE.md` 与 `AGENTS.md`，只写入口、命令、坑位、提交要求。
3. 创建 `jupiter/CLAUDE.md`、`saturn/CLAUDE.md`、`moon/CLAUDE.md` 三个高价值样板。
4. 创建 `.claude/settings.json`，只 deny secrets/build artifacts/object stores。
5. 创建 `deny-protected-paths.sh` 和 `post-edit-rustfmt.sh`，先支持 dry-run。
6. 运行：

```bash
jq . .claude/settings.json
libra status --short
```

7. 在 `ceres/`、`jupiter/`、`moon/apps/web/` 启动一次 Agent 并询问本目录规则，记录差距后再补子目录 `CLAUDE.md`。

---

## 9. 参考

- Anthropic, How Claude Code works in large codebases: Best practices and where to start, 2026-05-14: https://claude.com/blog/how-claude-code-works-in-large-codebases-best-practices-and-where-to-start
- Claude Code Settings: https://code.claude.com/docs/en/settings
- Claude Code Hooks: https://code.claude.com/docs/en/hooks
- Claude Code Subagents: https://code.claude.com/docs/en/sub-agents
- Claude Code Skills: https://code.claude.com/docs/en/slash-commands
- Claude Code MCP: https://code.claude.com/docs/en/mcp
- Mega `README.md`
- Mega `docs/development.md`
- Mega `docs/contributing.md`
- Mega `docs/api.md`
