import type { Metadata } from "next";
import { ThemeProvider } from "@/components/contexts/theme-provider";
import { Navbar } from "@/components/navbar";
import { Space_Mono, Space_Grotesk } from "next/font/google";
import { Footer } from "@/components/footer";
import "@/styles/globals.css";

import { GoogleAnalytics } from "@next/third-parties/google";
import { Analytics } from "@vercel/analytics/react";

const GeistSans = Space_Grotesk({
  subsets: ["latin"],
  variable: "--font-geist-sans",
  display: "swap",
  weight: "400",
});

const GeistMono = Space_Mono({
  subsets: ["latin"],
  variable: "--font-geist-mono",
  display: "swap",
  weight: "400",
});

export const metadata: Metadata = {
  title: "Mega - Redefining Open Source with Monolithic and Decentralized",
  metadataBase: new URL("https://gitmega.dev/"),
  description:
    "Mega is an unofficial open source implementation of Google Piper. It is a monorepo & monolithic codebase management system that supports Git. Mega is designed to manage large-scale codebases, streamline development, and foster collaboration.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`${GeistSans.variable} ${GeistMono.variable} font-regular antialiased tracking-wide`}
        suppressHydrationWarning
      >
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <Navbar />
          <main className="sm:container mx-auto w-[90vw] h-auto scroll-smooth">
            {children}
          </main>
          <Footer />
        </ThemeProvider>
        <Analytics />
      </body>

      <GoogleAnalytics gaId="G-WCSCZGFL72" />
    </html>
  );
}
