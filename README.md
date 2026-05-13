# Glean

Glean is a **local-first knowledge engine** with a Rust core. This repository is still early-stage; the current milestone focuses on a working Cargo workspace and an MCP-compatible **stdio** server (`glean mcp`).

## Building

Prerequisites: Rust stable (`rustup`), `cargo`, and **`protoc`** (Protocol Buffers compiler - required by LanceDB's Rust dependency chain). On macOS: `brew install protobuf`; on Debian/Ubuntu: `sudo apt-get install protobuf-compiler`.

```bash
cargo build -p glean-cli --release
```

The binary is emitted as `target/release/glean`.

## MCP (stdio)

Run the MCP server on stdin/stdout:

```bash
glean mcp
```

### Cursor (example)

Use **command + args** form when your client splits arguments:

- **Command**: `/absolute/path/to/target/release/glean`
- **Args**: `mcp`

Some UIs accept a single command string (`/path/to/glean mcp`); use whichever your editor supports.

### Implemented protocol surface (MVP)

This server speaks **newline-delimited JSON-RPC 2.0** with MCP-shaped methods:

- `initialize`
- `tools/list` (baseline tools only): `search_semantic`, `read_file_context`, `get_recent_changes`
- `tools/call`: **`search_semantic`** (Lance hybrid search: BM25 on chunk text plus embedding similarity via FastEmbed, fused with RRF; falls back to vector-only if FTS is unavailable), **`read_file_context`** (UTF-8 read under **`GLEAN_WORKSPACE_ROOT`** / cwd), **`get_recent_changes`** (SQLite shadow metadata).

Environment:

- **`GLEAN_STORAGE_ROOT`**: SQLite + Lance data (defaults to `~/.glean`).
- **`GLEAN_WORKSPACE_ROOT`**: workspace boundary for MCP / daemon (optional; defaults to cwd).
- **`RUST_LOG`**: optional filter for **`tracing`** on stderr and rolling files (e.g. `info`, `glean_core=debug`). If unset: MCP / `glean status` use **info** on stderr; **`glean daemon`** uses **warn** on stderr by default.

Rolling logs also land under **`{GLEAN_STORAGE_ROOT}/logs/`** (`cli.yyyy-mm-dd` / `daemon.yyyy-mm-dd`). Do not print diagnostics to **stdout** while running **`glean mcp`**. For quick inspection from a terminal, run **`glean logs`** (`-n` line count, `--source cli|daemon|all`); it does not install the tracing subscriber.

### Embedding model & rebuilding the vector index

Chunks are embedded with **FastEmbed** using **`AllMiniLM-L6-v2`** (**384-dimensional** `Float32` vectors) and stored in LanceDB `document_chunks` (see `.docs/02-Developer-Guide/lancedb-schema.md`). The ONNX artifacts download on first use under the FastEmbed cache directory.

If you upgrade Glean and see **`LanceDB schema mismatch`**, stop running processes, delete **`$GLEAN_STORAGE_ROOT/vectors`** (or the entire storage root), then run **`glean daemon`** again so the workspace is reindexed.

## Verification loop

```bash
chmod +x scripts/verify_rust.sh
./scripts/verify_rust.sh
```

## Optional: Cursor Hooks (local only)

This repo may ignore `.cursor/` for open-source hygiene. If you want automatic verification after an Agent completes a turn, configure a **user-level** Cursor hook (for example on `stop`) to run `scripts/verify_rust.sh`.

Treat **GitHub Actions** (`rust.yml`) as the shared CI gate for contributors (`fmt`, `clippy`, `tests`).

## Contributors

- Optional **Cursor Hooks** (e.g. on `stop`) pointing at `scripts/verify_rust.sh` are a **local productivity aid**. They are not required for correctness.
- **GitHub Actions** runs `cargo fmt`, `cargo clippy -D warnings`, and `cargo test --workspace` (same intent as `scripts/verify_rust.sh`).

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE).
