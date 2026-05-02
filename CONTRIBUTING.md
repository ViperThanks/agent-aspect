# Contributing to Agent Aspect

Thanks for your interest in contributing.

## Setup

```bash
rustup
cargo build
cargo test
```

## Development

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
scripts/smoke_test.sh
scripts/bridge_smoke_test.sh
scripts/relay_smoke_test.sh
```

## Pull Requests

- Keep PRs focused on a single concern.
- Include tests for new behavior.
- Run `cargo fmt` and `cargo clippy` before pushing.

## Design Changes

Changes to core design documents (event model, policy engine, enforcement modes) require an RFC. Use the **RFC** issue template.

## Reporting Issues

Use GitHub Issues. Include:
- Agent Aspect version (`agent-aspect --version` or commit hash)
- Agent and version (e.g., Claude Code 1.x, Codex CLI 0.x)
- Steps to reproduce
- Expected vs actual behavior
