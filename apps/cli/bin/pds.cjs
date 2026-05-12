#!/usr/bin/env node
/**
 * 从仓库任意位置经 pnpm / npx 调用时，定位 workspace 根目录并执行 `cargo run -p pds`。
 */
const { spawnSync } = require("child_process");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const manifest = path.join(repoRoot, "Cargo.toml");
let args = process.argv.slice(2);
// pnpm `run pds -- parse x` 会把 `--` 传给脚本，去掉以免传入 clap
if (args[0] === "--") {
  args = args.slice(1);
}
const r = spawnSync(
  "cargo",
  ["run", "-q", "--manifest-path", manifest, "-p", "pds", "--", ...args],
  { cwd: repoRoot, stdio: "inherit", env: process.env },
);
process.exit(r.status === null ? 1 : r.status);
