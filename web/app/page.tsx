import Link from "next/link";
import LiveDashboard from "@/components/LiveDashboard";

export default function DashboardPage() {
  return (
    <>
      {/* Hero */}
      <section className="pt-6 pb-16">
        <p className="eyebrow mb-5">Live Forward Test · Demo Account</p>
        <h1 className="text-5xl font-semibold tracking-[-0.032em] text-ink leading-[1.10] mb-5">
          ARES Trading Bot
        </h1>
        <p className="text-[18px] text-ink-md leading-relaxed max-w-2xl mb-8">
          Open-source algorithmic trading bot built in Rust. Momentum FVG scalper
          with ATR quality filter, EMA trend filter, configurable session window,
          automatic position sizing, and Telegram alerts — runs on any MT5 symbol.
        </p>
        <div className="flex gap-3 flex-wrap">
          <Link href="/trades"   className="btn-primary">View Trades</Link>
          <Link href="/backtest" className="btn-secondary">Backtest Results</Link>
        </div>
      </section>

      {/* Live data — client-side polling */}
      <LiveDashboard />

      {/* Strategy */}
      <section className="mb-16">
        <p className="eyebrow mb-6">Strategy</p>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <FeatureCard
            icon="◈"
            title="Momentum FVG"
            body="3-candle pattern: pre → impulse → post. Body ≥ 50% of range, close in top/bottom 20%."
          />
          <FeatureCard
            icon="⬡"
            title="ATR Quality Gate"
            body="FVG zone ≥ 0.3× ATR(14) and impulse body ≥ 1.0× ATR(14). Weak setups are discarded before entry."
          />
          <FeatureCard
            icon="◎"
            title="EMA-20 + Zone 75%"
            body="Long above EMA-20, short below. Limit order placed 75% deep into the FVG for better confirmation."
          />
          <FeatureCard
            icon="◉"
            title="Risk Management"
            body="Fixed % risk per trade. Min 1.5× RR. SL at FVG boundary. Session restricted to 08–13 UTC."
          />
        </div>
      </section>

      {/* Backtest CTA */}
      <section className="card-featured p-10 mb-16">
        <div className="flex flex-col sm:flex-row sm:items-center gap-6 mb-8">
          <div>
            <p className="eyebrow mb-2">Backtest Highlight · v1.4.0</p>
            <h2 className="text-2xl font-semibold tracking-tight-sm text-ink">
              M5 · 50k Bars · 5% Risk
            </h2>
            <p className="text-sm text-ink-sub mt-1">XAUUSDm · Sep 2025 – Jun 2026 · 83 trades</p>
          </div>
          <Link href="/backtest" className="btn-primary sm:ml-auto shrink-0">
            Full Results →
          </Link>
        </div>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-6 pt-6 border-t border-hl">
          {[
            { label: "Profit Factor", value: "2.08",    sub: "vs 1.17 in v1.3.0" },
            { label: "Net Return",    value: "+448.7%",  sub: "$5k → $27,439"     },
            { label: "Win Rate",      value: "62.7%",    sub: "vs 50% in v1.3.0"  },
            { label: "Max Drawdown",  value: "−$2,669",  sub: "vs −$5,482 in v1.3.0" },
          ].map(({ label, value, sub }) => (
            <div key={label}>
              <p className="text-xs text-ink-sub mb-1.5">{label}</p>
              <p className="font-mono text-xl font-semibold tracking-tight-md">{value}</p>
              <p className="text-xs text-ink-ter mt-1">{sub}</p>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}

function FeatureCard({ icon, title, body }: { icon: string; title: string; body: string }) {
  return (
    <div className="card">
      <span className="text-accent text-lg mb-4 block">{icon}</span>
      <p className="font-medium text-[15px] text-ink mb-2 tracking-tight">{title}</p>
      <p className="text-sm text-ink-sub leading-relaxed">{body}</p>
    </div>
  );
}
