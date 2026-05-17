# `glean-cli`

Single binary **`glean`**: the workspace default-run target for the Glean local-first knowledge engine.

Business logic (daemon loop, MCP JSON-RPC routing, config editor, status) lives in **[`glean-host`](../../packages/host)**. This crate is a **thin shell**: Clap routing, stdio MCP framing, `tracing-subscriber`, and human-readable output.

See also [`glean-core`](../../packages/core) (engine) and the [repository root `README.md`](../../README.md).

**MCP debugging:** stdio lines must be JSON-RPC (examples in the root README). For iteration, run **`cargo test -p glean-host mcp::router`** first; use **`cargo test -p glean-cli --test mcp_subprocess`** for a real `glean` binary + temp storage smoke test.

## CLI vs host vs core

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| Shell | **`glean-cli`** | `main`, Clap, stdin/stdout MCP loop, `glean logs`, logging install |
| Host | **`glean-host`** | `daemon`, `mcp`, `config`, `status`, `workspace`, `parsers` |
| Engine | **`glean-core`** | `GleanEngine`, LanceDB, pipeline, config merge (read) |

Tauri or other desktops should depend on **`glean-core` + `glean-host`**, not on `glean-cli`.

## Commands

| Command            | Role                                                                                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **`glean daemon`** | Watch the workspace; sync into **`<workspace>/.glean/`** (SQLite + Lance). Config from **`$GLEAN_STORAGE_ROOT/config.toml`**.                  |
| **`glean mcp`**    | Short-lived **JSON-RPC 2.0** MCP server on **stdin/stdout**. Do not write logs to **stdout**.                                                   |
| **`glean logs`**   | Tail rolling logs under **`{GLEAN_STORAGE_ROOT}/logs/`**.                                                                                    |
| **`glean config`** | **`list`** / **`init`** / **`set`** — global **`$GLEAN_STORAGE_ROOT/config.toml`** only.                                                     |
| **`glean status`** | Version, global storage, workspace index paths, legacy layout warnings.                                                                      |

Run **`glean --help`** and **`glean <command> --help`** for full Clap help.

## Environment

| Variable                   | Used by                                 | Meaning                                                                                             |
| -------------------------- | --------------------------------------- | --------------------------------------------------------------------------------------------------- |
| **`GLEAN_STORAGE_ROOT`**   | daemon, MCP, logs, config, rerank cache | Global home (default `~/.glean`). **Not** the per-project index.                                    |
| **`GLEAN_WORKSPACE_ROOT`** | daemon (`--workspace` alternative), MCP | Project root; index at **`<workspace>/.glean/`**.                                                    |
| **`GLEAN_LOG`**            | all                                     | `tracing` filter; overrides `[log].level` when set.                                                 |

## Build

From the **repository root** (workspace):

```bash
cargo build -p glean-cli --release
```

Binary: `target/release/glean`.

Prerequisites match the workspace: Rust stable and **`protoc`** (see root `README.md`).

### npm / pnpm shim (`bin/glean.cjs`)

[`package.json`](./package.json) exposes a **`bin`** named `glean` that runs [`bin/glean.cjs`](./bin/glean.cjs). That file is a small **Node** launcher, not the Rust executable: it finds the repo workspace root and spawns:

```bash
cargo run -q --manifest-path <repo-root>/Cargo.toml -p glean-cli -- <args>
```

Use it when the package is linked in a JS monorepo (e.g. `pnpm exec glean …`). Editors and production setups should still point at the **`cargo build --release`** binary when you want a fixed path and no compile step on each invocation.

### Cargo feature: `enterprise`

Forwards to **`glean-host/enterprise`**, which augments the parser registry via **`glean-enterprise`**:

```bash
cargo build -p glean-cli --features enterprise --release
```

Default builds omit it; behavior matches community parsers only.

## Library

`glean-cli` is also a **library** (`glean_cli`) for integration tests and the `main` binary entrypoint.

## License

Apache-2.0, same as the rest of the workspace.
