#!/usr/bin/env python3
"""
Portfolio simulator — combines trade logs from multiple Ares backtest runs
into a single compounding account, sorted chronologically by open time.

Usage: python3 scripts/portfolio_sim.py < combined_trades.txt
Env:   STOP_OUT_PCT=0.0 (fraction of initial balance, e.g. 0.2 = stop at $120 on $600)
"""

import sys
import os
import re
from decimal import Decimal

TRADE_RE = re.compile(
    r'\[(\d{4}-\d{2}-\d{2} \d{2}:\d{2}) \S+\].*?pnl=([+-]?\d+\.\d+) bal=(\d+\.\d+)'
)

def parse_trades(lines):
    trades = []
    for line in lines:
        m = TRADE_RE.search(line)
        if not m:
            continue
        open_time  = m.group(1)
        pnl_abs    = Decimal(m.group(2))
        bal_after  = Decimal(m.group(3))
        bal_before = bal_after - pnl_abs
        if bal_before <= 0:
            continue
        pnl_ratio = pnl_abs / bal_before
        trades.append((open_time, pnl_ratio))
    return trades

def simulate(trades, start_balance=Decimal("600"), stop_out_pct=Decimal("0.0")):
    trades_sorted  = sorted(trades, key=lambda t: t[0])
    stop_out_bal   = start_balance * stop_out_pct

    balance        = start_balance
    peak           = balance
    max_dd         = Decimal("0")
    wins = losses  = 0
    margin_called  = False

    for open_time, pnl_ratio in trades_sorted:
        if balance <= stop_out_bal:
            print(f"  *** MARGIN CALL at {open_time}: balance ${balance:.2f} ≤ stop-out ${stop_out_bal:.2f} ***")
            margin_called = True
            break

        pnl      = balance * pnl_ratio
        balance += pnl
        if balance > peak:
            peak = balance
        dd = balance - peak
        if dd < max_dd:
            max_dd = dd
        if pnl >= 0:
            wins  += 1
        else:
            losses += 1

    total = wins + losses
    wr    = wins / total * 100 if total else 0
    ret   = (balance - start_balance) / start_balance * 100

    print("─" * 47)
    print(f"  Combined Portfolio (all pairs)")
    print("─" * 47)
    print(f"  Stop-out level : {float(stop_out_pct)*100:.0f}% of initial (${stop_out_bal:.2f})")
    print(f"  Trades         : {total}  (W={wins} L={losses}  WR={wr:.1f}%)")
    print(f"  Start balance  : ${start_balance:,.2f}")
    print(f"  Final balance  : ${balance:,.2f}")
    print(f"  Total return   : {ret:+,.1f}%")
    print(f"  Max drawdown   : ${max_dd:,.2f}  ({float(max_dd/peak)*100:.1f}% of peak)")
    if margin_called:
        print(f"  *** MARGIN CALL triggered ***")
    print("─" * 47)

if __name__ == "__main__":
    lines        = sys.stdin.readlines()
    trades       = parse_trades(lines)
    start_bal    = Decimal(os.environ.get("BACKTEST_BALANCE", "600"))
    stop_out_pct = Decimal(os.environ.get("STOP_OUT_PCT", "0.0"))
    if not trades:
        print("No trades found in input.")
        sys.exit(1)
    simulate(trades, start_balance=start_bal, stop_out_pct=stop_out_pct)
