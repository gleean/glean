"use client";

import { Button } from "@glean/ui/components/ui/button";
import { ArrowRight, FolderOpen, Lock } from "lucide-react";
import { useGleanApp } from "@/contexts/glean-app-context";

export function GleanNoWorkspace() {
	const { selectWorkspace, loading } = useGleanApp();
	return (
		<div className="relative w-full max-w-md overflow-hidden rounded-xl border border-border/70 bg-card p-8 text-center">
			<div className="flex flex-col items-center gap-4">
				<div className="flex h-10 w-10 items-center justify-center rounded-lg border border-border bg-background">
					<FolderOpen className="h-4 w-4 text-accent" />
				</div>
				<div className="flex flex-col gap-1.5">
					<h2 className="text-[15px] font-semibold tracking-tight">
						Open a workspace
					</h2>
					<p className="text-[12.5px] leading-relaxed text-muted-foreground">
						Pick any folder to begin indexing. Vectors live next to your source
						— nothing crosses the network.
					</p>
				</div>
				<Button
					onClick={selectWorkspace}
					disabled={loading}
					size="sm"
					className="gap-2"
				>
					<FolderOpen className="h-3.5 w-3.5" />
					Choose folder
					<ArrowRight className="h-3.5 w-3.5" />
				</Button>
				<div className="flex items-center gap-1.5 text-[10.5px] text-muted-foreground">
					<Lock className="h-2.5 w-2.5" />
					On-device · always
				</div>
			</div>
		</div>
	);
}
