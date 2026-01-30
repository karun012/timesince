# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`timesince` is a Rust CLI tool that tracks how long it's been since you last did something. Users add events (e.g., "workout"), mark them done, and query elapsed time. Data is stored as JSON at `~/.config/timesince/data.json`.

## Build & Development Commands

```bash
cargo build --verbose          # Build
cargo build --release          # Release build
cargo test --verbose           # Run all tests
cargo clippy                   # Lint
cargo fmt                      # Format code
```

A `justfile` provides shortcuts: `just run`, `just add <event>`, `just did <event>`, `just check <event>`, `just remove <event>`, `just list`.

The project uses [devenv.sh](https://devenv.sh) with direnv for reproducible Rust toolchain setup (stable channel).

## Architecture

All code lives in `src/main.rs`. Key components:

- **CLI parsing**: `clap` with derive macros. Accepts either a bare event name (`timesince workout`) or subcommands (`timesince did workout`). Subcommands: `did`, `remove`, `list`.
- **DataStore**: Handles JSON file I/O. Uses `dirs::config_dir()` to resolve the data path. Creates parent directories on first write.
- **Events**: Stored as `HashMap<String, DateTime<Utc>>` with `#[serde(flatten)]` for direct serialization. Each event maps to its last-completed UTC timestamp.
- **Display**: Uses `console` crate for terminal styling and `comfy-table` for tabular output. `chrono-humanize` provides human-readable durations.

## CI/CD

- **CI** (`.github/workflows/rust.yml`): Builds and tests on push to main and PRs (Ubuntu).
- **Release** (`.github/workflows/release.yml`): Triggered by `v*` tags. Builds binaries for Linux, macOS, and Windows using `upload-rust-binary-action`.

## Testing

Tests are in `src/main.rs` using `#[cfg(test)]`. They use the `tempfile` crate to create isolated data directories, avoiding filesystem side effects. Tests cover duration formatting, event CRUD operations.
