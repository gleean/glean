# `glean-host`

Host runtime library for Glean: workspace resolution, parser assembly, global config editing, status aggregation, MCP JSON-RPC routing, and the daemon event loop.

There is **no** `main` in this crate. Shells link it as follows:

```
glean-cli  →  glean-host  →  glean-core
                ↓ (optional feature `enterprise`)
           glean-enterprise
```

## Modules

| Module | Role |
|--------|------|
| `workspace` | Resolve `GLEAN_WORKSPACE_ROOT` / cwd |
| `parsers` | `build_default_registry()` (community + optional enterprise) |
| `config` | Global `config.toml` init / set / list helpers |
| `status` | `StatusReport` + `collect_status()` |
| `mcp` | JSON-RPC router (`handle_json_line`) — transport-agnostic |
| `daemon` | `run_daemon_loop(DaemonRunOptions)` — inject `Arc<GleanEngine>` + `CancellationToken` |

## Tauri / desktop

Depend on **`glean-core`** + **`glean-host`** (not `glean-cli`).

| Mode | Pattern |
|------|---------|
| **Sidecar (recommended)** | Spawn `glean daemon`; UI uses read-only `GleanEngine` or status APIs |
| **Embedded daemon** | `glean_host::daemon::run_daemon_loop` with shared `Arc<GleanEngine>`; enforce single writer |
| **On-demand sync** | Call `glean_core::pipeline::run_incremental_sync` without a background loop |

MCP in-process is optional; for Cursor compatibility, sidecar `glean mcp` is enough.

## Enterprise

```bash
cargo build -p glean-cli --features enterprise --release
```

Enables `glean-host/enterprise` → `glean-enterprise` parser injection.

## Tests

```bash
cargo test -p glean-host
cargo test -p glean-host mcp::router
```

## License

Apache-2.0, same as the workspace.
