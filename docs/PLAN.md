# Agent Aspect — Roadmap

> A local runtime control plane for AI coding agents.

This document describes the public roadmap for Agent Aspect. It covers the project vision, architecture, and phased delivery plan.

## Vision

AI coding agents (Claude Code, Codex CLI, Kimi Code, etc.) can modify files, run shell commands, and make decisions at machine speed. Agent Aspect sits in front of these agents, observes tool calls, applies local policy, records an audit trail, and gives the user a control plane for approvals, conversations, jobs, and learned-rule suggestions.

Core product rule:

> Express mechanisms exhaustively, expose policy gradually, and make trust traceable.

## Architecture

```text
AI Agents
  ├─ Claude Code hooks
  ├─ Codex CLI hooks / transcripts
  └─ Kimi Code hooks
        │
        ▼
agent-aspect-hook ── Unix socket ── agent-aspectd
        │                           │
        │                           ├─ Rule engine
        │                           ├─ Learned-rule fallback
        │                           └─ SQLite audit.db
        │
        ▼
agent-aspect-bridge ── HTTP + token ── Web / mobile browser
        │
        └──── WSS ── agent-aspect-relay ── Phone browser  (optional)
```

## Design Principles

1. **Mechanisms are exhaustive.** The event system must describe all event phases (before / mid / after) and all action combinations. The UI does not need to expose everything at once.

2. **Policy is gradual.** Users start with a default mode and Learn Mode suggestions. Full event binding lives in configuration for advanced users.

3. **Trust is traceable.** Every active rule carries a source stamp: Default, Learned, User, or Community. When a user is blocked, they see "who blocked this" before "how."

## Enforcement Modes

| Mode | Behavior |
|------|----------|
| **Observer** | Log everything, block nothing. Good for trying a new agent. |
| **Autonomous** | Auto-allow safe calls, ask on risky ones (main branch, high-risk shell, budget). |
| **Guard** | Ask before most write operations. |
| **Paranoid** | Ask before everything. |

## Phased Delivery

### Phase 1 — Mac Core (current)

- Hook integration with Claude Code, Codex CLI, Kimi Code
- Rule engine with default policy rules
- SQLite audit store
- Bridge HTTP UI (Home, Conversations, Audit, Run)
- CLI tool: `agent-aspect init / doctor / mode / bridge / daemon`
- Learn Mode v1 (observe and suggest)
- Optional relay for remote phone access
- Conversation import and overview
- Job runner with cancellation and log streaming

### Phase 2 — Mobile (planned)

- SwiftUI iPhone UI: mode switch + pending approvals + sessions
- watchOS approval MVP
- Tailscale-first / relay fallback transport

### Phase 3 — Ecosystem (future)

- Policy Studio (Mac standalone window)
- Community rule set import/export
- Additional agent integrations
- Cross-agent event protocol

## Event Model

Agent Aspect uses a six-column event model internally:

| Phase | Event | Example Policy | Channel | Action | Source |
|-------|-------|---------------|---------|--------|--------|
| before | `tool.request` | Block push to main | iPhone | `require_approval` | Default |
| before | `shell.high_risk` | rm -rf circuit breaker | Watch | `deny` | User |
| mid | `question.asked` | Only push critical during commute | iPhone | `notify` | Learned |
| after | `task.completed` | Summary only at night | Watch | `summarize` | User |
| after | `diff.large` | Audit large changes | Audit log | `log` | Community |

The system supports all combinations; the product UI does not expose them all at once.

## Rule Sources

Every active rule carries a source stamp:

- **Default** — Product preset
- **Learned (day N, X obs)** — Suggested by Learn Mode
- **User** — Written by the user
- **Community@name** — Imported from a community rule set

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines. For changes to the core design principles or event model, open an RFC using the issue template.
