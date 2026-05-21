import type { NextConfig } from "next";
import path from "node:path";
import { fileURLToPath } from "node:url";

const isProd = process.env.NODE_ENV === "production";
const internalHost = process.env.TAURI_DEV_HOST || "localhost";
/** Monorepo root (`glean/`), not a parent directory that happens to have another lockfile. */
const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

const nextConfig: NextConfig = {
	output: "export",
	turbopack: {
		root: workspaceRoot,
	},
	transpilePackages: ["@glean/ui"],
	images: {
		unoptimized: true,
	},
	assetPrefix: isProd ? undefined : `http://${internalHost}:3000`,
};

export default nextConfig;
