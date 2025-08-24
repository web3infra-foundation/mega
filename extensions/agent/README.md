# agent

本目录用于存放智能代码审查（code review suggest）、AI agent 等相关服务和逻辑。

## 为什么放在 extensions/ 下？
- extensions 目录专门用于扩展主业务的智能、AI、外部服务能力。
- agent 作为 AI 智能能力的统一入口，便于与主业务（如 mono、jupiter 等）解耦，方便独立开发、部署和横向扩展。
- 未来可复用于多种场景，如代码审查、自动补全、文档生成等。

## 建议结构
- src/         # 主要代码实现
- api/         # HTTP/gRPC 接口
- model/       # 数据结构与类型
- service/     # 业务逻辑

主业务可通过 API 调用本目录下的 agent 服务，实现智能代码审查等功能。
