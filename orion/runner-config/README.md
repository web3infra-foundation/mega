# Orion Runner 生产环境配置

此目录包含生产环境下 Orion Worker 的运行配置。

**详细部署文档**: [docs/deployment.md](../docs/deployment.md)

## 文件说明

| 文件 | 说明 |
|------|------|
| `.env.prod` | 生产环境变量（部署时重命名为 `.env`）|
| `scorpio.toml` | scorpiofs 配置（存储路径、Antares 等）|
| `run.sh` | 启动脚本（systemd ExecStart）|

## CI 部署流程

1. 触发：推送到 `main` 分支（`orion/**` 路径）
2. 编译 orion 二进制
3. 打包 orion + runner-config/* 为 artifacts
4. rsync 部署到目标 VM
5. 重启 `orion-runner.service`

## 目标机器路径

```
/data/scorpio/store              # 数据存储
/data/scorpio/antares/           # overlay 目录
/workspace/mount                 # FUSE 挂载点
/{root,home/orion}/orion-runner/ # 运行目录
  ├── orion             # 二进制
  ├── run.sh            # 启动脚本
  ├── scorpio.toml      # 配置
  └── .env              # 环境变量
```
