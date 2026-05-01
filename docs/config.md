# Configuration Reference

Agent Aspect stores its config at `~/.checkpoint/config.toml`. The file is created on first run with sensible defaults.

## Config File Location

```
~/.checkpoint/config.toml
```

## Fields

### `mode`

Enforcement mode. Controls how the rule engine evaluates tool calls.

| Value | Behavior |
|-------|----------|
| `observer` | Log everything, block nothing. Useful for watching without interfering. |
| `autonomous` | Auto-allow safe calls. Ask on risky operations (force push, rm -rf, sudo). |
| `guard` | Ask before most write operations. Default mode. |
| `paranoid` | Ask before every operation. |

```toml
mode = "guard"
```

### `bridge_addr`

Address the bridge HTTP server listens on.

```toml
bridge_addr = "127.0.0.1:7676"
```

Default: `127.0.0.1:7676`

### `bridge_lan_enabled`

Allow bridge to be accessible from other devices on your LAN.

```toml
bridge_lan_enabled = false
```

Default: `false`

Set to `true` if you want phone access over LAN/Tailscale without a relay.

### `log_level`

Log verbosity for the daemon.

```toml
log_level = "info"
```

Values: `trace`, `debug`, `info`, `warn`, `error`. Default: `info`

### `audit_retention_days`

Number of days to keep audit events. Older events are purged on daemon startup.

```toml
audit_retention_days = 90
```

Default: `90`

### `job_timeout_secs`

Timeout for remote jobs in seconds.

```toml
job_timeout_secs = 300
```

Default: `300` (5 minutes)

### `agent_prompt_timeout_secs`

Timeout for agent prompt continuation requests in seconds.

```toml
agent_prompt_timeout_secs = 600
```

Default: `600` (10 minutes)

### `job_max_output_kb`

Maximum output size per job in kilobytes. Output beyond this is truncated.

```toml
job_max_output_kb = 512
```

Default: `512`

### `relay_url`

WebSocket URL of a checkpoint-relay instance. Used by the bridge to connect to a remote relay for phone access.

```toml
relay_url = "wss://relay.example.com/ws"
```

Default: not set (relay disabled)

### `provider_binaries`

Override the binary path for specific agents. Legacy format; prefer `[providers.*]`.

```toml
[provider_binaries]
claude_code = "/usr/local/bin/claude"
```

### `providers`

Per-provider configuration overrides. Each key is a provider ID (`claude_code`, `codex_cli`, `kimi_code`).

```toml
[providers.claude_code]
enabled = true
command = "claude"
display_name = "Claude Code"
supports_resume = true
supports_new = true
```

All fields are optional. Unset fields inherit the built-in default for that provider.

| Field | Type | Description |
|-------|------|-------------|
| `enabled` | bool | Whether this provider is active. Default: `true` |
| `command` | string | CLI binary name. Used for PATH lookup. |
| `display_name` | string | Human-readable name shown in the UI. |
| `supports_resume` | bool | Whether the provider supports resuming sessions. |
| `supports_new` | bool | Whether the provider supports creating new sessions. |
| `supports_stop_observer` | bool | Whether the provider supports stopping observer sessions. |
| `supports_permission_passthrough` | bool | Whether the provider can inherit a runtime permission mode. Default: `false` |
| `permission_mode_cli_arg` | string? | CLI arg injected when bypass mode is active (e.g. `"--dangerously-skip-permissions"`). |
| `permission_mode_env_vars` | [(string, string)] | Env vars set when bypass mode is active (e.g. `[["VIBE_ISLAND_SKIP", "1"]]`). |

Permission passthrough fields are currently verified for Claude Code only. Other providers (Codex CLI, Kimi Code) default to disabled.

## Environment Variables

| Variable | Effect |
|----------|--------|
| `CHECKPOINT_BRIDGE_ADDR` | Override bridge listen address (e.g. `0.0.0.0:7676`) |
| `CHECKPOINT_RELAY_URL` | Override relay WebSocket URL |
| `CHECKPOINT_RELAY_SETUP_TOKEN` | One-time registration token for relay |
| `CHECKPOINT_MODE` | Override enforcement mode (daemon) |
| `CHECKPOINT_AGENT` | Override agent detection (for testing) |
| `CHECKPOINT_ASSUME_NO_TTY` | Disable TTY prompts in hook-cli (non-interactive environments) |
| `VIBE_ISLAND_SKIP` | Skip Vibe Island permission hook (fallback fix) |

## Example Config

```toml
mode = "guard"
bridge_addr = "127.0.0.1:7676"
bridge_lan_enabled = true
log_level = "info"
audit_retention_days = 90
job_timeout_secs = 300
relay_url = "wss://relay.example.com/ws"

[providers.claude_code]
command = "/usr/local/bin/claude"

[providers.codex_cli]
command = "codex"
```
