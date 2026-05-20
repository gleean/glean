"use client";

import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import {
	currentWorkspace,
	daemonRunning,
	getStatus,
	isTauri,
	openDirectoryDialog,
	pickWorkspace,
	tryRestoreWorkspace,
} from "@/lib/tauri";
import type { StatusReport } from "@/lib/types";

export type GleanAppContextValue = {
	workspace: string | null;
	status: StatusReport | null;
	daemonOk: boolean;
	loading: boolean;
	error: string | null;
	refresh: () => Promise<void>;
	selectWorkspace: () => Promise<void>;
	clearError: () => void;
	reportError: (message: string) => void;
	searchVersion: number;
};

const GleanAppContext = createContext<GleanAppContextValue | null>(null);

export function GleanAppProvider({ children }: { children: ReactNode }) {
	const [workspace, setWorkspace] = useState<string | null>(null);
	const [status, setStatus] = useState<StatusReport | null>(null);
	const [daemonOk, setDaemonOk] = useState(false);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [searchVersion, setSearchVersion] = useState(0);
	const refreshing = useRef(false);

	const refresh = useCallback(async () => {
		if (!isTauri()) {
			setLoading(false);
			return;
		}
		if (refreshing.current) return;
		refreshing.current = true;
		setLoading(true);
		try {
			const ws = await currentWorkspace();
			setWorkspace(ws);
			if (!ws) {
				setStatus(null);
				setDaemonOk(false);
				return;
			}
			const [st, daemon] = await Promise.allSettled([
				getStatus(),
				daemonRunning(),
			]);
			if (st.status === "fulfilled") setStatus(st.value);
			else throw new Error(String(st.reason));
			setDaemonOk(daemon.status === "fulfilled" ? daemon.value : false);
		} catch (e) {
			setError(e instanceof Error ? e.message : String(e));
		} finally {
			setLoading(false);
			refreshing.current = false;
		}
	}, []);

	const selectWorkspace = useCallback(async () => {
		if (!isTauri()) return;
		setLoading(true);
		setError(null);
		try {
			const path = await openDirectoryDialog();
			if (!path) return;
			await pickWorkspace(path);
			setSearchVersion((v) => v + 1);
			await refresh();
		} catch (e) {
			setError(e instanceof Error ? e.message : String(e));
		} finally {
			setLoading(false);
		}
	}, [refresh]);

	const clearError = useCallback(() => setError(null), []);
	const reportError = useCallback((message: string) => setError(message), []);

	useEffect(() => {
		queueMicrotask(() => {
			void (async () => {
				if (isTauri()) {
					try {
						await tryRestoreWorkspace();
					} catch (e) {
						setError(e instanceof Error ? e.message : String(e));
					}
				}
				await refresh();
			})();
		});
	}, [refresh]);

	const value = useMemo<GleanAppContextValue>(
		() => ({
			workspace,
			status,
			daemonOk,
			loading,
			error,
			refresh,
			selectWorkspace,
			clearError,
			reportError,
			searchVersion,
		}),
		[
			workspace,
			status,
			daemonOk,
			loading,
			error,
			refresh,
			selectWorkspace,
			clearError,
			reportError,
			searchVersion,
		],
	);

	return (
		<GleanAppContext.Provider value={value}>
			{children}
		</GleanAppContext.Provider>
	);
}

export function useGleanApp(): GleanAppContextValue {
	const ctx = useContext(GleanAppContext);
	if (!ctx) throw new Error("useGleanApp must be used within GleanAppProvider");
	return ctx;
}
