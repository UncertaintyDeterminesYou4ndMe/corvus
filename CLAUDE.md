# Corvus - Claude Code Diagnostic CLI

## Commands

```bash
cargo build --release    # build release binary (2.5MB)
cargo test               # run all tests (17 unit tests)
cargo install --path .   # install to ~/.cargo/bin/
```

## Architecture

```
src/
├── main.rs              # CLI entry (clap derive), command routing
├── cmd/                 # One file per subcommand
│   ├── check.rs         # Configuration diagnosis → health report
│   ├── env.rs           # Environment variable summary
│   ├── models.rs        # Provider model discovery (/v1/models)
│   ├── stats.rs         # Usage statistics + cost estimation
│   └── sniff.rs         # Reverse proxy entry point
├── config/              # Configuration reading layer
│   ├── env_vars.rs      # All ANTHROPIC_*/CLAUDE_* env vars
│   ├── settings.rs      # ~/.claude/settings.json parsing
│   └── stats_cache.rs   # stats-cache.json + .claude.json usage
├── diagnosis/           # Diagnostic rules engine
│   ├── model_names.rs   # Model ID validation, suffix checks
│   └── provider.rs      # Provider type detection from URL
├── proxy/               # Sniff proxy (feature-gated)
│   ├── server.rs        # tokio+hyper reverse proxy
│   └── analyzer.rs      # Request/response analysis
├── known_models.rs      # Static model registry (IDs, pricing)
├── display.rs           # Colored output, formatting
└── errors.rs            # thiserror enum
```

## Engineering Principles

- **KISS**: No unnecessary abstractions. Direct function calls, no traits where not needed.
- **YAGNI**: Only build what's used. Feature-gate expensive dependencies (tokio, hyper behind `sniff`).
- **SRP**: Each module has one job. config/ reads, diagnosis/ checks, display/ formats.
- **Security**: API keys masked in output by default. No logging of sensitive headers.

## Adding a New Diagnostic Rule

1. Add the check function in `diagnosis/model_names.rs` or `diagnosis/provider.rs`
2. Call it from `diagnosis::run_all_checks()` in `diagnosis/mod.rs`
3. Add a unit test in the same file
4. Run `cargo test`

## Adding a New Known Model

Edit `known_models.rs` → add entry to `KNOWN_MODELS` array with id, family, aliases, pricing.

## Feature Flags

- `sniff` (default: on): Enables tokio+hyper for the reverse proxy. Disable with `--no-default-features` for smaller binary.
- `completions` (default: off): Enables `clap_complete` for shell completion generation.
