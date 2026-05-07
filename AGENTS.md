# Java Updater (java-updater-rs)

Guidelines for coding agents in this repository.

## Project Overview

The Java Updater is a Rust-based CLI tool designed to automate the process of downloading, unpacking, and replacing Java installations. It supports multiple vendors and provides a flexible configuration system.

- **Main Technologies:** Rust (2024 edition), `clap` (CLI), `reqwest` (HTTP), `serde` (YAML/JSON), `tracing` (logging).
- **Architecture:**
  - Multi-threaded execution using a thread pool for parallel installation processing.
  - Modular vendor support (Azul, Eclipse) implemented via feature flags.
  - Variable expansion in configuration paths (e.g., `${JU_CONFIG_ARCH}`, `${env.PATH}`).
  - Hook system for notifications on failure, success, or update.

## Building and Running

This project uses `just` (<https://github.com/casey/just>) as a command runner.

### Key Commands

- **Build:** `just build` (debug) or `just build-release` (release).
- **Run:** `just run -- [ARGS]` or `just run-full -- [ARGS]` (with all features).
- **Test:** `just test` (all features, all targets).
- **Lint:** `just clippy` or `just clippy-pedantic`.
- **Format:** `just fmt`.
- **Clean:** `just clean`.
- **Install:** `just install` (installs to cargo bin directory).

### Configuration

By default, the tool looks for `java-updater.yml` in the current directory. You can specify a custom config path using the `--config` argument.

## Development Conventions

- **Shell:** `just` recipes are configured to use PowerShell (`pwsh`).
- **Error Handling:** Uses `anyhow` for high-level error propagation and `thiserror` for library-level errors.
- **Logging:** Uses `tracing`. Adjust log level via `RUST_LOG` environment variable or `-v`, `-vv`, etc. CLI flags.
- **Formatting:** Adheres to `rustfmt` (see `.rustfmt.toml`). Use `just fmt` before committing.
- **Testing:**
  - Unit tests are located inline within `src/` modules.
  - Integration tests are located in the `tests/` directory.
- **Platform Support:**
  - Windows: Uses `zip` for extraction and taskbar progress support.
  - Unix: Uses `tar` and `flate2` for extraction.
- **Performance:** Release builds are highly optimized for size and speed (see `profile.release` in `Cargo.toml`).
