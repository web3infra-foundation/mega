# 修复非根 Cell 文件变更未触发增量编译的问题

## 问题描述

### 现象
- **CL HM7ZXVEV**: 修改 `project/buck2_test/toolchains/BUCK` 后，输出 "No impacted Buck targets detected"
- **CL M0YGTOEO**: 修改根目录的 `.mega_cedar.json` 后，编译一直在进行中（需要验证是否正确触发）

### 预期行为
根据 Buck2 和 Meta 的增量编译实现，以下文件变更应该触发增量编译：
- 工具链文件（toolchains cell 中的文件）
- 环境变量配置
- BUCK 文件
- Cargo.toml/Cargo.lock
- package.json/yarn.lock
- .mega_cedar.json 等配置文件

## 根本原因分析

### 问题定位过程

1. **路径解析验证** ✓
   - `CellInfo::unresolve()` 能正确将 `toolchains/BUCK` 解析为 `toolchains//BUCK`
   - 测试 `test_unresolve_cell_with_trailing_slash` 通过

2. **Package 检测验证** ✓
   - `Changes::contains_package()` 能正确识别 `toolchains//BUCK` 属于 `toolchains//` package
   - 测试 `test_contains_package_toolchains_cell` 通过

3. **Target 查询问题** ✗
   - **根本原因**: `targets_arguments()` 硬编码了 `//...` 模式
   - `//...` 只查询 **root cell** 的 targets
   - 其他 cell（toolchains, prelude 等）的 targets 完全被忽略

### 问题流程

```
修改 toolchains/BUCK
  ↓
Git 检测到变更: "toolchains/BUCK"
  ↓
CellInfo::unresolve() → "toolchains//BUCK" ✓
  ↓
Changes::contains_package(&toolchains//) → true ✓
  ↓
diff::immediate_target_changes() 检查 diff.targets()
  ↓
diff.targets() 中没有任何 toolchains cell 的 target ✗
  ↓
结果: "No impacted Buck targets detected"
```

### 为什么 diff.targets() 中没有 toolchains 的 target？

```rust
// orion/buck/run.rs (修复前)
pub fn targets_arguments() -> &'static [&'static str] {
    &[
        "targets",
        "//...",  // ← 只查询 root cell！
        "--target-platforms",
        "prelude//platforms:default",
        // ...
    ]
}
```

执行的命令：
```bash
buck2 targets //... --json-lines ...
```

这只会返回 root cell 的 targets，不包括：
- `toolchains//...` 的 targets
- `prelude//...` 的 targets
- 其他自定义 cell 的 targets

## 修复方案

### 1. 添加 `CellInfo::get_all_cell_patterns()` 方法

```rust
// orion/buck/cells.rs
impl CellInfo {
    /// Returns target patterns for all cells (e.g., ["root//...", "toolchains//...", ...])
    /// This is used to query targets from all cells, not just the root cell.
    pub fn get_all_cell_patterns(&self) -> Vec<String> {
        self.cells
            .keys()
            .map(|cell_name| format!("{}//...", cell_name.as_str()))
            .collect()
    }
}
```

### 2. 修改 `targets_arguments()` 移除硬编码的 `//...`

```rust
// orion/buck/run.rs
pub fn targets_arguments() -> &'static [&'static str] {
    &[
        "targets",
        // 移除 "//...",  ← 不再硬编码
        "--target-platforms",
        "prelude//platforms:default",
        // ...
    ]
}
```

### 3. 修改 `get_repo_targets()` 接受 CellInfo 参数

```rust
// orion/src/buck_controller.rs
fn get_repo_targets(
    file_name: &str,
    repo_path: &Path,
    cells: Option<&CellInfo>,  // ← 新增参数
) -> anyhow::Result<Targets> {
    // ...
    
    // If cells info is provided, query all cells; otherwise just query root cell
    if let Some(cells_info) = cells {
        let cell_patterns = cells_info.get_all_cell_patterns();
        tracing::debug!("Querying targets for cells: {:?}", cell_patterns);
        command.args(&cell_patterns);
    } else {
        // Default: only query root cell (for backward compatibility)
        command.arg("//...");
    }
    
    // ...
}
```

### 4. 更新调用点传递 CellInfo

```rust
// orion/src/buck_controller.rs
let base = get_repo_targets("base.jsonl", &old_repo, Some(&cells))?;
let diff = get_repo_targets("diff.jsonl", &mount_path, Some(&cells))?;
```

## 修复后的行为

### 执行的命令
```bash
buck2 targets root//... toolchains//... prelude//... --json-lines ...
```

### 效果
- 查询所有 cell 的 targets
- `diff.targets()` 包含 toolchains cell 的 targets
- 修改 `toolchains/BUCK` 能正确触发增量编译

## 测试验证

### 单元测试
```rust
#[test]
fn test_get_all_cell_patterns() {
    let cell_json = serde_json::json!({
        "root": "/path/to/repo",
        "toolchains": "/path/to/repo/toolchains",
        "prelude": "/path/to/repo/prelude"
    });
    let cells = CellInfo::parse(&serde_json::to_string(&cell_json).unwrap()).unwrap();
    
    let patterns = cells.get_all_cell_patterns();
    
    assert_eq!(patterns.len(), 3);
    assert!(patterns.contains(&"root//...".to_string()));
    assert!(patterns.contains(&"toolchains//...".to_string()));
    assert!(patterns.contains(&"prelude//...".to_string()));
}

#[test]
fn test_contains_package_toolchains_cell() {
    // 验证 toolchains/BUCK 能触发 toolchains// package 检测
    // ...
}
```

### 集成测试
需要在实际环境中验证：
1. 修改 `toolchains/BUCK` → 应该触发增量编译
2. 修改 `.mega_cedar.json` → 应该触发增量编译
3. 修改 `prelude` cell 中的文件 → 应该触发增量编译

## 关于挂载路径的说明

### Web 前端路由
- 项目浏览: `https://app.gitmega.com/mega/code/tree/main/project`
- 文件查看: `https://app.gitmega.com/mega/code/blob/main/.mega_cedar.json`

### 虚拟机挂载路径
根据 Orion 日志，实际的挂载路径格式为：
- 基础挂载点: `/tmp/mega-mount-{id}/` 或类似路径
- 项目路径: 挂载点下的相对路径

**重要**: 测试时应该参考实际日志中的挂载路径，而不是本地开发路径 `/Users/jackie/work/project/buck2_test`。

### 如何查看实际挂载路径
1. 查看 Orion 日志中的 `mount_point` 参数
2. 查看 `get_build_targets()` 函数的日志输出
3. 查看 `buck2 targets` 命令的执行目录

示例日志格式：
```
Get cells at "/tmp/mega-mount-abc123/project"
Analyzing changes [...]
```

## 相关 CL

- **CL HM7ZXVEV**: 修改 `project/buck2_test/toolchains/BUCK`（触发此次修复）
- **CL M0YGTOEO**: 修改 `.mega_cedar.json`（需要验证是否正确触发）

## 后续工作

1. ✅ 提交代码修复
2. ⏳ 在实际环境中验证 CL HM7ZXVEV 是否能正确触发
3. ⏳ 验证 CL M0YGTOEO 是否能正确触发
4. ⏳ 检查日志确认所有 cell 的 targets 都被正确查询
5. ⏳ 更新团队文档，说明挂载路径的分析方法

## 性能影响

### 查询时间
- 修复前: 只查询 root cell
- 修复后: 查询所有 cell（root + toolchains + prelude + ...）

### 预期影响
- 查询时间会略微增加（取决于 cell 数量和 target 数量）
- 但这是必要的，否则非 root cell 的变更无法被检测
- Buck2 本身支持并行查询多个 cell，性能影响应该可控

## 参考

- Buck2 文档: https://buck2.build/docs/concepts/cell/
- Meta 的 Target Determinator 实现
- Orion 项目的 `orion/buck/cells.rs` 和 `orion/src/repo/diff.rs`
