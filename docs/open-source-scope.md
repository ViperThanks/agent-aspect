# Open-Source Scope

## What Is In Scope

Everything in the `crates/` directory is open source:

| Crate | Description |
|-------|-------------|
| `core` | Shared types, SQLite audit store, rule engine, normalization, transcripts |
| `daemon` | Unix-socket daemon that evaluates hook requests |
| `hook-cli` | Agent hook entrypoint |
| `cli` | `agent-aspect` command-line management tool |
| `bridge` | Token-protected HTTP bridge and embedded web UI |
| `relay` | User-owned VPS relay for phone access |
| `shared_ui` | Shared frontend primitives used by bridge and relay |

Also in scope:

- `docs/` -- public-facing documentation
- `scripts/` -- smoke tests
- `docs/assets/` -- screenshots and diagrams

## What Is Not In Scope

Local working notes, machine-specific agent configuration, IDE state, secrets,
runtime databases, logs, and build outputs are excluded by ignore rules.

## What Is Not Built

These are explicitly out of scope for the project:

| Item | Reason |
|------|--------|
| Hosted multi-user cloud accounts | Agent Aspect is local-first. No accounts, no cloud sync. |
| Relay-side transcript storage | Relay does not store user content. Persistent user data stays on your Mac. |
| Native iOS / watchOS app | The web UI is the mobile interface. No native app planned. |
| Multi-tenant SaaS relay | One relay serves one Mac. Not a shared service. |
| Gemini runtime support | Gemini CLI is a candidate. No runtime experiments have been done. Not claimed as supported. |
| Learned rules overriding deny | Deny rules are always authoritative. No exceptions. |
| E2E encryption for relay | Relay sees traffic in transit. Use Tailscale or a trusted relay instead. |

## Agent Support Status

| Agent | Status | Notes |
|-------|--------|-------|
| Claude Code | Supported | Full hook integration, tested. |
| Codex CLI | Supported | Full hook integration, tested. |
| Kimi Code | Supported | Full hook integration, tested. |
| Gemini CLI | Candidate | Normalize stub exists. No runtime experiments. Not claimed as supported. |

"Candidate" means there is code scaffolding but no proven end-to-end flow. It is not advertised as a supported agent.

## Relay Positioning

The relay is:

- An optional remote phone channel
- A WebSocket proxy that does not store user content (only persists its own auth state)
- Self-hosted on infrastructure you control
- Not a default dependency

The relay is not:

- Cloud sync
- An account system
- A multi-tenant SaaS
- Required for Agent Aspect to function

## Contributing

Contributions are welcome. The project is in active development. Please open an issue before submitting a PR for anything beyond bug fixes.

## License

Licensed under the Apache License, Version 2.0. See [../LICENSE](../LICENSE).
