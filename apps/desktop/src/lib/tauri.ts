import { invoke, isTauri as tauriIsTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import type { SearchHit, StatusReport } from "./types";

export type { SearchHit, StatusReport } from "./types";

export function isTauri(): boolean {
	return typeof window !== "undefined" && tauriIsTauri();
}

export async function pickWorkspace(path: string): Promise<string> {
	return invoke<string>("pick_workspace", { path });
}

export async function getStatus(): Promise<StatusReport> {
	return invoke<StatusReport>("get_status");
}

export async function semanticSearch(
	query: string,
	limit?: number,
): Promise<SearchHit[]> {
	return invoke<SearchHit[]>("semantic_search", { query, limit });
}

export async function daemonRunning(): Promise<boolean> {
	return invoke<boolean>("daemon_running");
}

export async function currentWorkspace(): Promise<string | null> {
	return invoke<string | null>("current_workspace");
}

export async function openDirectoryDialog(): Promise<string | null> {
	const selected = await open({
		directory: true,
		multiple: false,
		title: "Select workspace folder",
	});
	if (selected === null) return null;
	if (Array.isArray(selected)) return null;
	return selected;
}
