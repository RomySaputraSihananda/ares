import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Backtest Results",
  description: "ARES v1.4.0 backtest results: M5 Momentum FVG scalper on XAUUSDm. ATR quality filter, EMA-20 trend filter, entry zone 75%, London session.",
};

const results: Array<{
  symbol: string; period: string; tf: string; risk: string; version: string;
  trades: number | null; wr: number; pf: number | null; ret: number | null; dd: number | null;
  highlight: boolean; note?: string;
}> = [
  { symbol: "XAUUSDm", period: "50k bars (~8 mo)", tf: "M5", risk: "5%", version: "v1.4.0", trades: 83,  wr: 62.7, pf: 2.08, ret: 448.7, dd: null, highlight: true,  note: "Session 08–13 UTC" },
  { symbol: "XAUUSDm", period: "2025 Full Year",   tf: "M5", risk: "5%", version: "v1.4.0", trades: 26,  wr: 65.4, pf: 2.08, ret: 78.8,  dd: null, highlight: false, note: "Session 08–13 UTC" },
  { symbol: "XAUUSDm", period: "50k bars (~8 mo)", tf: "M5", risk: "1%", version: "v1.4.0", trades: 59,  wr: 61.0, pf: 1.93, ret: 25.4,  dd: -4.8, highlight: false, note: "Session 08–13 UTC" },
  { symbol: "XAUUSDm", period: "50k bars (~8 mo)", tf: "M5", risk: "5%", version: "v1.3.0", trades: 224, wr: 50.0, pf: 1.17, ret: 174.7, dd: null, highlight: false, note: "No ATR filter" },
];

const params = [
  ["Timeframe",         "M5"],
  ["Symbol",            "XAUUSDm"],
  ["Session",           "08:00–13:00 UTC"],
  ["EMA Period",        "20"],
  ["ATR FVG Filter",    "≥ 0.3× ATR(14)"],
  ["ATR Impulse Filter","≥ 1.0× ATR(14)"],
  ["Entry Zone",        "75% into FVG"],
  ["Min RR",            "1.5×"],
  ["FVG Expiry",        "10 candles"],
  ["Body PCT Min",      "50%"],
  ["Close PCT Min",     "80%"],
];

// Simplified balance milestones for equity progression (50k bars, 5% risk, v1.4.0)
// $5,000 start → $27,439 end
const equityPoints = [
  { label: "Start",    value: 5000  },
  { label: "Oct '25",  value: 4953  },
  { label: "Nov '25",  value: 5173  },
  { label: "Dec '25",  value: 5901  },
  { label: "Jan '26",  value: 6902  },
  { label: "Feb '26",  value: 8240  },
  { label: "Mar '26",  value: 12440 },
  { label: "Apr '26",  value: 15600 },
  { label: "May '26",  value: 16038 },
  { label: "Jun '26",  value: 27439 },
];

export default function BacktestPage() {
  const maxEquity = Math.max(...equityPoints.map(p => p.value));
  const minEquity = Math.min(...equityPoints.map(p => p.value));

  // Normalise to SVG viewBox height 80px
  const h = 80;
  const w = 340;
  const pts = equityPoints.map((p, i) => {
    const x = (i / (equityPoints.length - 1)) * w;
    const y = h - ((p.value - minEquity) / (maxEquity - minEquity)) * h;
    return `${x},${y}`;
  }).join(" ");

  return (
    <>
      <section className="pt-6 pb-12">
        <p className="eyebrow mb-4">Quantitative Analysis</p>
        <h1 className="text-4xl font-semibold tracking-[-0.032em] text-ink mb-3">
          Backtest Results
        </h1>
        <p className="text-[15px] text-ink-sub max-w-xl">
          Historical simulation on real MT5 tick data. Includes spread, commission ($7/lot), and slippage.
          London session filter (08–13 UTC) applied.
        </p>
      </section>

      {/* Highlight */}
      <div className="card-featured p-10 mb-6">
        <div className="flex flex-col sm:flex-row sm:items-start gap-4 mb-8">
          <div>
            <span className="status-pill status-bull mb-3 inline-block">v1.4.0 · Best Result</span>
            <h2 className="text-2xl font-semibold tracking-tight-sm text-ink">M5 · 50k Bars · 5% Risk</h2>
            <p className="text-sm text-ink-sub mt-1">XAUUSDm · Sep 2025 – Jun 2026</p>
          </div>
          <Link href="/trades" className="btn-primary sm:ml-auto shrink-0">View Live Trades →</Link>
        </div>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-6 pt-8 border-t border-hl">
          {[
            { label: "Profit Factor", value: "2.08",    note: "> 2.0 = strong"     },
            { label: "Net Return",    value: "+448.7%",  note: "$5,000 → $27,439"   },
            { label: "Win Rate",      value: "62.7%",    note: "83 trades"          },
            { label: "vs v1.3.0",     value: "+274pp",   note: "v1.3.0 was +174.7%" },
          ].map(({ label, value, note }) => (
            <div key={label}>
              <p className="text-xs text-ink-sub mb-1.5">{label}</p>
              <p className="font-mono text-xl font-semibold text-ink">{value}</p>
              <p className="text-xs text-ink-ter mt-1">{note}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Equity curve */}
      <div className="card mb-10 p-6">
        <div className="flex items-center justify-between mb-5">
          <div>
            <p className="eyebrow mb-1">Equity Progression</p>
            <p className="text-xs text-ink-ter">50k M5 bars · 5% risk · XAUUSDm</p>
          </div>
          <div className="text-right">
            <p className="font-mono text-lg font-semibold text-bull">$27,439</p>
            <p className="text-xs text-ink-ter">from $5,000</p>
          </div>
        </div>
        <div className="relative w-full overflow-x-auto">
          <svg viewBox={`0 0 ${w} ${h + 20}`} className="w-full" style={{ minWidth: 280 }}>
            {/* Grid lines */}
            {[0, 0.25, 0.5, 0.75, 1].map((t) => {
              const y = h - t * h;
              const val = Math.round(minEquity + t * (maxEquity - minEquity));
              return (
                <g key={t}>
                  <line x1="0" y1={y} x2={w} y2={y} stroke="#23252a" strokeWidth="0.5" />
                  <text x={w + 2} y={y + 4} fontSize="7" fill="#62666d" textAnchor="start">
                    ${(val / 1000).toFixed(0)}k
                  </text>
                </g>
              );
            })}
            {/* Area fill */}
            <defs>
              <linearGradient id="areaGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%"   stopColor="#27a644" stopOpacity="0.18" />
                <stop offset="100%" stopColor="#27a644" stopOpacity="0.01" />
              </linearGradient>
            </defs>
            <polygon
              points={`0,${h} ${pts} ${w},${h}`}
              fill="url(#areaGrad)"
            />
            {/* Line */}
            <polyline
              points={pts}
              fill="none"
              stroke="#27a644"
              strokeWidth="1.5"
              strokeLinejoin="round"
              strokeLinecap="round"
            />
            {/* Dots + labels */}
            {equityPoints.map((p, i) => {
              const x = (i / (equityPoints.length - 1)) * w;
              const y = h - ((p.value - minEquity) / (maxEquity - minEquity)) * h;
              return (
                <g key={i}>
                  <circle cx={x} cy={y} r="2.5" fill="#27a644" />
                  <text
                    x={x}
                    y={h + 14}
                    fontSize="6.5"
                    fill="#62666d"
                    textAnchor="middle"
                  >
                    {p.label}
                  </text>
                </g>
              );
            })}
          </svg>
        </div>
      </div>

      {/* Filter impact */}
      <div className="mb-10">
        <p className="eyebrow mb-5">What Changed in v1.4.0</p>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div className="card p-5">
            <div className="flex items-center gap-2 mb-3">
              <span className="text-accent text-base">⬡</span>
              <p className="text-sm font-medium text-ink">ATR Quality Gate</p>
            </div>
            <p className="text-sm text-ink-sub leading-relaxed mb-4">
              FVG zone ≥ 0.3× ATR(14) and impulse body ≥ 1.0× ATR(14). Weak, low-energy setups are discarded before entry.
            </p>
            <div className="flex gap-3 pt-3 border-t border-hl">
              <div>
                <p className="text-[10px] text-ink-ter mb-0.5">Trades cut</p>
                <p className="font-mono text-sm font-semibold text-ink">224 → 83</p>
              </div>
              <div className="w-px bg-hl" />
              <div>
                <p className="text-[10px] text-ink-ter mb-0.5">PF impact</p>
                <p className="font-mono text-sm font-semibold text-bull">1.17 → 2.08</p>
              </div>
            </div>
          </div>

          <div className="card p-5">
            <div className="flex items-center gap-2 mb-3">
              <span className="text-accent text-base">◎</span>
              <p className="text-sm font-medium text-ink">Entry Zone 75%</p>
            </div>
            <p className="text-sm text-ink-sub leading-relaxed mb-4">
              Limit order placed 75% deep into the FVG instead of the midpoint. Forces price to retest the level, confirming it as real support/resistance.
            </p>
            <div className="flex gap-3 pt-3 border-t border-hl">
              <div>
                <p className="text-[10px] text-ink-ter mb-0.5">DD reduction</p>
                <p className="font-mono text-sm font-semibold text-bull">−63%</p>
              </div>
              <div className="w-px bg-hl" />
              <div>
                <p className="text-[10px] text-ink-ter mb-0.5">Win rate</p>
                <p className="font-mono text-sm font-semibold text-bull">50% → 62.7%</p>
              </div>
            </div>
          </div>

          <div className="card p-5">
            <div className="flex items-center gap-2 mb-3">
              <span className="text-accent text-base">◈</span>
              <p className="text-sm font-medium text-ink">Session 08–13 UTC</p>
            </div>
            <p className="text-sm text-ink-sub leading-relaxed mb-4">
              Entries restricted to London open + early NY overlap. XAU liquidity and directional moves concentrate in this window — other hours produce noise.
            </p>
            <div className="flex gap-3 pt-3 border-t border-hl">
              <div>
                <p className="text-[10px] text-ink-ter mb-0.5">vs all sessions</p>
                <p className="font-mono text-sm font-semibold text-bull">PF ↑ best</p>
              </div>
              <div className="w-px bg-hl" />
              <div>
                <p className="text-[10px] text-ink-ter mb-0.5">DD vs no filter</p>
                <p className="font-mono text-sm font-semibold text-bull">lowest</p>
              </div>
            </div>
          </div>
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
                {["Symbol", "Period", "TF", "Risk", "Version", "Trades", "Win Rate", "Profit Factor", "Return", "Max DD", ""].map(h => (
                  <th key={h}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {results.map((r, i) => (
                <tr key={i} className={r.highlight ? "bg-s2" : ""}>
                  <td className="font-mono font-medium text-ink">{r.symbol}</td>
                  <td className="font-medium text-ink">{r.period}</td>
                  <td className="font-mono text-ink-sub">{r.tf}</td>
                  <td className="font-mono text-ink-sub">{r.risk}</td>
                  <td className="font-mono text-xs text-ink-ter">{r.version}</td>
                  <td className="font-mono text-ink-md">{r.trades ?? "—"}</td>
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
          <ol className="space-y-4">
            {[
              ["Detect Impulse",    "3-candle momentum: body ≥ 50% of range, close in top/bottom 20%."],
              ["ATR Quality Gate",  "FVG zone ≥ 0.3× ATR(14). Impulse body ≥ 1.0× ATR(14). Weak setups discarded."],
              ["EMA-20 Filter",     "Long only above EMA-20, short only below. Confirms short-term trend."],
              ["Entry Zone 75%",    "Limit order at 75% depth inside FVG — waits for price to retrace before committing."],
              ["Manage Risk",       "SL at FVG boundary, TP at ≥ 1.5× RR, position sized at exactly N% risk per trade."],
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
