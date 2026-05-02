# Release Notes — v0.1.0

First public release of Agent Aspect.

## What It Is

Agent Aspect is a local runtime control plane for AI coding agents. It sits in front of Claude Code, Codex CLI, and Kimi Code, observes tool calls, applies local policy, records an audit trail, and gives you a control plane for approvals, conversations, jobs, and learned-rule suggestions.

Everything runs on your Mac. No accounts, no cloud sync, no hosted service.

## Features

### Policy Enforcement

- 10 built-in default rules covering risky operations: force push, bulk delete, sensitive file write, sudo, secret patterns, package install, pipe-to-shell, and git add all
- 4 enforcement modes: Observer (log only), Autonomous (auto-allow safe, ask risky), Guard (ask before writes), Paranoid (ask everything)
- Rules evaluated locally in the daemon — no external service calls
- Learn Mode suggests future auto-allow rules from your real ask-to-allow patterns (explicit deny always wins)

### Audit Trail

- Every tool call event and decision stored in local SQLite
- Filter and paginate audit history in the bridge UI
- Feedback per event (useful / noisy / wrong) feeds into Learn Mode
- Configurable retention with automatic purge

### Bridge Control Center

- Token-protected local HTTP UI at `http://127.0.0.1:7676`
- Home dashboard: current mode, pending approvals, recent conversations, device list
- Conversations tab: per-project grouping, chat messages, tool activity timeline
- Events tab: filter by action / tool / agent, approve or deny pending asks
- Run tab: submit whitelisted jobs or agent prompts, view live logs
- SSE push for real-time updates

### Remote Job Runner

- Whitelisted job kinds: git status, cargo test, smoke test, checkpoint status
- Agent prompt jobs: launch Claude / Kimi / Codex with a custom prompt, resume existing conversations
- Log streaming, cancellation, and stale-job recovery
- Runtime drift guard: warns before resuming if model, permissions, or project path have changed

### Multi-Agent Support

| Agent | Status |
|-------|--------|
| Claude Code | Supported — full hook integration, tested |
| Codex CLI | Supported — full hook integration, tested |
| Kimi Code | Supported — full hook integration, tested |
| Gemini CLI | Candidate — scaffolding only, not claimed as supported |

### Optional Relay

- Self-hosted relay on a VPS for phone access when Mac and phone are on different networks
- WebSocket proxy with HMAC-signed JWT tokens
- Rate-limited registration, body size limits, pending request caps
- Not required for normal use — local and LAN/Tailscale work without it

## Security

- Trust anchor is your Mac — daemon, rule engine, and audit store run locally
- Bridge token generated via `getrandom` (256-bit), stored at `~/.agent-aspect/bridge.token` (0600)
- All API endpoints require Bearer token auth except `GET /health`
- CORS disabled by default
- Jobs restricted to whitelisted kinds and known project paths
- Learned rules cannot override explicit deny
- Device IDs are for audit attribution only, not authentication

## Known Limitations

- macOS only (Apple Silicon tested)
- No end-to-end encryption between phone and Mac through relay
- No native iOS/watchOS app — mobile uses the web UI
- No multi-user authentication or authorization
- Gemini CLI is a candidate with scaffolding only
- Bridge uses synchronous HTTP (tiny_http) — suitable for single-user local use
- No full-text search across conversations

## Quickstart

```bash
cargo build --release
cargo install --path crates/cli
cargo install --path crates/daemon
cargo install --path crates/hook-cli
cargo install --path crates/bridge

agent-aspect init
agent-aspect doctor
agent-aspect mode guard
agent-aspect daemon start
agent-aspect bridge start
agent-aspect bridge token
open http://127.0.0.1:7676
```

## Docs

- [Configuration reference](docs/config.md)
- [Relay deployment guide](docs/relay.md)
- [Security model](docs/security.md)
- [Open-source scope](docs/open-source-scope.md)

## License

Apache License, Version 2.0
