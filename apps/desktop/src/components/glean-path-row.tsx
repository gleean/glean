"use client";

import type { ReactNode } from "react";
import { CopyButton } from "./copy-button";

export function GleanPathRow({
	label,
	value,
	badge,
}: {
	label: string;
	value: string;
	badge?: ReactNode;
}) {
	return (
		<div className="grid grid-cols-1 gap-1 border-b py-3 last:border-b-0 sm:grid-cols-[140px_1fr] sm:gap-4">
			<div className="flex items-center gap-2 text-xs font-medium text-muted-foreground sm:text-sm">
				{label}
				{badge}
			</div>
			<div className="flex items-start gap-2">
				<code className="min-w-0 flex-1 break-all rounded bg-muted px-2 py-1 font-mono text-xs text-foreground">
					{value || "—"}
				</code>
				{value ? (
					<CopyButton value={value} label={`Copy ${label.toLowerCase()}`} />
				) : null}
			</div>
		</div>
	);
}
