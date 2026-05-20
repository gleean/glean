"use client";

import { Badge } from "@glean/ui/components/ui/badge";
import { Button } from "@glean/ui/components/ui/button";
import { Input } from "@glean/ui/components/ui/input";
import { Switch } from "@glean/ui/components/ui/switch";
import { FolderOpen, Loader2 } from "lucide-react";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import { GleanPathRow } from "@/components/glean-path-row";
import { useGleanApp } from "@/contexts/glean-app-context";
import {
	getGlobalConfigToml,
	initGlobalConfig,
	revealPathInFileManager,
	setGlobalConfigKey,
} from "@/lib/tauri";

function PageHeader({ title, subtitle }: { title: string; subtitle?: string }) {
	return (
		<header className="sticky top-0 z-10 flex shrink-0 items-end justify-between gap-4 border-b border-border/60 bg-background/85 px-6 py-4 backdrop-blur">
			<div>
				<h1 className="text-[16px] font-semibold tracking-tight">{title}</h1>
				{subtitle ? (
					<p className="text-[12px] text-muted-foreground">{subtitle}</p>
				) : null}
			</div>
		</header>
	);
}

function Row({
	label,
	description,
	control,
}: {
	label: string;
	description?: string;
	control: ReactNode;
}) {
	return (
		<div className="flex items-center justify-between gap-4 border-b border-border/60 px-4 py-3 last:border-b-0">
			<div className="min-w-0 flex-1">
				<div className="text-[12.5px] font-medium">{label}</div>
				{description ? (
					<div className="text-[11.5px] text-muted-foreground">
						{description}
					</div>
				) : null}
			</div>
			<div className="shrink-0">{control}</div>
		</div>
	);
}

function Group({
	title,
	description,
	children,
	badge,
}: {
	title: string;
	description?: string;
	children: ReactNode;
	badge?: ReactNode;
}) {
	return (
		<section>
			<div className="mb-2 flex items-center justify-between gap-2 px-1">
				<div className="flex items-center gap-2">
					<h2 className="text-[10.5px] font-semibold uppercase tracking-wider text-muted-foreground">
						{title}
					</h2>
					{badge}
				</div>
				{description ? (
					<span className="text-[11px] text-muted-foreground">
						{description}
					</span>
				) : null}
			</div>
			<div className="overflow-hidden rounded-lg border border-border/70 bg-card">
				{children}
			</div>
		</section>
	);
}

export default function SettingsPage() {
	const { status, refresh, reportError, clearError } = useGleanApp();
	const [saving, setSaving] = useState(false);
	const [rerankEnabled, setRerankEnabled] = useState(true);
	const [logLevel, setLogLevel] = useState("info");
	const [watchInterval, setWatchInterval] = useState("10");
	const [maxFileSize, setMaxFileSize] = useState("10485760");
	const [useGitignore, setUseGitignore] = useState(true);

	const loadConfigFields = useCallback(async () => {
		try {
			const toml = await getGlobalConfigToml();
			const rerankMatch = toml.match(/\[rerank\][\s\S]*?enabled\s*=\s*(\w+)/);
			if (rerankMatch) {
				setRerankEnabled(rerankMatch[1].toLowerCase() === "true");
			}
			const logMatch = toml.match(/\[log\][\s\S]*?level\s*=\s*"?([^"\n]+)"?/);
			if (logMatch) setLogLevel(logMatch[1].trim());
			const watchMatch = toml.match(
				/\[indexing\][\s\S]*?watch_interval\s*=\s*(\d+)/,
			);
			if (watchMatch) setWatchInterval(watchMatch[1]);
			const maxMatch = toml.match(
				/\[indexing\][\s\S]*?max_file_size\s*=\s*(\d+)/,
			);
			if (maxMatch) setMaxFileSize(maxMatch[1]);
			const gitMatch = toml.match(
				/\[indexing\][\s\S]*?use_gitignore\s*=\s*(\w+)/,
			);
			if (gitMatch) {
				setUseGitignore(gitMatch[1].toLowerCase() === "true");
			}
		} catch (e) {
			reportError(e instanceof Error ? e.message : String(e));
		}
	}, [reportError]);

	useEffect(() => {
		void loadConfigFields();
	}, [loadConfigFields]);

	const applyKey = async (key: string, value: string) => {
		setSaving(true);
		clearError();
		try {
			await setGlobalConfigKey(key, value);
			await refresh();
			await loadConfigFields();
		} catch (e) {
			reportError(e instanceof Error ? e.message : String(e));
		} finally {
			setSaving(false);
		}
	};

	const onRerankChange = async (checked: boolean) => {
		setRerankEnabled(checked);
		await applyKey("rerank.enabled", checked ? "true" : "false");
	};

	const onLogLevelBlur = async () => {
		const level = logLevel.trim().toLowerCase();
		if (!level) return;
		await applyKey("log.level", level);
	};

	const onWatchIntervalBlur = async () => {
		const n = Number.parseInt(watchInterval, 10);
		if (Number.isNaN(n) || n < 0) {
			reportError("watch_interval must be a non-negative integer (seconds)");
			return;
		}
		await applyKey("indexing.watch_interval", String(n));
	};

	const onMaxFileSizeBlur = async () => {
		const n = Number.parseInt(maxFileSize, 10);
		if (Number.isNaN(n) || n < 0) {
			reportError("max_file_size must be a non-negative integer (bytes)");
			return;
		}
		await applyKey("indexing.max_file_size", String(n));
	};

	const onGitignoreChange = async (checked: boolean) => {
		setUseGitignore(checked);
		await applyKey("indexing.use_gitignore", checked ? "true" : "false");
	};

	const onInitConfig = async () => {
		setSaving(true);
		clearError();
		try {
			await initGlobalConfig(false);
			await refresh();
			await loadConfigFields();
		} catch (e) {
			reportError(e instanceof Error ? e.message : String(e));
		} finally {
			setSaving(false);
		}
	};

	const openConfigFolder = async () => {
		const path = status?.global_config_path;
		if (!path) return;
		const dir = path.replace(/[/\\][^/\\]+$/, "");
		try {
			await revealPathInFileManager(dir);
		} catch (e) {
			reportError(e instanceof Error ? e.message : String(e));
		}
	};

	const revealIndex = async () => {
		const path = status?.index_root;
		if (!path) return;
		try {
			await revealPathInFileManager(path);
		} catch (e) {
			reportError(e instanceof Error ? e.message : String(e));
		}
	};

	return (
		<div className="flex flex-col">
			<PageHeader
				title="Settings"
				subtitle="Storage, indexing behaviour, and the daemon."
			/>

			<div className="flex flex-col gap-6 px-6 py-5">
				<Group
					title="Global storage"
					description="GLEAN_STORAGE_ROOT to override"
				>
					<div className="px-4 py-1">
						<GleanPathRow
							label="Storage root"
							value={status?.storage_root ?? ""}
						/>
						<GleanPathRow
							label="Config path"
							value={status?.global_config_path ?? ""}
							badge={
								status ? (
									<Badge
										variant={
											status.global_config_exists ? "default" : "secondary"
										}
										className="text-[10px]"
									>
										{status.global_config_exists ? "exists" : "missing"}
									</Badge>
								) : null
							}
						/>
						<div className="grid grid-cols-1 gap-1 border-b py-3 last:border-b-0 sm:grid-cols-[140px_1fr] sm:gap-4">
							<div className="text-[12px] font-medium text-muted-foreground">
								Log level
							</div>
							<Input
								value={logLevel}
								onChange={(e) => setLogLevel(e.target.value)}
								onBlur={() => void onLogLevelBlur()}
								disabled={saving}
								className="h-8 font-mono text-[11px]"
								placeholder="info"
							/>
						</div>
					</div>
				</Group>

				<Group title="Watch & index">
					<Row
						label="Watch debounce"
						description="Seconds between daemon polls after a file change (0 = initial sync only)."
						control={
							<Input
								value={watchInterval}
								onChange={(e) => setWatchInterval(e.target.value)}
								onBlur={() => void onWatchIntervalBlur()}
								disabled={saving}
								className="h-8 w-28 text-[12px]"
							/>
						}
					/>
					<Row
						label="Re-index on startup"
						description="Walk the workspace and pick up changes on launch."
						control={<Switch disabled />}
					/>
					<Row
						label="Max file size (bytes)"
						description="Files larger than this are skipped during indexing."
						control={
							<Input
								value={maxFileSize}
								onChange={(e) => setMaxFileSize(e.target.value)}
								onBlur={() => void onMaxFileSizeBlur()}
								disabled={saving}
								className="h-8 w-32 font-mono text-[11px]"
							/>
						}
					/>
					<Row
						label="Use .gitignore"
						description="Respect ignore rules when scanning the workspace."
						control={
							<Switch
								checked={useGitignore}
								onCheckedChange={(v) => void onGitignoreChange(v)}
								disabled={saving}
							/>
						}
					/>
				</Group>

				<Group title="Reranker">
					<Row
						label="Use reranker"
						description="On-device rerank of the top vector candidates."
						control={
							<Switch
								checked={rerankEnabled}
								onCheckedChange={(v) => void onRerankChange(v)}
								disabled={saving}
							/>
						}
					/>
				</Group>

				<Group title="Actions">
					<div className="flex flex-wrap gap-2 p-3">
						{status && !status.global_config_exists ? (
							<Button
								variant="default"
								size="sm"
								disabled={saving}
								onClick={() => void onInitConfig()}
							>
								Create default config
							</Button>
						) : null}
						<Button
							variant="outline"
							size="sm"
							className="gap-2"
							disabled={!status?.global_config_path || saving}
							onClick={() => void openConfigFolder()}
						>
							{saving ? (
								<Loader2 className="h-3.5 w-3.5 animate-spin" />
							) : (
								<FolderOpen className="h-3.5 w-3.5" />
							)}
							Open config folder
						</Button>
						<Button
							variant="outline"
							size="sm"
							className="gap-2"
							disabled={!status?.index_root || saving}
							onClick={() => void revealIndex()}
						>
							<FolderOpen className="h-3.5 w-3.5" />
							Reveal index
						</Button>
					</div>
					{saving ? (
						<p className="border-t border-border/60 px-4 py-2 text-[11px] text-muted-foreground">
							Saving config and restarting daemon…
						</p>
					) : null}
				</Group>
			</div>
		</div>
	);
}
