# Glean Desktop

Tauri 2 shell with **Next.js 16** (static export) and Rust (`glean-desktop`) using **`glean-core`** + **`glean-host`** for read-only search and status. Indexing runs in a **sidecar** (`glean daemon`); the UI never writes LanceDB or SQLite.

**UI spec (local)**: [`.docs/05-Desktop-UI/README.md`](../../.docs/05-Desktop-UI/README.md) — document-driven iteration; not in public Git.

## UI overview

| Route | Purpose |
|-------|---------|
| `/` | Semantic search — dual-pane results + chunk preview; Cmd+K palette |
| `/index` | Index health — DB/vectors status, paths, legacy warnings |
| `/settings` | Global storage / log level (read-only); edit controls coming soon |
| `/about` | Version, storage model, sidecar notes |

Shell: top **ChromeBar** (workspace picker, search trigger), left **ActivityRail**, bottom **StatusBar** (daemon + version).

## Source layout

```text
apps/desktop/
├── src/
│   ├── app/                    # Next.js App Router (SSG)
│   ├── components/             # product UI (glean-*, theme-*)
│   ├── contexts/glean-app-context.tsx
│   └── lib/
│       ├── tauri.ts            # sole API module (+ types.ts)
│       └── utils.ts            # re-exports cn from @glean/ui
packages/ui/                    # @glean/ui — shared shadcn (new-york)
└── src-tauri/                  # Tauri + glean-desktop crate
```

## Prerequisites

- Rust **1.91+**, **`protoc`**, Node **18+**, **pnpm 9**
- Build the CLI (sidecar):

  ```bash
  cargo build -p glean-cli
  ./scripts/prepare_sidecar_bin.sh   # Tauri externalBin (required before tauri dev/build)
  ```

  Resolves `target/debug/glean` (or `GLEAN_BIN`). Bundled releases use `glean` next to the app executable.

## Development

From the repository root:

```bash
pnpm install
pnpm --filter @glean/desktop tauri dev
```

This is the **only** supported way to exercise workspace pick, daemon, and search. Browser-only `pnpm dev` shows a static “desktop shell required” message.

## Production build

```bash
cargo build -p glean-cli --release
./scripts/prepare_sidecar_bin.sh
pnpm --filter @glean/desktop build    # → apps/desktop/out
pnpm --filter @glean/desktop tauri build
```

The sidecar script copies `target/*/release/glean` to `target/release/glean-<triple>` for Tauri `externalBin` bundling.

## GitHub Release (release-please)

Desktop versions are managed by **[release-please](https://github.com/googleapis/release-please)** on push to `main`:

1. Merge feature PRs with **Conventional Commits** (`feat:`, `fix:`, `feat!:` for breaking).
2. Release-please opens a **Release PR** (bumps `package.json`, `tauri.conf.json`, `Cargo.toml`, `CHANGELOG.md`).
3. **Merge the Release PR** → tag `vX.Y.Z` + GitHub Release; **`build-desktop`** in the same run uploads **unsigned** `.dmg` / `.msi` and standalone **`glean-*`** CLI binaries.
4. To re-run assets for an existing tag (e.g. `v0.1.2`): **Actions → Release Desktop → Run workflow** with that tag.

Config: [`release-please-config.json`](../../release-please-config.json), [`.release-please-manifest.json`](../../.release-please-manifest.json).

Do **not** hand-tag for routine releases. See [`.docs/04-Ops-Security/desktop-release.md`](../../.docs/04-Ops-Security/desktop-release.md) for the full pipeline, commit rules, and Gatekeeper / SmartScreen notes.

## Frontend API

All invoke wrappers live in [`src/lib/tauri.ts`](src/lib/tauri.ts):

- `pickWorkspace`, `getStatus`, `semanticSearch`, `daemonRunning`, `currentWorkspace`
- `openDirectoryDialog` — used by context before `pick_workspace`
- `isTauri()` — gate non-desktop environments

See [`.docs/05-Desktop-UI/03-api-contract.md`](../../.docs/05-Desktop-UI/03-api-contract.md) for JSON shapes.

## Environment

| Variable | Purpose |
|----------|---------|
| `GLEAN_BIN` | Override path to `glean` for sidecar |
| `GLEAN_WORKSPACE_ROOT` | Set when picking workspace in UI |
| `GLEAN_STORAGE_ROOT` | Global home (`~/.glean`) |
| `TAURI_DEV_HOST` | Dev asset host (default `localhost`) |

## Verify

```bash
pnpm --filter @glean/desktop build
cargo clippy -p glean-desktop -- -D warnings
```

## Shared UI (`@glean/ui`)

Primitive components live in [`packages/ui`](../../packages/ui), not under `apps/desktop`.

```ts
import { Button } from "@glean/ui/components/ui/button";
```

Add a new shadcn component from the repo root:

```bash
cd packages/ui && pnpm dlx shadcn@latest add <component>
```

[`next.config.ts`](next.config.ts) sets `transpilePackages: ["@glean/ui"]`.

## Notes

- **Single writer**: only the sidecar runs `glean daemon`.
- MCP for Cursor remains **`glean mcp`** from the CLI.
- Workspace is **session-only** (not persisted across app restarts yet).
