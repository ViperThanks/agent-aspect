# Provider Adapter SDK

> 目标：新增一个 CLI provider 时，只改 adapter / provider registry / fixture，不碰 routes、Relay UI、workflow runner。

## 接入边界

Provider 接入只允许写在这些层：

| 层 | 文件 | 职责 |
|----|------|------|
| 静态能力 | `crates/core/src/provider_registry.rs` | command、display name、capability defaults、TOML override |
| Hook 协议 | `crates/core/src/adapter.rs` + `normalize.rs` | payload 归一化、响应 envelope |
| Transcript | `crates/core/src/transcript.rs` + `title_import.rs` | transcript locator / parser / title extraction |
| Command | `crates/bridge/src/provider.rs` | new / continue command build |
| Fixture | `crates/core/src/**` tests | payload、transcript、command snapshot、round-trip |

`routes.rs`、Relay proxy、workflow runner 不应该出现新 provider 的分支。

## Capability Registry

`ProviderRegistry` 对外暴露 `ProviderCapabilities`：

| 字段 | 含义 |
|------|------|
| `supports_pretooluse` | 支持 before-tool hook 事件 |
| `supports_posttooluse` | 支持 after-tool hook 事件 |
| `supports_stop` | 支持 stop / turn-end lifecycle event |
| `supports_transcript` | Agent Aspect 能读取 transcript |
| `supports_resume` | 支持继续既有会话 |
| `supports_native_timeout` | provider 有原生 timeout 控制 |

Bridge `/run/context` 会把 capabilities 下发给 UI；Bridge / Relay 只读取 capability 展示功能，不写 provider 特判。

## 30 分钟接入清单

1. 在 `ProviderRegistry::builtin_defaults()` 增加 provider 默认配置。
2. 在 `AgentId` 增加枚举值，并实现 `AgentAdapter`。
3. 增加 normalize fixture：覆盖 allow / ask / deny 需要的最小 payload。
4. 增加 transcript fixture：至少覆盖 user、assistant、tool 三类消息。
5. 增加 command build snapshot：new 和 resume 各一例。
6. 跑 `cargo test -p agent-aspect-core -p agent-aspect-bridge` 和 shared UI tests。

## 设计原则

- Capability 是 UI 和 runner 的契约；不要让 UI 猜 provider。
- Provider 特判只留在 adapter 层；越过这条边界，后续每加一个 provider 都会把系统撕开一次。
- 未经 runtime 验证的能力默认 `false`，先可观察，再声明支持。
