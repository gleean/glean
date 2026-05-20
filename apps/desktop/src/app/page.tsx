"use client";

import { ArrowRight, FileText, Loader2, Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { GleanNoWorkspace } from "@/components/glean-no-workspace";
import { useGleanApp } from "@/contexts/glean-app-context";
import { revealPathInFileManager, semanticSearch } from "@/lib/tauri";
import type { SearchHit } from "@/lib/types";
import { cn } from "@/lib/utils";

const SUGGESTIONS = [
	"How does indexing work?",
	"rerank model path",
	"Q1 roadmap",
	"daemon server route",
];

function basename(p: string) {
	const parts = p.split(/[\\/]/).filter(Boolean);
	return parts[parts.length - 1] ?? p;
}

function highlight(text: string, q: string) {
	if (!q.trim()) return text;
	const tokens = q
		.trim()
		.split(/\s+/)
		.filter((t) => t.length > 1)
		.map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
	if (tokens.length === 0) return text;
	const re = new RegExp(`(${tokens.join("|")})`, "gi");
	const parts = text.split(re);
	return parts.map((p, i) =>
		re.test(p) ? (
			<mark
				key={i.toString()}
				className="rounded-sm bg-accent-soft px-0.5 text-foreground"
			>
				{p}
			</mark>
		) : (
			<span key={i.toString()}>{p}</span>
		),
	);
}

export default function HomePage() {
	const { workspace, status, reportError, clearError } = useGleanApp();
	const [query, setQuery] = useState("");
	const [hits, setHits] = useState<SearchHit[]>([]);
	const [loading, setLoading] = useState(false);
	const [submitted, setSubmitted] = useState<string | null>(null);
	const [activeIdx, setActiveIdx] = useState<number>(0);
	const inputRef = useRef<HTMLInputElement>(null);

	useEffect(() => {
		inputRef.current?.focus();
	}, []);

	const runSearch = async (q: string) => {
		if (!q.trim() || !workspace) return;
		clearError();
		setLoading(true);
		setSubmitted(q);
		try {
			const r = await semanticSearch(q, 32);
			setHits(r);
			setActiveIdx(0);
		} catch (e) {
			setHits([]);
			reportError(e instanceof Error ? e.message : String(e));
		} finally {
			setLoading(false);
		}
	};

	const active = useMemo(() => hits[activeIdx] ?? null, [hits, activeIdx]);

	if (!workspace) {
		return (
			<div className="flex h-full items-center justify-center p-8">
				<GleanNoWorkspace />
			</div>
		);
	}

	return (
		<div className="flex h-full flex-col">
			{/* Search bar */}
			<div className="border-b border-border/60 bg-background/60 px-4 py-3">
				<form
					onSubmit={(e) => {
						e.preventDefault();
						runSearch(query);
					}}
					className="flex items-center gap-2 rounded-lg border border-border/70 bg-card px-3 py-2 transition-colors focus-within:border-accent/60 focus-within:glean-glow"
				>
					{loading ? (
						<Loader2 className="h-4 w-4 shrink-0 animate-spin text-accent" />
					) : (
						<Search className="h-4 w-4 shrink-0 text-muted-foreground" />
					)}
					<input
						ref={inputRef}
						value={query}
						onChange={(e) => setQuery(e.target.value)}
						placeholder="Ask anything about your workspace…"
						className="min-w-0 flex-1 bg-transparent text-[14px] outline-none placeholder:text-muted-foreground"
					/>
					{status?.rerank_enabled && status.rerank_model_ready ? (
						<span className="hidden items-center gap-1 rounded border border-accent/30 bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-foreground sm:inline-flex">
							<span className="size-1 rounded-full bg-accent" />
							reranked
						</span>
					) : null}
				</form>

				{!submitted && hits.length === 0 ? (
					<div className="mt-3 flex flex-wrap items-center gap-1.5">
						<span className="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground">
							Try
						</span>
						{SUGGESTIONS.map((s) => (
							<button
								key={s}
								type="button"
								onClick={() => {
									setQuery(s);
									runSearch(s);
								}}
								className="rounded-full border border-border/70 bg-card px-2.5 py-0.5 text-[11.5px] text-muted-foreground transition-colors hover:border-accent/40 hover:text-foreground"
							>
								{s}
							</button>
						))}
					</div>
				) : null}
			</div>

			{/* Two-pane workbench */}
			<div className="flex min-h-0 flex-1">
				{/* Results list */}
				<div className="flex w-[42%] min-w-[320px] flex-col border-r border-border/60">
					<div className="flex items-center justify-between border-b border-border/60 px-4 py-2 text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground">
						<span>Results</span>
						<span className="font-mono normal-case tracking-normal text-[11px] text-muted-foreground/80">
							{hits.length}
						</span>
					</div>
					<div className="flex-1 overflow-y-auto">
						{!submitted ? (
							<EmptyHint label="Search to see results" />
						) : loading && hits.length === 0 ? (
							<EmptyHint label="Searching…" />
						) : hits.length === 0 ? (
							<EmptyHint label={`No matches for "${submitted}"`} />
						) : (
							<ul className="flex flex-col py-1">
								{hits.map((h, i) => (
									<li key={`${h.path}-${i}`}>
										<button
											type="button"
											onClick={() => setActiveIdx(i)}
											className={cn(
												"flex w-full items-start gap-2.5 border-l-2 px-4 py-2.5 text-left transition-colors",
												i === activeIdx
													? "border-l-accent bg-sidebar-accent/70"
													: "border-l-transparent hover:bg-muted/50",
											)}
										>
											<FileText className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
											<div className="min-w-0 flex-1">
												<div className="truncate text-[12.5px] font-medium">
													{highlight(basename(h.path), submitted ?? "")}
												</div>
												<div className="truncate font-mono text-[10.5px] text-muted-foreground/80">
													{h.path}
												</div>
												<p className="mt-1 line-clamp-2 text-[12px] leading-relaxed text-muted-foreground">
													{highlight(h.preview, submitted ?? "")}
												</p>
											</div>
										</button>
									</li>
								))}
							</ul>
						)}
					</div>
				</div>

				{/* Preview pane */}
				<div className="flex min-w-0 flex-1 flex-col bg-sidebar/30">
					{active ? (
						<>
							<div className="flex items-center justify-between border-b border-border/60 px-5 py-3">
								<div className="min-w-0">
									<div className="truncate text-[13px] font-semibold tracking-tight">
										{basename(active.path)}
									</div>
									<div className="truncate font-mono text-[11px] text-muted-foreground">
										{active.path}
									</div>
								</div>
								<button
									type="button"
									title="Reveal in file manager"
									className="inline-flex items-center gap-1.5 rounded-md border border-border/70 bg-background px-2.5 py-1 text-[11.5px] text-muted-foreground hover:bg-muted/60"
									onClick={() => {
										void revealPathInFileManager(active.path).catch((e) =>
											reportError(e instanceof Error ? e.message : String(e)),
										);
									}}
								>
									Reveal in Finder
									<ArrowRight className="h-3 w-3" />
								</button>
							</div>
							<div className="flex-1 overflow-y-auto p-6">
								<div className="mb-3 text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground">
									Matched preview
								</div>
								<div className="rounded-lg border border-border/70 bg-card p-5">
									<pre className="whitespace-pre-wrap break-words font-mono text-[12.5px] leading-relaxed text-foreground/90">
										{highlight(active.preview, submitted ?? "")}
									</pre>
								</div>
							</div>
						</>
					) : (
						<div className="flex flex-1 items-center justify-center p-8 text-center">
							<div className="max-w-xs text-[12px] text-muted-foreground">
								Select a result to preview the matched chunk. Press{" "}
								<kbd className="rounded border border-border bg-background px-1 font-mono text-[10px]">
									⏎
								</kbd>{" "}
								to open it in Finder.
							</div>
						</div>
					)}
				</div>
			</div>
		</div>
	);
}

function EmptyHint({ label }: { label: string }) {
	return (
		<div className="flex h-full items-center justify-center p-8 text-center">
			<span className="text-[12px] text-muted-foreground">{label}</span>
		</div>
	);
}
