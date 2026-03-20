# Corvus

**Corvus** is a diagnostic CLI for Claude Code — named after the crow genus, the smartest birds known for solving problems.

When using Claude Code with third-party providers (AWS Bedrock, relay services like OneAPI, apimart, etc.), three layers of model name mapping can break your setup:

```
Claude Code internal name → env var mapping → Provider's actual model ID
```

Any mismatch causes cryptic 400/403/500/503 errors. Corvus automates the diagnosis.

## Features

| Command | Description |
|---------|-------------|
| `corvus check` | Diagnose configuration and detect common issues |
| `corvus env` | Show all Claude Code environment variables and config |
| `corvus models` | Query provider's `/v1/models` and cross-reference with your config |
| `corvus stats` | Show usage statistics and cost estimates |
| `corvus sniff` | Local reverse proxy to intercept and analyze live API requests |

## Installation

```bash
# From source
cargo install --path .

# Or build release binary
cargo build --release
# Binary at: target/release/corvus
```

Requires Rust 1.70+.

## Usage

### `check` — Health report

```
corvus check [--skip-network]
```

```
Corvus - Claude Code Health Report
===================================

Provider: NewAPI/OneAPI relay service (https://api.example.com)

  [api_config]
    ✓  API key configured (sk-6****39B0)
    ✓  Base URL: https://api.example.com
    ℹ  Provider detected: NewAPI/OneAPI relay service

  [model_config]
    ⚠  ANTHROPIC_DEFAULT_SONNET_MODEL="claude-sonnet-4-6" — short alias may not be in provider's model list
       Fix: Try the full model ID: export ANTHROPIC_DEFAULT_SONNET_MODEL="claude-sonnet-4-6-20260220"
    ✓  ANTHROPIC_DEFAULT_HAIKU_MODEL="claude-haiku-4-5-20251001"

  [beta_flags]
    ✓  CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1

  [network]
    ✓  https://api.example.com reachable (339ms)

Summary: 8 ok, 1 info, 1 warning
```

Checks five categories: `api_config`, `model_config`, `beta_flags`, `files`, `network`.

### `env` — Environment summary

```
corvus env [--show-secrets]
```

Shows all `ANTHROPIC_*` / `CLAUDE_CODE_*` environment variables and config files, with API keys masked by default.

### `models` — Provider model list

```
corvus models [--url <url>] [--key <key>]
```

Queries `{base_url}/v1/models`, lists available model IDs, and highlights mismatches with your configured model names.

### `stats` — Usage statistics

```
corvus stats [--daily] [--by-model]
```

```
Corvus - Usage Statistics
=========================

Overview:
  First session: 2025-12-24
  Total sessions: 61 | Messages: 7350 | Startups: 368
  Longest session: 87 messages

Token Usage by Model:
  claude-sonnet-4-5-20250929:
    Input:              53.9K tokens  ($0.16)
    Output:            488.7K tokens  ($7.33)
    Cache Read:        129.1M tokens  ($38.74)
    Cache Write:        25.7M tokens  ($96.39)
    ─────────────────────────
    Estimated:                   $142.62

Active Hours:
  ░░░░░░░░░░▄▄░░█▄▄▄░░░░░░
  Peak: 14:00, 15:00, 17:00
```

Parses `~/.claude/stats-cache.json` and `~/.claude.json`. Cost estimation uses official Anthropic token pricing.

### `sniff` — Live request interceptor

```
corvus sniff [--port 8080] [--upstream <url>]
```

Starts a local reverse proxy. Point Claude Code at it to inspect every API request in real time:

```bash
# Terminal 1: start the proxy
corvus sniff --port 8080

# Terminal 2: run Claude Code through the proxy
ANTHROPIC_BASE_URL=http://localhost:8080 claude
```

```
Corvus Sniff - Listening on :8080 → https://api.example.com
═══════════════════════════════════════════════════════════

[14:23:01] POST /v1/messages
  Model: claude-sonnet-4-6  Messages: 12  Tools: 8
  Headers: anthropic-version=2023-06-01
  ⚠ Beta flags: computer-use-2025-01-24 (may not be supported by relay)
  → 200 OK (1.2s, 3,421 output tokens)
```

### Global options

```
-v, --verbose        Increase verbosity (-v, -vv)
--format text|json   Output format (default: text)
```

### Shell completions

```bash
corvus completions bash >> ~/.bashrc
corvus completions zsh  >> ~/.zshrc
corvus completions fish > ~/.config/fish/completions/corvus.fish
```

## Supported Providers

Corvus detects provider type from `ANTHROPIC_BASE_URL` and applies provider-specific rules:

| Provider | Detection |
|----------|-----------|
| Anthropic Direct | `api.anthropic.com` |
| AWS Bedrock | `amazonaws.com` / `bedrock` in URL, or `AWS_BEARER_TOKEN_BEDROCK` set |
| Google Vertex | `googleapis.com` / `vertex` in URL |
| apimart | `apimart` in URL |
| NewAPI / OneAPI | `one-api`, `oneapi`, `new-api`, `api2d` in URL |
| Custom relay | anything else |

Known issues by provider type are surfaced automatically in `corvus check`.

## Feature Flags

```toml
# Disable the sniff proxy (removes tokio/hyper, smaller binary)
cargo build --no-default-features --features completions

# Minimal build (no sniff, no completions)
cargo build --no-default-features
```

## Development

```bash
cargo test          # run all tests
cargo clippy        # lint
cargo build         # debug build
cargo build --release  # release build (~2.5MB stripped)
```

See [CLAUDE.md](CLAUDE.md) for architecture details and how to add new diagnostic rules or known models.

## License

MIT
