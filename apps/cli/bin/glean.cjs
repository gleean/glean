#!/usr/bin/env node
/**
 * Locate workspace root when invoked via pnpm/npx and run `cargo run -p glean-cli`.
 */
const { spawnSync } = require("child_process");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const manifest = path.join(repoRoot, "Cargo.toml");
let args = process.argv.slice(2);
if (args[0] === "--") {
  args = args.slice(1);
}
const r = spawnSync(
  "cargo",
  ["run", "-q", "--manifest-path", manifest, "-p", "glean-cli", "--", ...args],
  { cwd: repoRoot, stdio: "inherit", env: process.env },
);
process.exit(r.status === null ? 1 : r.status);
