"use client";

import { Button } from "@glean/ui/components/ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@glean/ui/components/ui/tooltip";
import { Check, Copy } from "lucide-react";
import { useState } from "react";

export function CopyButton({
	value,
	label = "Copy path",
}: {
	value: string;
	label?: string;
}) {
	const [copied, setCopied] = useState(false);

	const onCopy = async () => {
		try {
			await navigator.clipboard.writeText(value);
			setCopied(true);
			setTimeout(() => setCopied(false), 1500);
		} catch {
			// ignore
		}
	};

	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<Button
					variant="ghost"
					size="icon"
					className="h-7 w-7 shrink-0"
					onClick={onCopy}
					aria-label={label}
				>
					{copied ? (
						<Check className="h-3.5 w-3.5" />
					) : (
						<Copy className="h-3.5 w-3.5" />
					)}
				</Button>
			</TooltipTrigger>
			<TooltipContent>{copied ? "Copied" : label}</TooltipContent>
		</Tooltip>
	);
}
