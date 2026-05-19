import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";

import { GleanAppShell } from "@/components/glean-app-shell";
import { ThemeProvider } from "@/components/theme-provider";

import "./globals.css";

const geist = Geist({ subsets: ["latin"], variable: "--font-geist-sans" });
const geistMono = Geist_Mono({
	subsets: ["latin"],
	variable: "--font-geist-mono",
});

export const metadata: Metadata = {
	title: "Glean",
	description:
		"Local-first knowledge engine. Indexing via sidecar glean daemon.",
};

export default function RootLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<html
			lang="en"
			className={`${geist.variable} ${geistMono.variable} bg-background`}
			suppressHydrationWarning
		>
			<body className="font-sans antialiased">
				<ThemeProvider
					attribute="class"
					defaultTheme="system"
					enableSystem
					disableTransitionOnChange
				>
					<GleanAppShell>{children}</GleanAppShell>
				</ThemeProvider>
			</body>
		</html>
	);
}
