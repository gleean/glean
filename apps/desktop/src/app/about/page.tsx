"use client";

import { ArrowUpRight, Box, Cpu, Lock, Terminal } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import type { ReactNode } from "react";
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

function InfoCard({
	icon: Icon,
	title,
	children,
}: {
	icon: React.ElementType;
	title: string;
	children: ReactNode;
}) {
	return (
		<div className="flex flex-col gap-2 rounded-lg border border-border/70 bg-card p-4">
			<div className="flex h-7 w-7 items-center justify-center rounded-md border border-border bg-background">
				<Icon className="h-3.5 w-3.5 text-accent" />
			</div>
			<h3 className="text-[12.5px] font-semibold tracking-tight">{title}</h3>
			<div className="text-[11.5px] leading-relaxed text-muted-foreground">
				{children}
			</div>
		</div>
	);
}

export default function AboutPage() {
	const { status } = useGleanApp();
	const version = status?.version ?? "0.1.0";

	return (
		<div className="flex flex-col">
			<PageHeader title="About" />

			<div className="flex flex-col gap-6 px-6 py-5">
				{/* Identity card */}
				<section className="flex items-start justify-between gap-4 rounded-lg border border-border/70 bg-card p-5">
					<div className="flex items-center gap-3">
						<Image src="/glean.svg" alt="Glean" width={16} height={16} />
						<div className="flex flex-col">
							<div className="text-[14px] font-semibold tracking-tight">
								Glean Desktop
							</div>
							<div className="font-mono text-[10.5px] text-muted-foreground">
								v{version} · local-first
							</div>
						</div>
					</div>
					<div className="flex flex-wrap items-center gap-1.5">
						<Link
							href="https://github.com"
							target="_blank"
							rel="noreferrer"
							className="inline-flex items-center gap-1 rounded-md border border-border bg-background px-2 py-1 text-[11px] font-medium transition-colors hover:border-accent/40 hover:bg-accent-soft"
						>
							Source
							<ArrowUpRight className="h-3 w-3" />
						</Link>
						<Link
							href="/index"
							className="inline-flex items-center gap-1 rounded-md border border-border bg-background px-2 py-1 text-[11px] font-medium transition-colors hover:border-accent/40 hover:bg-accent-soft"
						>
							Index
							<ArrowUpRight className="h-3 w-3" />
						</Link>
					</div>
				</section>

				{/* Description */}
				<section className="rounded-lg border border-border/70 bg-card p-5">
					<p className="max-w-2xl text-[13px] leading-relaxed text-muted-foreground">
						Glean watches the folders you care about, builds a private index
						next to the source, and serves up semantic search in milliseconds.
						No accounts. No cloud. No telemetry.
					</p>
				</section>

				{/* Architecture */}
				<section>
					<div className="mb-2 px-1 text-[10.5px] font-semibold uppercase tracking-wider text-muted-foreground">
						Architecture
					</div>
					<div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
						<InfoCard icon={Lock} title="Global storage">
							<code className="font-mono text-[11px]">~/.glean</code> — config,
							reranker cache, logs.
						</InfoCard>
						<InfoCard icon={Box} title="Workspace index">
							<code className="font-mono text-[11px]">
								&lt;workspace&gt;/.glean/
							</code>{" "}
							— SQLite + vectors.
						</InfoCard>
						<InfoCard icon={Cpu} title="Sidecar daemon">
							Single-writer <code className="font-mono text-[11px]">glean</code>{" "}
							binary watches the workspace.
						</InfoCard>
						<InfoCard icon={Terminal} title="MCP integration">
							Editor agents launch{" "}
							<code className="font-mono text-[11px]">glean mcp</code>.
						</InfoCard>
					</div>
				</section>

				{/* Stack */}
				<section className="rounded-lg border border-border/70 bg-card p-5">
					<div className="mb-3 text-[10.5px] font-semibold uppercase tracking-wider text-muted-foreground">
						Stack
					</div>
					<ul className="grid grid-cols-1 gap-2 text-[12px] sm:grid-cols-2">
						<li className="flex items-baseline gap-2">
							<span className="w-16 shrink-0 font-medium text-foreground">
								UI
							</span>
							<span className="text-muted-foreground">
								Next.js + Tailwind in Tauri WebView.
							</span>
						</li>
						<li className="flex items-baseline gap-2">
							<span className="w-16 shrink-0 font-medium text-foreground">
								Engine
							</span>
							<span className="text-muted-foreground">
								Rust <code className="font-mono">glean-core</code> · read-only
								host.
							</span>
						</li>
						<li className="flex items-baseline gap-2">
							<span className="w-16 shrink-0 font-medium text-foreground">
								Storage
							</span>
							<span className="text-muted-foreground">
								SQLite + LanceDB. Inspectable.
							</span>
						</li>
						<li className="flex items-baseline gap-2">
							<span className="w-16 shrink-0 font-medium text-foreground">
								Models
							</span>
							<span className="text-muted-foreground">
								On-device embedding + ONNX reranker.
							</span>
						</li>
					</ul>
				</section>
			</div>
		</div>
	);
}
