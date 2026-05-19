"use client";

import {
	Alert,
	AlertDescription,
	AlertTitle,
} from "@glean/ui/components/ui/alert";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@glean/ui/components/ui/collapsible";
import { Skeleton } from "@glean/ui/components/ui/skeleton";
import { AlertCircle, Check, ChevronDown, RefreshCw, X } from "lucide-react";
import { GleanNoWorkspace } from "@/components/glean-no-workspace";
import { GleanPathRow } from "@/components/glean-path-row";
import { useGleanApp } from "@/contexts/glean-app-context";
import { cn } from "@/lib/utils";

function StatPill({ ok, label }: { ok: boolean; label: string }) {
	return (
		<span
			className={cn(
				"inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10.5px] font-medium",
				ok
					? "border-accent/30 bg-accent-soft text-foreground"
					: "border-border bg-muted/50 text-muted-foreground",
			)}
		>
			{ok ? (
				<Check className="h-2.5 w-2.5 text-accent" />
			) : (
				<X className="h-2.5 w-2.5" />
			)}
			{label}
		</span>
	);
}

function StatTile({
	label,
	value,
	hint,
}: {
	label: string;
	value: string;
	hint?: string;
}) {
	return (
		<div className="flex flex-col gap-1 rounded-lg border border-border/70 bg-card px-4 py-3">
			<div className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
				{label}
			</div>
			<div className="text-[20px] font-semibold tracking-tight tabular-nums leading-none mt-1">
				{value}
			</div>
			{hint ? (
				<div className="text-[11px] text-muted-foreground">{hint}</div>
			) : null}
		</div>
	);
}

function PageHeader({
	title,
	subtitle,
	actions,
}: {
	title: string;
	subtitle?: string;
	actions?: React.ReactNode;
}) {
	return (
		<header className="sticky top-0 z-10 flex shrink-0 items-end justify-between gap-4 border-b border-border/60 bg-background/85 px-6 py-4 backdrop-blur">
			<div>
				<h1 className="text-[16px] font-semibold tracking-tight">{title}</h1>
				{subtitle ? (
					<p className="text-[12px] text-muted-foreground">{subtitle}</p>
				) : null}
			</div>
			{actions ? (
				<div className="flex items-center gap-2">{actions}</div>
			) : null}
		</header>
	);
}

function SectionCard({
	title,
	meta,
	children,
}: {
	title: string;
	meta?: string;
	children: React.ReactNode;
}) {
	return (
		<section className="overflow-hidden rounded-lg border border-border/70 bg-card">
			<div className="flex items-center justify-between border-b border-border/70 px-4 py-2.5">
				<h2 className="text-[12px] font-semibold tracking-tight">{title}</h2>
				{meta ? (
					<span className="text-[10.5px] text-muted-foreground">{meta}</span>
				) : null}
			</div>
			<div className="px-4 py-2">{children}</div>
		</section>
	);
}

export default function IndexPage() {
	const { workspace, status, loading, refresh } = useGleanApp();

	return (
		<div className="flex flex-col">
			<PageHeader
				title="Index"
				subtitle="Inspect storage, vectors, and reranker for the active workspace."
				actions={
					<button
						type="button"
						onClick={() => refresh()}
						disabled={loading}
						className="inline-flex items-center gap-1.5 rounded-md border border-border/70 bg-card px-2.5 py-1 text-[11.5px] font-medium transition-colors hover:border-accent/40 hover:bg-accent-soft disabled:opacity-50"
					>
						<RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />
						Refresh
					</button>
				}
			/>

			<div className="flex flex-col gap-5 px-6 py-5">
				{!workspace ? (
					<div className="flex justify-center pt-6">
						<GleanNoWorkspace />
					</div>
				) : !status ? (
					<div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
						{[0, 1, 2].map((i) => (
							<Skeleton key={i} className="h-20 w-full rounded-lg" />
						))}
					</div>
				) : (
					<>
						{/* Tile row */}
						<div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
							<StatTile
								label="Index version"
								value={`v${status.version}`}
								hint={`log · ${status.log_level}`}
							/>
							<div className="flex flex-col gap-1 rounded-lg border border-border/70 bg-card px-4 py-3">
								<div className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
									Storage
								</div>
								<div className="mt-2 flex flex-wrap gap-1">
									<StatPill ok={status.index_db_exists} label="SQLite" />
									<StatPill ok={status.index_vectors_exists} label="Vectors" />
								</div>
							</div>
							<div className="flex flex-col gap-1 rounded-lg border border-border/70 bg-card px-4 py-3">
								<div className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
									Config
								</div>
								<div className="mt-2 flex flex-wrap gap-1">
									<StatPill ok={status.global_config_exists} label="Global" />
									<StatPill
										ok={!status.deprecated_workspace_config}
										label="Workspace"
									/>
								</div>
							</div>
							<div className="flex flex-col gap-1 rounded-lg border border-border/70 bg-card px-4 py-3">
								<div className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
									Reranker
								</div>
								<div className="mt-2 flex flex-wrap gap-1">
									<StatPill
										ok={status.rerank_enabled}
										label={status.rerank_enabled ? "On" : "Off"}
									/>
									<StatPill
										ok={status.rerank_model_ready}
										label={status.rerank_model_ready ? "Model" : "Missing"}
									/>
								</div>
							</div>
						</div>

						{/* Alerts */}
						{status.legacy_global_index ? (
							<Alert variant="destructive">
								<AlertCircle className="h-4 w-4" />
								<AlertTitle>Legacy global index detected</AlertTitle>
								<AlertDescription>
									A legacy global index lives under{" "}
									<span className="font-mono">{status.storage_root}</span>.
								</AlertDescription>
							</Alert>
						) : null}
						{status.deprecated_workspace_config ? (
							<Alert>
								<AlertCircle className="h-4 w-4" />
								<AlertTitle>Deprecated workspace config</AlertTitle>
								<AlertDescription className="break-all font-mono text-[11px]">
									{status.deprecated_workspace_config}
								</AlertDescription>
							</Alert>
						) : null}

						<SectionCard title="Storage layout" meta="on-device · inspectable">
							<GleanPathRow label="Workspace" value={status.workspace_root} />
							<GleanPathRow label="Index root" value={status.index_root} />
							<GleanPathRow label="DB" value={status.index_db_path} />
							<GleanPathRow label="Vectors" value={status.index_vectors_path} />
							<GleanPathRow
								label="Global config"
								value={status.global_config_path}
							/>
							<GleanPathRow
								label="Global storage"
								value={status.storage_root}
							/>
						</SectionCard>

						<SectionCard title="Reranker">
							<Collapsible>
								<CollapsibleTrigger className="flex items-center gap-1.5 py-2 text-[11.5px] text-muted-foreground transition-colors hover:text-foreground data-[state=open]:text-foreground">
									<ChevronDown className="h-3 w-3 transition-transform [[data-state=open]_&]:rotate-180" />
									Show model path
								</CollapsibleTrigger>
								<CollapsibleContent>
									<GleanPathRow
										label="Model path"
										value={status.rerank_model_path}
									/>
								</CollapsibleContent>
							</Collapsible>
						</SectionCard>
					</>
				)}
			</div>
		</div>
	);
}
