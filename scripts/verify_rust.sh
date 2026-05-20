#!/usr/bin/env bash
# Rust gate: local / Cursor Hook. Mirrors rust.yml intent but includes glean-desktop.
# CI (rust.yml) uses --exclude glean-desktop and skips WebKit; run this script locally for full workspace.
# LanceDB needs protoc on PATH (CI: protobuf-compiler).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# glean-desktop: Tauri externalBin expects target/release/glean-<rustc-host>; see prepare_sidecar_bin.sh
cargo build -p glean-cli --release
bash scripts/prepare_sidecar_bin.sh

cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
