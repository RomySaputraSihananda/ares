import type { Metadata } from "next";
import "./globals.css";
import Nav from "@/components/Nav";

const SITE_URL = "https://ares-six-peach.vercel.app";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: "ARES — Automated Gold Trading Bot",
    template: "%s · ARES",
  },
  description:
    "Live forward-test results for ARES, an M5 Momentum FVG scalper built in Rust. Trades XAUUSDm with EMA-20 trend filter and automated risk management.",
  keywords: [
    "algorithmic trading", "gold trading bot", "XAUUSD EA",
    "MT5 expert advisor", "Rust trading bot", "FVG scalper",
    "forex robot", "automated trading", "ICT strategy",
  ],
  authors: [{ name: "Romy Saputra Sihananda" }],
  creator: "Romy Saputra Sihananda",
  robots: {
    index: true,
    follow: true,
    googleBot: { index: true, follow: true, "max-snippet": -1, "max-image-preview": "large" },
  },
  openGraph: {
    type: "website",
    url: SITE_URL,
    siteName: "ARES Trading Bot",
    title: "ARES — Automated Gold Trading Bot",
    description: "Live forward-test · M5 Momentum FVG scalper · XAUUSDm · Built in Rust",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: "ARES — Automated Gold Trading Bot",
    description: "Live forward-test · M5 Momentum FVG scalper · XAUUSDm · Built in Rust",
  },
  icons: {
    icon: "/favicon.svg",
    shortcut: "/favicon.svg",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="min-h-screen antialiased">
        <Nav />
        <main className="max-w-5xl mx-auto px-4 sm:px-6 py-12">
          {children}
        </main>
        <footer style={{ borderTop: "1px solid var(--c-hl)" }} className="mt-24">
          <div className="max-w-5xl mx-auto px-6 py-16 grid grid-cols-1 sm:grid-cols-3 gap-10">
            <div>
              <div className="flex items-center gap-2 mb-3">
                <svg width="22" height="22" viewBox="0 0 26 26" fill="none" aria-hidden="true">
                  <rect width="26" height="26" rx="6" fill="#5e6ad2"/>
                  <path d="M8 19 L13 8 L18 19" stroke="white" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
                  <path d="M10.2 15.5 H15.8" stroke="white" strokeWidth="1.8" strokeLinecap="round"/>
                </svg>
                <span className="font-mono text-sm font-semibold" style={{ letterSpacing: "0.18em", color: "var(--c-ink)" }}>ARES</span>
              </div>
              <p className="text-sm leading-relaxed" style={{ color: "var(--c-ink-sub)" }}>
                M5 Momentum FVG Scalper<br />Built in Rust · XAUUSDm
              </p>
            </div>
            <div>
              <p className="eyebrow mb-4">Navigation</p>
              <ul className="space-y-2 text-sm" style={{ color: "var(--c-ink-sub)" }}>
                <li><a href="/"         className="hover:text-ink transition-colors">Dashboard</a></li>
                <li><a href="/trades"   className="hover:text-ink transition-colors">Trades</a></li>
                <li><a href="/backtest" className="hover:text-ink transition-colors">Backtest</a></li>
              </ul>
            </div>
            <div>
              <p className="eyebrow mb-4">Disclaimer</p>
              <p className="text-xs leading-relaxed" style={{ color: "var(--c-ink-ter)" }}>
                Past performance does not guarantee future results. Demo account forward test. Not financial advice.
              </p>
            </div>
          </div>
        </footer>
      </body>
    </html>
  );
}
