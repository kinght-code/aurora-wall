# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust Cargo workspace for `aurora-wall`. The main crates live in `crates/`: `cli` provides the `aurora-wall` binary, `config` and `state` handle persistence, `daemon` manages background application, and backend/media crates isolate platform and asset logic. Supporting files live in `examples/` for sample config and `packaging/` for the Arch `PKGBUILD` and systemd user service. Design notes live in `../docs/`, and `../plan.txt` tracks the broader roadmap.

## Build, Test, and Development Commands

Run commands from the repository root.

- `cargo build --release`: build the release binary.
- `cargo run -q -p aurora-wall-cli -- doctor`: verify runtime dependencies and session detection.
- `cargo run -q -p aurora-wall-cli -- apply`: apply the saved wallpaper configuration.
- `cargo test --workspace`: run all workspace tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: enforce lint cleanliness before review.
- `cargo fmt --all --check`: verify Rust formatting.

## Coding Style & Naming Conventions

Use standard Rust formatting with 4-space indentation and `cargo fmt`. Keep crate names kebab-case, modules snake_case, types CamelCase, and functions snake_case. Follow the workspace lint posture in `Cargo.toml`: `dbg!`, `todo!`, and `unwrap()` are denied by Clippy, so prefer explicit error propagation with context.

## Testing Guidelines

There is no dedicated `tests/` tree yet, so add unit tests close to the code they cover or create integration tests under a crate’s `tests/` directory when behavior spans modules. Name tests for the behavior they verify, for example `applies_saved_video_wallpaper`. Run `cargo test --workspace` before opening a PR, and add regression coverage for CLI flows, config parsing, or backend selection when changing those paths.

## Commit & Pull Request Guidelines

Recent history uses very terse subjects (`aurora-wall`), which is not descriptive enough for ongoing work. Use short imperative commit titles instead, for example `cli: add doctor dependency check`. PRs should explain the user-visible change, note affected crates, list verification commands, and include terminal output or screenshots when behavior changes packaging, service output, or CLI UX.
