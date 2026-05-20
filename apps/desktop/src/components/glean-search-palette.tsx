"use client";

import {
	Dialog,
	DialogContent,
	DialogTitle,
} from "@glean/ui/components/ui/dialog";
import { CornerDownLeft, FileText, Loader2, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useGleanApp } from "@/contexts/glean-app-context";
import { revealPathInFileManager, semanticSearch } from "@/lib/tauri";
import type { SearchHit } from "@/lib/types";
import { cn } from "@/lib/utils";

function basename(p: string) {
	const parts = p.split(/[\\/]/).filter(Boolean);
	return parts[parts.length - 1] ?? p;
}

export function GleanSearchPalette({
	open,
	onOpenChange,
}: {
	open: boolean;
	onOpenChange: (v: boolean) => void;
}) {
	const { workspace, reportError } = useGleanApp();
	const [query, setQuery] = useState("");
	const [hits, setHits] = useState<SearchHit[]>([]);
	const [loading, setLoading] = useState(false);
	const [active, setActive] = useState(0);
	const inputRef = useRef<HTMLInputElement>(null);

	// reset on open
	useEffect(() => {
		if (open) {
			setQuery("");
			setHits([]);
			setActive(0);
			requestAnimationFrame(() => inputRef.current?.focus());
		}
	}, [open]);

	// debounced search
	useEffect(() => {
		if (!open || !workspace) return;
		const q = query.trim();
		if (!q) {
			setHits([]);
			setLoading(false);
			return;
		}
		setLoading(true);
		const t = setTimeout(async () => {
			try {
				const r = await semanticSearch(q, 24);
				setHits(r);
				setActive(0);
			} catch {
				setHits([]);
			} finally {
				setLoading(false);
			}
		}, 120);
		return () => clearTimeout(t);
	}, [query, workspace, open]);

	// keyboard nav
	useEffect(() => {
		if (!open) return;
		function onKey(e: KeyboardEvent) {
			if (e.key === "ArrowDown") {
				e.preventDefault();
				setActive((i) => Math.min(i + 1, Math.max(0, hits.length - 1)));
			} else if (e.key === "ArrowUp") {
				e.preventDefault();
				setActive((i) => Math.max(0, i - 1));
			} else if (e.key === "Enter" && hits[active]) {
				e.preventDefault();
				void revealPathInFileManager(hits[active].path).catch((err) =>
					reportError(err instanceof Error ? err.message : String(err)),
				);
				onOpenChange(false);
			}
		}
		window.addEventListener("keydown", onKey);
		return () => window.removeEventListener("keydown", onKey);
	}, [open, hits, active, onOpenChange, reportError]);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				showCloseButton={false}
				className="top-[18%] translate-y-0 max-w-2xl gap-0 overflow-hidden p-0 border-border/80 shadow-2xl"
			>
				<DialogTitle className="sr-only">Search workspace</DialogTitle>

				<div className="flex items-center gap-3 border-b border-border/70 px-4 py-3">
					{loading ? (
						<Loader2 className="h-4 w-4 shrink-0 animate-spin text-muted-foreground" />
					) : (
						<Search className="h-4 w-4 shrink-0 text-muted-foreground" />
					)}
					<input
						ref={inputRef}
						value={query}
						onChange={(e) => setQuery(e.target.value)}
						placeholder={
							workspace
								? "Search across this workspace…"
								: "Open a workspace first"
						}
						disabled={!workspace}
						className="min-w-0 flex-1 bg-transparent text-[14px] outline-none placeholder:text-muted-foreground"
					/>
					<kbd className="inline-flex items-center gap-1 rounded border border-border bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
						esc
					</kbd>
				</div>

				<div className="max-h-[420px] overflow-y-auto p-1.5">
					{!workspace ? (
						<div className="px-4 py-12 text-center text-[12px] text-muted-foreground">
							No workspace selected.
						</div>
					) : !query ? (
						<div className="px-4 py-12 text-center text-[12px] text-muted-foreground">
							Start typing to search semantically across files.
						</div>
					) : hits.length === 0 && !loading ? (
						<div className="px-4 py-12 text-center text-[12px] text-muted-foreground">
							No matches for{" "}
							<span className="font-mono text-foreground/80">
								&quot;{query}&quot;
							</span>
						</div>
					) : (
						<ul className="flex flex-col">
							{hits.map((h, i) => (
								<li key={`${h.path}-${i}`}>
									<button
										type="button"
										onMouseEnter={() => setActive(i)}
										onClick={() => onOpenChange(false)}
										className={cn(
											"flex w-full items-start gap-3 rounded-md px-3 py-2.5 text-left transition-colors",
											i === active ? "bg-sidebar-accent" : "hover:bg-muted/60",
										)}
									>
										<FileText className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
										<div className="min-w-0 flex-1">
											<div className="flex items-center gap-2">
												<span className="truncate text-[13px] font-medium">
													{basename(h.path)}
												</span>
												<span className="truncate font-mono text-[10.5px] text-muted-foreground">
													{h.path}
												</span>
											</div>
											<p className="mt-0.5 line-clamp-1 text-[12px] text-muted-foreground">
												{h.preview}
											</p>
										</div>
									</button>
								</li>
							))}
						</ul>
					)}
				</div>

				<div className="flex items-center justify-between gap-3 border-t border-border/70 bg-sidebar/60 px-3 py-2 text-[11px] text-muted-foreground">
					<div className="flex items-center gap-3">
						<span className="inline-flex items-center gap-1">
							<kbd className="rounded border border-border bg-background px-1 text-[10px]">
								↑
							</kbd>
							<kbd className="rounded border border-border bg-background px-1 text-[10px]">
								↓
							</kbd>
							navigate
						</span>
						<span className="inline-flex items-center gap-1">
							<kbd className="inline-flex items-center rounded border border-border bg-background px-1 text-[10px]">
								<CornerDownLeft className="h-2.5 w-2.5" />
							</kbd>
							open
						</span>
					</div>
					<span className="font-mono">
						{hits.length ? `${hits.length} results` : ""}
					</span>
				</div>
			</DialogContent>
		</Dialog>
	);
}
