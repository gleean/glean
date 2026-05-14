# `glean-cli`

Single binary **`glean`**: the workspace default-run target for the Glean local-first knowledge engine. It hosts the long-running **daemon**, the **stdio MCP** server, and a few human-facing commands.

For storage layout, MCP tools, and embedding behavior, see the [repository root `README.md`](../../README.md).

**MCP debugging:** stdio lines must be JSON-RPC (examples in the root README). For iteration, run **`cargo test -p glean-cli mcp_protocol::router`** first; use **`cargo test -p glean-cli --test mcp_subprocess`** for a real `glean` binary + temp storage smoke test.

## Commands

| Command            | Role                                                                                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **`glean daemon`** | Watch the workspace (via `glean-core` notify pipeline), debounce, and run incremental sync into LanceDB + SQLite under `GLEAN_STORAGE_ROOT`. |
| **`glean mcp`**    | Short-lived **JSON-RPC 2.0** MCP server on **stdin/stdout**. Each stdin line must be valid JSON-RPC (plaintext like `initialize` is not accepted). Root `README.md` has **Manual stdin** echo examples. Do not write logs to **stdout** in this mode. |
| **`glean logs`**   | Print the tail of rolling log files under `{GLEAN_STORAGE_ROOT}/logs/`. Options: `-n` / `--lines`, `--source cli` / `daemon` / `all`.        |
| **`glean config`** | **`list`** (alias **`show`**): merged effective TOML to stdout. **`init`**: default writes **`$GLEAN_STORAGE_ROOT/config.toml`** (`~/.glean` when unset); with **`--workspace`**, writes **`<workspace>/.glean/config.toml`**. **`set KEY VALUE`**: patch one key in workspace `.glean/config.toml` only. Use **`--force`** on **`init`** to overwrite. |
| **`glean status`** | Emit version via **`tracing`** (stderr); useful for quick sanity checks.                                                                     |

Run **`glean --help`** and **`glean <command> --help`** for full Clap help.

## Environment

| Variable                   | Used by                                 | Meaning                                                                                             |
| -------------------------- | --------------------------------------- | --------------------------------------------------------------------------------------------------- |
| **`GLEAN_STORAGE_ROOT`**   | daemon, MCP, logs                       | Index root (default `~/.glean`).                                                                    |
| **`GLEAN_WORKSPACE_ROOT`** | daemon (`--workspace` alternative), MCP | Workspace boundary; daemon also accepts **`--workspace`** (defaults to cwd).                        |
| **`GLEAN_LOG`**            | all                                     | `tracing` `EnvFilter` string (e.g. `info`, `glean_core=debug`). If unset: MCP / `glean status` default to **info**; daemon uses **info** for rolling files and **warn** on stderr. `RUST_LOG` is ignored. |

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

Optional path dependency on **`glean-enterprise`** (workspace member). When enabled, the CLI augments the parser registry with enterprise parsers before opening the engine:

```bash
cargo build -p glean-cli --features enterprise --release
```

Default builds omit it; behavior matches community parsers only.

## Library

`glean-cli` is also a **library** (`glean_cli`) for integration tests and embedding the same `run()` entrypoint used by the binary.

## License

Apache-2.0, same as the rest of the workspace.
