# yokushi

A Rust CLI hook validator for AI coding agents. Reads hook JSON from stdin, matches the tool or command against YAML-configured suppression rules, and outputs a block decision in the format expected by the target agent.

## Overview

yokushi is designed to run as a `PreToolUse` hook. When an AI agent is about to call a tool, the agent sends a JSON payload to yokushi via stdin. yokushi evaluates the payload against a set of rules and either allows the action (exit 0) or blocks it (exit 2 or JSON response, depending on the agent type).

```
AI agent → PreToolUse hook → yokushi → allow / block
```

## Installation

```bash
cargo install --path .
```

Or build without installing:

```bash
cargo build --release
```

## Usage

```
yokushi [OPTIONS]
```

**Options:**

| Flag | Description |
|------|-------------|
| `-c, --config <FILE>` | Path to YAML config file |
| `-a, --agent <NAME>` | Agent type for output format (default: `default`) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

**Input:** Hook JSON via stdin.

**Output:** Depends on the agent's configured output mode (see [Configuration](#configuration)).

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Allow (also used for JSON-format blocks with `claude-code`) |
| `2` | Block (used by `default` / exit-mode agents) |

## Configuration

### Discovery order

yokushi searches for config in the following order (first found wins):

1. `--config <FILE>` CLI flag
2. `YOKUSHI_CONFIG` environment variable
3. `./yokushi.yaml` (current directory)
4. `~/.config/yokushi/config.yaml`

If no config file is found, all tool calls are allowed.

### Config format

```yaml
version: "1"

# Agent-specific output behavior.
# "json"  — write JSON to stdout and exit 0 (Claude Code format)
# "exit"  — write message to stderr and exit 2
agents:
  claude-code:
    output: json
  default:
    output: exit

# Suppression rules (matched in order; first match wins).
rules:
  - pattern: "git push"
    message: "Direct git push is prohibited. Please use pull requests."

  - pattern: "\\bawk\\b"
    message: "Use of 'awk' is prohibited. Use 'rg' instead."

  - tool: "Write"
    pattern: "\\.env$"
    message: "Writing to .env files is prohibited."

  - tool: "WebFetch"
    pattern: "example\\.com"
    message: "Access to example.com is restricted."
```

### Rule fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `pattern` | Yes | — | Regex pattern to match against the field value. Falls back to literal substring match if the pattern is invalid regex. |
| `message` | Yes | — | Human-readable reason shown when the rule blocks. |
| `tool` | No | `Bash` | Tool name to match (e.g. `Bash`, `Write`, `WebFetch`). |
| `field` | No | auto | `tool_input` field to match against. Auto-detected per tool (see table below). |

### Default fields per tool

| Tool | Default field |
|------|---------------|
| `Bash` | `command` |
| `Write`, `Edit`, `Read`, `Glob`, `Grep` | `file_path` |
| `WebFetch` | `url` |
| `WebSearch` | `query` |
| `Task` | `prompt` |
| *(other)* | `command` |

## Example

Block a `git push` command via the Claude Code hook format:

```bash
echo '{"tool_name":"Bash","hook_event_name":"PreToolUse","tool_input":{"command":"git push origin main"}}' \
  | yokushi --agent claude-code --config config.example.yaml
```

Expected output (stdout, exit 0):

```json
{"type":"hook","hook_event_name":"PreToolUse","decision":"block","reason":"blocked by rule 'git push' (matched: \"git push origin main\"): Direct git push is prohibited. Please use pull requests."}
```

### Claude Code hook setup

Add the following to your Claude Code settings (`.claude/settings.json`):

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "yokushi --agent claude-code"
          }
        ]
      }
    ]
  }
}
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy
```

## License

MIT
