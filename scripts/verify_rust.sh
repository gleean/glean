#!/usr/bin/env bash
# Rust 门禁脚本：供本地 / Cursor Hook / CI 复用。
# LanceDB pulls protobuf codegen - ensure `protoc` is on PATH (CI installs protobuf-compiler).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
