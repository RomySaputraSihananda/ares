import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Backtest Results",
  description: "ARES backtest results: M5 Momentum FVG scalper on XAUUSDm. Best run: PF 1.42, +43.2% net return, −11.5% max drawdown.",
};

const results = [
  { period: "1 Month",   tf: "M5", risk: "1%", trades: 159, wr: 55.3, pf: 1.42, ret: 43.2,  dd: -11.5, highlight: true },
  { period: "1 Month",   tf: "M5", risk: "5%", trades: 159, wr: 55.3, pf: 1.26, ret: 390,   dd: -156,  highlight: false },
  { period: "1 Week",    tf: "M5", risk: "1%", trades: 34,  wr: 47.1, pf: 0.94, ret: -6.2,  dd: -8.1,  highlight: false },
  { period: "Yesterday", tf: "M1", risk: "1%", trades: 36,  wr: 47.2, pf: 1.05, ret: 6.6,   dd: -51,   highlight: false },
  { period: "Yesterday", tf: "M5", risk: "1%", trades: 3,   wr: 66.7, pf: null, ret: null,  dd: null,  highlight: false, note: "Too few trades" },
];

const params = [
  ["Timeframe",      "M5"],
  ["Symbol",         "XAUUSDm"],
  ["EMA Period",     "20"],
  ["Min FVG Pips",   "3"],
  ["Min SL Pips",    "5"],
  ["Min RR",         "1.5×"],
  ["FVG Expiry",     "10 candles"],
  ["Body PCT Min",   "60%"],
  ["Close PCT Min",  "80%"],
];

export default function BacktestPage() {
  return (
    <>
      <section className="pt-6 pb-12">
        <p className="eyebrow mb-4">Quantitative Analysis</p>
        <h1 className="text-4xl font-semibold tracking-[-0.032em] text-ink mb-3">
          Backtest Results
        </h1>
        <p className="text-[15px] text-ink-sub max-w-xl">
          Historical simulation on real MT5 tick data. Includes spread costs, commission, and slippage.
        </p>
      </section>

      {/* Highlight */}
      <div className="card-featured p-10 mb-10">
        <div className="flex flex-col sm:flex-row sm:items-start gap-4 mb-8">
          <div>
            <span className="status-pill status-bull mb-3 inline-block">Recommended</span>
            <h2 className="text-2xl font-semibold tracking-tight-sm text-ink">M5 · 1 Month · 1% Risk</h2>
            <p className="text-sm text-ink-sub mt-1">XAUUSDm · Exness Demo</p>
          </div>
          <Link href="/trades" className="btn-primary sm:ml-auto shrink-0">View Live Trades →</Link>
        </div>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-6 pt-8 border-t border-hl">
          {[
            { label: "Profit Factor", value: "1.42",  note: "> 1.3 = good" },
            { label: "Net Return",    value: "+43.2%", note: "on $600 balance" },
            { label: "Max Drawdown",  value: "−11.5%", note: "manageable" },
            { label: "Win Rate",      value: "55.3%",  note: "159 trades" },
          ].map(({ label, value, note }) => (
            <div key={label}>
              <p className="text-xs text-ink-sub mb-1.5">{label}</p>
              <p className="font-mono text-xl font-semibold text-ink">{value}</p>
              <p className="text-xs text-ink-ter mt-1">{note}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Results table */}
      <div className="rounded-lg border border-hl overflow-hidden mb-10">
        <div className="px-6 py-4 border-b border-hl">
          <p className="eyebrow">All Runs</p>
        </div>
        <div className="overflow-x-auto">
          <table className="data-table">
            <thead>
              <tr>
                {["Period", "TF", "Risk", "Trades", "Win Rate", "Profit Factor", "Return", "Max DD", ""].map(h => (
                  <th key={h}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {results.map((r, i) => (
                <tr key={i} className={r.highlight ? "bg-s2" : ""}>
                  <td className="font-medium text-ink">{r.period}</td>
                  <td className="font-mono text-ink-sub">{r.tf}</td>
                  <td className="font-mono text-ink-sub">{r.risk}</td>
                  <td className="font-mono text-ink-md">{r.trades}</td>
                  <td className="font-mono text-ink-md">{r.wr.toFixed(1)}%</td>
                  <td className="font-mono font-medium">
                    {r.pf != null
                      ? <span className={r.pf >= 1.3 ? "text-bull" : r.pf < 1 ? "text-bear" : "text-ink-md"}>{r.pf.toFixed(2)}</span>
                      : <span className="text-ink-ter">—</span>}
                  </td>
                  <td className="font-mono font-medium">
                    {r.ret != null
                      ? <span className={r.ret >= 0 ? "text-bull" : "text-bear"}>{r.ret >= 0 ? "+" : ""}{r.ret.toFixed(1)}%</span>
                      : <span className="text-ink-ter">—</span>}
                  </td>
                  <td className="font-mono">
                    {r.dd != null
                      ? <span className={Math.abs(r.dd) > 30 ? "text-bear" : Math.abs(r.dd) > 15 ? "text-amber-400" : "text-ink-md"}>{r.dd.toFixed(1)}%</span>
                      : <span className="text-ink-ter">—</span>}
                  </td>
                  <td className="text-xs text-ink-ter">{r.note ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Params + How it works */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-6 mb-10">
        <div className="card">
          <p className="eyebrow mb-6">Parameters</p>
          <dl className="divide-y divide-hl">
            {params.map(([k, v]) => (
              <div key={k} className="flex justify-between py-3">
                <dt className="text-sm text-ink-sub">{k}</dt>
                <dd className="font-mono text-sm font-medium text-ink">{v}</dd>
              </div>
            ))}
          </dl>
        </div>

        <div className="card">
          <p className="eyebrow mb-6">How It Works</p>
          <ol className="space-y-5">
            {[
              ["Detect Impulse", "3-candle momentum: body ≥ 60% of range, close in top/bottom 20%."],
              ["Find FVG",       "Measure gap between candle 1 high and candle 3 low (bull) or reverse."],
              ["EMA Filter",     "Long only above EMA-20, short only below. Strict trend confirmation."],
              ["Place Limit",    "Set limit at FVG midpoint. Auto-cancel after 10 candles if unfilled."],
              ["Manage Risk",    "SL at FVG boundary, TP at ≥ 1.5× RR, size at exactly 1% risk."],
            ].map(([title, desc], i) => (
              <li key={i} className="flex gap-4">
                <span className="flex-shrink-0 w-6 h-6 rounded-full bg-s2 border border-hl text-xs font-mono text-ink-sub flex items-center justify-center">
                  {i + 1}
                </span>
                <div>
                  <p className="text-sm font-medium text-ink">{title}</p>
                  <p className="text-sm text-ink-sub mt-0.5 leading-relaxed">{desc}</p>
                </div>
              </li>
            ))}
          </ol>
        </div>
      </div>

      {/* Disclaimer */}
      <div className="rounded-lg border border-hl p-5">
        <p className="text-xs text-ink-ter leading-relaxed">
          <span className="text-ink-sub font-medium">Disclaimer — </span>
          Backtests simulate on historical data and do not account for requotes, broker restrictions, or changing market regimes.
          Past results do not guarantee future performance. For informational purposes only.
        </p>
      </div>
    </>
  );
}
