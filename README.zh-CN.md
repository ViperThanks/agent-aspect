# Agent Aspect

[English](README.md) | [简体中文](README.zh-CN.md)

> 给 AI coding agent 准备 before、around、after hooks。

Agent Aspect 是一个本地运行时控制平面，面向 Claude Code、Codex CLI、Kimi Code 等 AI 编程 agent。它站在 agent 和真实动作之间，观察工具调用，执行本地策略，记录审计日志，并提供审批、会话、任务和学习规则的控制入口。

## 它做什么

- **策略拦截：** 通过 agent hook 系统拦截高风险工具调用，并返回 allow / ask / deny。
- **统一审计：** 用 SQLite 记录事件、决策、会话、任务、反馈和设备归因。
- **Bridge 控制台：** 在本机提供 token 保护的 HTTP UI，覆盖首页、会话、审计和执行工作流。
- **远程任务：** 提交白名单本地任务或 provider prompt，支持取消、日志流和 stale job 恢复。
- **会话视图：** 导入 provider 标题和 transcript，按 provider / project 组织聊天与工具活动。
- **运行时护栏：** 继续会话前绑定 provider/runtime identity，提示 model、permission、path 或 binary drift。
- **Learn Mode：** 从真实 ask→allow 模式中建议未来 auto-allow 规则，但显式 deny 永远优先。
- **多设备归因：** 记录是哪个浏览器或本地 hook 做出了决策。
- **Relay 手机控制（可选）：** 通过自托管 relay，让手机在不同网络下继续审批、查看会话和提交任务。

## 架构

```text
AI Agents
  ├─ Claude Code hooks
  ├─ Codex CLI hooks / transcripts
  └─ Kimi Code hooks
        │
        ▼
checkpoint-hook ── Unix socket ── checkpointd
        │                           │
        │                           ├─ Rule engine
        │                           ├─ Learned-rule fallback
        │                           └─ SQLite audit.db
        │
        ▼
checkpoint-bridge ── HTTP + token ── Web / mobile browser
        │
        └──── WSS ── checkpoint-relay ── Phone browser  (optional)
```

核心产品原则：

> 机制表达要穷尽，策略暴露要渐进，信任链路要可追踪。

## 推荐部署

| 路径 | 适用场景 |
|------|----------|
| **本机模式** | 默认。daemon + bridge 跑在 Mac 上，手机可通过 LAN 访问。 |
| **LAN / Tailscale** | 手机和 Mac 在同一网络或 Tailscale mesh 中，不需要 relay。 |
| **自托管 relay** | 手机在移动网络、Mac 在另一个网络。你在自己的 VPS 上运行 `checkpoint-relay`。 |

Relay 是可选的远程手机通道。它不是默认依赖，不是云同步，不是账号系统，也不是多租户 SaaS。详见 [docs/relay.md](docs/relay.md)。

## 快速开始

### 前置条件

- macOS（Apple Silicon）
- Rust 工具链（`rustup`）
- 已安装 Claude Code、Codex CLI 或 Kimi Code

### 构建

```bash
cargo build --release
```

### 安装并运行

```bash
# 安装 CLI、daemon、hook 和 bridge
cargo install --path crates/cli
cargo install --path crates/daemon
cargo install --path crates/hook-cli
cargo install --path crates/bridge

# 初始化配置
checkpoint init

# 检查环境
checkpoint doctor

# 启动 daemon（后台 Unix socket 服务）
checkpoint daemon start

# 启动 bridge（token 保护的 HTTP UI）
checkpoint bridge start

# 获取浏览器访问 token
checkpoint bridge token

# 打开 bridge UI
open http://127.0.0.1:7676
```

### 设置执行模式

```bash
# Observer：只记录，不阻断
checkpoint mode observer

# Autonomous：安全动作自动允许，高风险动作询问
checkpoint mode autonomous

# Guard：大多数写操作前询问
checkpoint mode guard

# Paranoid：所有操作前询问
checkpoint mode paranoid
```

### 构建并运行 relay（可选）

```bash
# 构建 relay binary
cargo install --path crates/relay

# 在 VPS 上运行，或本机测试
checkpoint-relay
```

配对和部署见 [docs/relay.md](docs/relay.md)。

## 开发

```bash
cargo fmt --check
cargo test
scripts/smoke_test.sh
scripts/bridge_smoke_test.sh
scripts/relay_smoke_test.sh
```

常用本地命令：

```bash
checkpoint doctor
checkpoint mode guard
checkpoint bridge start
checkpoint bridge status
checkpoint bridge token
```

## 目录结构

```text
crates/
  core/       共享类型、SQLite 审计存储、规则引擎、normalization、transcripts
  daemon/     评估 hook 请求的 Unix socket daemon
  hook-cli/   Agent hook 入口
  cli/        checkpoint 命令行管理工具
  bridge/     token 保护的 HTTP bridge 和内嵌 Web UI
  relay/      用于手机访问 Mac bridge 的自托管 relay
  shared_ui/  bridge 和 relay 共用的前端基础模块

docs/         公开配置、relay、安全和开源范围文档
scripts/      core / bridge / relay 冒烟测试
```

## 支持的 Agent

| Agent | 状态 |
|-------|------|
| Claude Code | Supported |
| Codex CLI | Supported |
| Kimi Code | Supported |
| Gemini CLI | Candidate（已有骨架，但不宣称 runtime support） |

## 安全模型

- **信任锚点在你的 Mac。** daemon、规则引擎和审计库都在本地运行。
- Bridge token 本地生成，并存储在 `~/.checkpoint/bridge.token`。
- 除 `GET /health` 外，所有 bridge API 都需要 Bearer token。
- Bridge 默认不启用 CORS。
- Job 只允许白名单类型和已知 project path。
- Device ID 只用于审计归因，不用于认证或授权。
- Learned rules 永远不能覆盖显式 deny。Deny 始终优先。
- 如果在公网运行 relay，必须使用 HTTPS/WSS，且不要使用不可信 relay。

完整安全模型见 [docs/security.md](docs/security.md)。

## 文档

- [docs/config.md](docs/config.md) -- 配置参考
- [docs/relay.md](docs/relay.md) -- Relay 部署指南
- [docs/security.md](docs/security.md) -- 安全模型
- [docs/open-source-scope.md](docs/open-source-scope.md) -- 开源范围

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
