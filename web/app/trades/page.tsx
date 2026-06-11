import { getAccount, getDeals, type Deal } from "@/lib/mt5";
import { formatCurrency, formatDate } from "@/lib/format";
import EquityChart from "@/components/EquityChart";
import TradesRefresher from "@/components/TradesRefresher";

export const dynamic = "force-dynamic";
const MAGIC = 19730;

export default async function TradesPage() {
  let deals: Deal[] = [];
  let account = null;
  let error = false;

  try {
    const now = new Date().toISOString().slice(0, 19);
    [account, deals] = await Promise.all([
      getAccount(),
      getDeals("2026-06-01T00:00:00", now),
    ]);
  } catch { error = true; }

  const aresDeals = deals.filter(d => d.magic === MAGIC && d.type !== 2);
  const closed    = aresDeals.filter(d => d.entry === 1);
  const wins    = closed.filter(d => d.profit > 0);
  const losses  = closed.filter(d => d.profit < 0);
  const netPnl  = closed.reduce((s, d) => s + d.profit + d.swap + d.commission, 0);
  const grossW  = wins.reduce((s, d) => s + d.profit, 0);
  const grossL  = Math.abs(losses.reduce((s, d) => s + d.profit, 0));
  const pf      = grossL > 0 ? grossW / grossL : 0;
  const wr      = closed.length > 0 ? (wins.length / closed.length) * 100 : 0;
  const currency = account?.currency ?? "USD";

  return (
    <>
      <section className="pt-6 pb-12">
        <p className="eyebrow mb-4">Live Forward Test</p>
        <h1 className="text-4xl font-semibold tracking-[-0.032em] text-ink mb-3">
          Trade History
        </h1>
        <p className="text-[15px] text-ink-sub">
          All closed trades from ARES · magic {MAGIC}
        </p>
        <div className="mt-3 flex items-center gap-2">
          <span className="w-1.5 h-1.5 rounded-full bg-bull pulse-dot" />
          <TradesRefresher />
        </div>
      </section>

      {error ? (
        <div className="card text-center text-ink-sub text-sm mb-12 py-10">
          Unable to fetch trade data.
        </div>
      ) : (
        <>
          {/* Stats */}
          <div className="grid grid-cols-2 sm:grid-cols-5 gap-3 mb-10">
            {[
              { label: "Total Trades", value: closed.length.toString(), mono: true },
              { label: "Win Rate",     value: `${wr.toFixed(1)}%`,      mono: true },
              { label: "Profit Factor",value: pf > 0 ? pf.toFixed(2) : "—", mono: true },
              { label: "Net P&L",      value: formatCurrency(netPnl, currency), mono: true, colored: netPnl !== 0 ? netPnl > 0 : undefined },
              { label: "W / L",        value: `${wins.length} / ${losses.length}`, mono: true },
            ].map(({ label, value, mono, colored }) => (
              <div key={label} className="stat-tile">
                <p className="text-xs text-ink-sub mb-2">{label}</p>
                <p className={`${mono ? "font-mono" : ""} text-xl font-semibold ${
                  colored === true ? "text-bull" : colored === false ? "text-bear" : "text-ink"
                }`}>{value}</p>
              </div>
            ))}
          </div>

          {/* Equity curve */}
          <div className="card mb-10">
            <p className="eyebrow mb-6">Equity Curve</p>
            <EquityChart deals={closed} currency={currency} />
          </div>

          {/* Trade table */}
          <div className="rounded-lg border border-hl overflow-hidden">
            <div className="px-6 py-4 border-b border-hl">
              <p className="eyebrow">Closed Trades</p>
            </div>
            {closed.length === 0 ? (
              <div className="text-center text-ink-sub text-sm py-12">No closed trades yet.</div>
            ) : (
              <div className="overflow-x-auto">
                <table className="data-table">
                  <thead>
                    <tr>
                      {["Time", "Symbol", "Side", "Volume", "Price", "P&L"].map(h => (
                        <th key={h}>{h}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {closed.slice().reverse().map((d) => (
                      <tr key={d.ticket}>
                        <td className="text-ink-sub text-xs whitespace-nowrap">{formatDate(d.time)}</td>
                        <td className="font-mono font-medium text-ink">{d.symbol || "—"}</td>
                        <td>
                          <span className={`status-pill ${d.type === 0 ? "status-bull" : "status-bear"}`}>
                            {d.type === 0 ? "BUY" : "SELL"}
                          </span>
                        </td>
                        <td className="font-mono text-ink-md">{d.volume}</td>
                        <td className="font-mono text-ink-md">{d.price.toFixed(2)}</td>
                        <td className={`font-mono font-semibold ${d.profit >= 0 ? "text-bull" : "text-bear"}`}>
                          {formatCurrency(d.profit, currency)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}
    </>
  );
}

