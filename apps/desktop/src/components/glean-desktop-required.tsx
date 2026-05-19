import {
	Alert,
	AlertDescription,
	AlertTitle,
} from "@glean/ui/components/ui/alert";

export function GleanDesktopRequired() {
	return (
		<div className="flex min-h-dvh items-center justify-center p-8">
			<Alert className="max-w-md">
				<AlertTitle>Desktop shell required</AlertTitle>
				<AlertDescription>
					Run{" "}
					<code className="font-mono text-sm">
						pnpm --filter @glean/desktop tauri dev
					</code>{" "}
					from the repository root. Browser-only{" "}
					<code className="font-mono text-sm">pnpm dev</code> does not connect
					to the local Glean engine.
				</AlertDescription>
			</Alert>
		</div>
	);
}
