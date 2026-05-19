"use client";

import {
	Alert,
	AlertDescription,
	AlertTitle,
} from "@glean/ui/components/ui/alert";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@glean/ui/components/ui/tooltip";
import {
	Database,
	FolderOpen,
	Home,
	Info,
	Loader2,
	Search,
	Settings,
} from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { type ReactNode, useEffect, useState } from "react";
import { GleanDesktopRequired } from "@/components/glean-desktop-required";
import { GleanSearchPalette } from "@/components/glean-search-palette";
import { ThemeToggle } from "@/components/theme-toggle";
import { GleanAppProvider, useGleanApp } from "@/contexts/glean-app-context";
import { isTauri } from "@/lib/tauri";
import { cn } from "@/lib/utils";

const NAV = [
	{ href: "/", label: "Search", icon: Home, shortcut: "1" },
	{ href: "/index", label: "Index", icon: Database, shortcut: "2" },
	{ href: "/settings", label: "Settings", icon: Settings, shortcut: "3" },
	{ href: "/about", label: "About", icon: Info, shortcut: "4" },
] as const;

function basename(p: string) {
	const parts = p.split(/[\\/]/).filter(Boolean);
	return parts[parts.length - 1] ?? p;
}

function ChromeBar() {
	const { workspace, selectWorkspace, loading } = useGleanApp();
	const [paletteOpen, setPaletteOpen] = useState(false);

	useEffect(() => {
		function onKey(e: KeyboardEvent) {
			if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
				e.preventDefault();
				setPaletteOpen(true);
			}
		}
		window.addEventListener("keydown", onKey);
		return () => window.removeEventListener("keydown", onKey);
	}, []);

	return (
		<>
			<div className="relative flex h-10 shrink-0 items-center justify-between border-b border-border/60 bg-sidebar pl-3 pr-2 select-none">
				{/* Left: brand + workspace breadcrumb */}
				<div className="flex items-center gap-2 min-w-0">
					<Image src="/glean.svg" alt="Glean" width={16} height={16} />
					<span className="text-[12.5px] font-semibold tracking-tight">
						Glean
					</span>
					{workspace ? (
						<>
							<span className="text-muted-foreground/40">/</span>
							<button
								type="button"
								onClick={selectWorkspace}
								disabled={loading}
								className="group flex items-center gap-1.5 rounded px-1.5 py-0.5 text-[12px] text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
								title={workspace}
							>
								<FolderOpen className="h-3 w-3 opacity-60" />
								<span className="max-w-[220px] truncate">
									{basename(workspace)}
								</span>
							</button>
						</>
					) : (
						<button
							type="button"
							onClick={selectWorkspace}
							disabled={loading}
							className="ml-1 rounded px-1.5 py-0.5 text-[12px] text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
						>
							Open workspace…
						</button>
					)}
				</div>

				{/* Center: palette trigger */}
				<button
					type="button"
					onClick={() => setPaletteOpen(true)}
					disabled={!workspace}
					className={cn(
						"absolute left-1/2 top-1/2 flex h-7 w-[380px] -translate-x-1/2 -translate-y-1/2 items-center gap-2 rounded-md border border-border/70 bg-background/60 px-2.5 text-[12px] text-muted-foreground transition-colors",
						"hover:border-border hover:bg-background hover:text-foreground",
						"disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-background/60 disabled:hover:text-muted-foreground",
						"focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
					)}
				>
					{loading ? (
						<Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin" />
					) : (
						<Search className="h-3.5 w-3.5 shrink-0" />
					)}
					<span className="flex-1 text-left">
						{workspace
							? "Search this workspace…"
							: "Open a workspace to search"}
					</span>
					<kbd className="inline-flex items-center gap-0.5 rounded border border-border/70 bg-muted/60 px-1 py-0 text-[10px] font-medium text-muted-foreground">
						<span className="font-sans">⌘</span>K
					</kbd>
				</button>

				{/* Right: theme */}
				<div className="flex items-center gap-1">
					<ThemeToggle />
				</div>
			</div>

			<GleanSearchPalette open={paletteOpen} onOpenChange={setPaletteOpen} />
		</>
	);
}

function ActivityRail() {
	const pathname = usePathname();
	return (
		<nav
			aria-label="Primary"
			className="flex w-[52px] shrink-0 flex-col items-center justify-between border-r border-border/60 bg-sidebar py-2"
		>
			<ul className="flex flex-col items-center gap-0.5">
				{NAV.map((item) => {
					const active =
						item.href === "/"
							? pathname === "/"
							: pathname === item.href || pathname?.startsWith(`${item.href}/`);
					const Icon = item.icon;
					return (
						<li key={item.href}>
							<Tooltip>
								<TooltipTrigger asChild>
									<Link
										href={item.href}
										aria-current={active ? "page" : undefined}
										className={cn(
											"relative flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition-colors",
											"hover:bg-sidebar-accent hover:text-foreground",
											active && "bg-sidebar-accent text-foreground",
										)}
									>
										{active && (
											<span
												className="absolute -left-2 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent"
												aria-hidden
											/>
										)}
										<Icon className="h-[18px] w-[18px]" strokeWidth={1.75} />
										<span className="sr-only">{item.label}</span>
									</Link>
								</TooltipTrigger>
								<TooltipContent
									side="right"
									sideOffset={6}
									className="text-[11px]"
								>
									<span className="font-medium">{item.label}</span>
								</TooltipContent>
							</Tooltip>
						</li>
					);
				})}
			</ul>
		</nav>
	);
}

function StatusBar() {
	const { daemonOk, workspace, status, loading } = useGleanApp();
	const state = !workspace ? "idle" : daemonOk ? "ok" : "down";
	const label = !workspace
		? "No workspace"
		: daemonOk
			? "Daemon online"
			: "Daemon offline";

	return (
		<footer className="flex h-7 shrink-0 items-center justify-between border-t border-border/60 bg-sidebar px-3 text-[11px] text-muted-foreground">
			<div className="flex items-center gap-3">
				<span className="inline-flex items-center gap-1.5">
					<span className="relative flex h-1.5 w-1.5">
						<span
							className={cn(
								"absolute inline-flex h-full w-full rounded-full opacity-70",
								state === "ok" && "bg-accent glean-ring-pulse",
								state === "down" && "bg-destructive",
								state === "idle" && "bg-muted-foreground",
							)}
						/>
						<span
							className={cn(
								"relative inline-flex h-1.5 w-1.5 rounded-full",
								state === "ok" && "bg-accent",
								state === "down" && "bg-destructive",
								state === "idle" && "bg-muted-foreground",
							)}
						/>
					</span>
					<span>{label}</span>
				</span>
				{status?.rerank_enabled && status.rerank_model_ready ? (
					<span className="hidden md:inline">Reranker · ready</span>
				) : null}
			</div>
			<div className="flex items-center gap-3">
				{loading ? <span>Syncing…</span> : null}
				{status ? (
					<>
						<span className="font-mono">
							v<span className="text-foreground/80">{status.version}</span>
						</span>
						<span className="hidden md:inline">log · {status.log_level}</span>
					</>
				) : null}
			</div>
		</footer>
	);
}

function ShellChrome({ children }: { children: ReactNode }) {
	return (
		<div className="flex h-dvh w-full flex-col overflow-hidden bg-background">
			<ChromeBar />
			<GlobalErrorBar />
			<div className="flex min-h-0 flex-1">
				<ActivityRail />
				<main className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
					<div className="flex-1 overflow-y-auto">{children}</div>
					<StatusBar />
				</main>
			</div>
		</div>
	);
}

function GlobalErrorBar() {
	const { error, clearError } = useGleanApp();
	if (!error) return null;
	return (
		<div className="border-b border-border/60 px-3 py-2">
			<Alert variant="destructive" className="py-2">
				<AlertTitle className="text-sm">Error</AlertTitle>
				<AlertDescription className="flex items-start justify-between gap-2 text-sm">
					<span className="break-all">{error}</span>
					<button
						type="button"
						onClick={clearError}
						className="shrink-0 text-xs underline underline-offset-2"
					>
						Dismiss
					</button>
				</AlertDescription>
			</Alert>
		</div>
	);
}

export function GleanAppShell({ children }: { children: ReactNode }) {
	const [runtimeReady, setRuntimeReady] = useState(false);
	const [hasTauri, setHasTauri] = useState(false);

	useEffect(() => {
		queueMicrotask(() => {
			setHasTauri(isTauri());
			setRuntimeReady(true);
		});
	}, []);

	// Same UI on server and first client paint; `isTauri()` differs (window / webview), so branch only after mount.
	if (!runtimeReady) {
		return (
			<div
				className="flex min-h-dvh w-full items-center justify-center bg-background p-8"
				aria-busy="true"
			>
				<Loader2
					className="h-6 w-6 animate-spin text-muted-foreground"
					aria-hidden
				/>
				<span className="sr-only">Loading…</span>
			</div>
		);
	}

	if (!hasTauri) {
		return <GleanDesktopRequired />;
	}
	return (
		<GleanAppProvider>
			<TooltipProvider delayDuration={200}>
				<ShellChrome>{children}</ShellChrome>
			</TooltipProvider>
		</GleanAppProvider>
	);
}
