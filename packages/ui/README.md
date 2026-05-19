# @glean/ui

Shared **shadcn/ui** (new-york) primitives for Glean frontends.

## Consumers

- [`apps/desktop`](../../apps/desktop) — Tauri + Next.js desktop shell

## Imports

```ts
import { Button } from "@glean/ui/components/ui/button";
import { cn } from "@glean/ui/lib/utils";
```

Styles: `@import "@glean/ui/globals.css"` in the app entry CSS.

## Add a component

Run from this directory (uses [`components.json`](./components.json)):

```bash
pnpm dlx shadcn@latest add <component>
```

## Exports

See [`package.json`](./package.json) `exports` field: `globals.css`, `lib/*`, `components/*`, `hooks/*`.
