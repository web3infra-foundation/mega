# Dependencies 页面

这个目录包含了 Rust crate 依赖关系的两个独立页面：

## 文件结构

- `index.tsx` - 表格视图页面，显示依赖关系的表格形式
- `graph/index.tsx` - 图形视图页面，显示依赖关系的图形可视化
- `README.md` - 本说明文件

## 功能特性

### 表格视图页面 (`index.tsx`)
- 显示依赖关系的表格形式
- 支持搜索功能
- 可展开查看详细信息（版本、发布日期、描述）
- 分页功能
- 点击 "Graph" 按钮导航到图形视图页面

### 图形视图页面 (`graph/index.tsx`)
- 使用 D3.js 渲染依赖关系图
- 交互式节点和连接
- 支持缩放和拖拽
- 根据 CVE 数量显示不同颜色
- **使用本地模拟数据展示**（包含20个依赖节点）
- 分页功能
- 点击 "Table" 按钮返回表格视图页面

## 使用方法

### 表格视图
1. 访问 `/dependencies` 页面，默认显示表格视图
2. 使用搜索框过滤依赖项
3. 点击展开按钮查看详细信息
4. 点击 "Graph" 按钮切换到图形视图

### 图形视图
1. 访问 `/dependencies/graph` 页面，显示图形视图
2. 拖拽节点移动位置
3. 使用鼠标滚轮缩放
4. 悬停查看节点信息
5. 点击 "Table" 按钮返回表格视图

## 技术栈

- React 18
- Next.js 14
- D3.js 7.9.0
- TypeScript
- Tailwind CSS

## 数据说明

### 模拟数据结构
图形视图目前使用本地模拟数据，包含以下依赖关系：

**主节点**: `tokio-1.35.1` (CVE: 0)
- `bytes-1.5.0` (CVE: 2)
  - `serde-1.0.195` (CVE: 1)
  - `log-0.4.20` (CVE: 0)
- `futures-0.3.29` (CVE: 0)
  - `pin-project-1.1.3` (CVE: 0)
  - `futures-core-0.3.29` (CVE: 0)
- `mio-0.8.8` (CVE: 5)
  - `libc-0.2.150` (CVE: 3)
  - `log-0.4.20` (CVE: 0)
- `num_cpus-1.16.0` (CVE: 0)
  - `hermit-abi-0.3.3` (CVE: 0)
- `parking_lot-0.12.1` (CVE: 8)
  - `lock_api-0.4.11` (CVE: 2)
  - `scopeguard-1.2.0` (CVE: 0)
- `signal-hook-registry-1.4.1` (CVE: 0)
  - `libc-0.2.150` (CVE: 3)
- `socket2-0.5.5` (CVE: 12)
  - `libc-0.2.150` (CVE: 3)
  - `winapi-0.3.9` (CVE: 0)
- `tracing-0.1.40` (CVE: 0)
  - `tracing-core-0.1.32` (CVE: 0)
  - `log-0.4.20` (CVE: 0)
- `windows-sys-0.48.0` (CVE: 0)
  - `windows-targets-0.48.5` (CVE: 0)
- `serde-1.0.195` (CVE: 1)
  - `serde_derive-1.0.195` (CVE: 0)

### 颜色编码
- **青色** (rgb(50,224,196)): 初始源节点 (tokio-1.35.1)
- **绿色** (rgb(46,204,113)): CVE 数量 = 0
- **红色** (rgb(229,72,77)): CVE 数量 > 0

## 组件依赖

- `DependencyGraph` - 位于 `../../../../../component/DependencyGraph.tsx`
- `CrateInfoLayout` - 位于 `../../layout.tsx`

## 实现细节

### 页面导航
```typescript
// 从表格页面导航到图形页面
const handleNavigateToGraph = () => {
    router.push(`/${nsfront}/rust/rust-ecosystem/crate-info/${crateName}/dependencies/graph`);
};

// 从图形页面返回表格页面
const handleBackToTable = () => {
    router.push(`/${nsfront}/rust/rust-ecosystem/crate-info/${crateName}/dependencies`);
};
```

### 保持的UI元素
- 搜索栏（在两个页面都可见）
- Table/Graph 切换按钮（始终可见）
- 分页组件（在两个页面都显示）
- 页面布局和样式

### 页面特定功能
- **表格页面**: 表格内容、展开/收起功能、搜索过滤
- **图形页面**: D3.js 图形渲染、交互功能

## 未来计划

当后端 API 可用时，可以替换 `DependencyGraph` 组件中的 `generateMockDependencyData()` 函数，恢复使用真实的 API 数据。
