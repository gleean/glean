# Glean

Glean is a **local-first knowledge engine** with a Rust core. This repository is still early-stage; the current milestone focuses on a working Cargo workspace and an MCP-compatible **stdio** server (`glean mcp`).

## Building

Prerequisites: Rust **stable 1.91+** (`rustup`), `cargo`, and **`protoc`** (Protocol Buffers compiler - required by LanceDB's Rust dependency chain). On macOS: `brew install protobuf`; on Debian/Ubuntu: `sudo apt-get install protobuf-compiler`.

```bash
cargo build -p glean-cli --release
```

The binary is emitted as `target/release/glean`.

## MCP (stdio)

Run the MCP server on stdin/stdout:

```bash
glean mcp
```

### Manual stdin (debugging)

The server reads **newline-delimited JSON**: each line must be a full **JSON-RPC 2.0** object (`jsonrpc`, `method`, and for requests an **`id`**). Typing plain text such as `initialize` is **not** valid JSON, so you will see **`Parse error` (`-32700`)** on stdout.

**`INFO` lines from `lance::dataset_events` on stderr** while the process starts are normal: the engine opens the Lance dataset before the read loop; they are not protocol traffic.

One-shot check (one request, then EOF):

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/release/glean mcp
```

Expect a single JSON line on stdout with a `result` payload (capabilities + `serverInfo`). Two requests on two lines:

```bash
printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | ./target/release/glean mcp
```

### Automated tests (recommended over manual stdio)

MCP behaviour is awkward to poke interactively because **stdin needs JSON lines**. Use the bundled tests instead:

- **Fast in-process tests** (`handle_json_line`):  
  `cargo test -p glean-cli mcp_protocol::router`
- **Real subprocess + temp storage** (`CARGO_BIN_EXE_glean` stdio framing):  
  `cargo test -p glean-cli --test mcp_subprocess`

The router tests cover `initialize`, invalid JSON / plaintext lines, unknown methods, `tools/list`, and a `tools/call`/`search_semantic` round-trip with indexed content.

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
- **`GLEAN_LOG`**: optional filter for **`tracing`** on stderr and rolling files (same syntax as `tracing_subscriber::EnvFilter`, e.g. `info`, `glean_core=debug`). If unset: MCP / `glean status` use **info** on both stderr and rolling files; **`glean daemon`** uses **info** for rolling files and **warn** on stderr. The `glean` binary does **not** read **`RUST_LOG`**.
- **Runtime TOML** (optional): merged from `$GLEAN_STORAGE_ROOT/config.toml` then `<workspace>/.glean/config.toml` (workspace root is `GLEAN_WORKSPACE_ROOT` or cwd). Reserved keys such as `[rerank]` support future behavior; internal design notes live under `.docs/02-Developer-Guide/configuration-system.md` when that directory exists locally. **`glean config list`** (alias **`show`**) prints the merged effective config (stdout); **`glean config init`** writes a template to **`$GLEAN_STORAGE_ROOT/config.toml`** by default (typically **`~/.glean/config.toml`**), or to **`<workspace>/.glean/config.toml`** when **`--workspace`** is set (use **`--force`** to overwrite); **`glean config set SECTION.field value`** patches a single scalar in the workspace `.glean/config.toml` only.

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
