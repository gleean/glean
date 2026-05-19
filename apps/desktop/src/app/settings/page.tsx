"use client";

import { Badge } from "@glean/ui/components/ui/badge";
import { Button } from "@glean/ui/components/ui/button";
import { Input } from "@glean/ui/components/ui/input";
import { Switch } from "@glean/ui/components/ui/switch";
import { FolderOpen } from "lucide-react";
import type { ReactNode } from "react";
import { GleanPathRow } from "@/components/glean-path-row";
import { useGleanApp } from "@/contexts/glean-app-context";

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
	const { status } = useGleanApp();

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
							<code className="rounded bg-muted px-2 py-1 font-mono text-[11px]">
								{status?.log_level ?? "—"}
							</code>
						</div>
					</div>
				</Group>

				<Group
					title="Watch & index"
					badge={
						<Badge variant="secondary" className="text-[10px]">
							Coming soon
						</Badge>
					}
				>
					<Row
						label="Watch debounce"
						description="Delay before reindexing after a file change."
						control={
							<Input
								disabled
								placeholder="500 ms"
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
						label="Skip files larger than 1 MB"
						description="Avoid indexing large binaries and minified bundles."
						control={<Switch disabled defaultChecked />}
					/>
				</Group>

				<Group title="Reranker">
					<Row
						label="Use reranker"
						description="On-device rerank of the top vector candidates. Adds 10–20ms."
						control={
							<Switch
								defaultChecked={status?.rerank_enabled ?? true}
								disabled
							/>
						}
					/>
				</Group>

				<Group title="Actions">
					<div className="flex flex-wrap gap-2 p-3">
						<Button variant="outline" size="sm" disabled className="gap-2">
							<FolderOpen className="h-3.5 w-3.5" />
							Open config folder
							<Badge variant="secondary" className="ml-1 text-[10px]">
								Soon
							</Badge>
						</Button>
						<Button variant="outline" size="sm" disabled className="gap-2">
							<FolderOpen className="h-3.5 w-3.5" />
							Reveal index
							<Badge variant="secondary" className="ml-1 text-[10px]">
								Soon
							</Badge>
						</Button>
					</div>
				</Group>
			</div>
		</div>
	);
}
