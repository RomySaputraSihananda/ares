import type { Metadata } from "next";
import "./globals.css";
import Nav from "@/components/Nav";

export const metadata: Metadata = {
  title: "ARES — Automated Trading Bot",
  description: "Live forward-test results for ARES, an M5 Momentum FVG scalper built in Rust.",
  openGraph: {
    title: "ARES Trading Bot",
    description: "Live forward-test · M5 Momentum FVG · XAUUSDm",
    type: "website",
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
        <footer className="border-t border-hl mt-section">
          <div className="max-w-5xl mx-auto px-6 py-16 grid grid-cols-1 sm:grid-cols-3 gap-10">
            <div>
              <span className="font-mono text-xs tracking-widest text-ink-sub uppercase">ARES</span>
              <p className="text-sm text-ink-sub mt-3 leading-relaxed">
                M5 Momentum FVG Scalper<br />Built in Rust · XAUUSDm
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-ink-sub uppercase tracking-eyebrow mb-4">Navigation</p>
              <ul className="space-y-2 text-sm text-ink-sub">
                <li><a href="/"         className="hover:text-ink transition-colors">Dashboard</a></li>
                <li><a href="/trades"   className="hover:text-ink transition-colors">Trades</a></li>
                <li><a href="/backtest" className="hover:text-ink transition-colors">Backtest</a></li>
              </ul>
            </div>
            <div>
              <p className="text-xs font-medium text-ink-sub uppercase tracking-eyebrow mb-4">Disclaimer</p>
              <p className="text-xs text-ink-ter leading-relaxed">
                Past performance does not guarantee future results. Demo account forward test. Not financial advice.
              </p>
            </div>
          </div>
        </footer>
      </body>
    </html>
  );
}
