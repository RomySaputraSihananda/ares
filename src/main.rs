mod detector;

use anyhow::Context;
use chrono::NaiveDate;
use domain::Side;
use rust_decimal::Decimal;

// ── helpers ──────────────────────────────────────────────────────────────────

fn rolling_ema(prices: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    let k = Decimal::from(2u32) / Decimal::from((period + 1) as u32);
    let mut out = vec![None; prices.len()];
    if prices.len() < period { return out; }
    let seed: Decimal = prices[..period].iter().sum::<Decimal>() / Decimal::from(period);
    out[period - 1] = Some(seed);
    let mut ema = seed;
    for i in period..prices.len() {
        ema = prices[i] * k + ema * (Decimal::ONE - k);
        out[i] = Some(ema);
    }
    out
}

fn size_position(
    balance:       Decimal,
    risk_pct:      Decimal,
    sl_distance:   Decimal,
    value_per_lot: Decimal,
    vol_step:      Decimal,
    min_vol:       Decimal,
    max_vol:       Decimal,
) -> Option<Decimal> {
    if sl_distance == Decimal::ZERO { return None; }
    let raw    = (balance * risk_pct) / (sl_distance * value_per_lot);
    let volume = (raw / vol_step).floor() * vol_step;
    if volume < min_vol { return None; }
    Some(volume.min(max_vol))
}

fn actual_entry(side: Side, level: Decimal, spread: Decimal) -> Decimal {
    match side {
        Side::Long  => level + spread,
        Side::Short => level,
    }
}

fn actual_exit(side: Side, level: Decimal, is_sl: bool, spread: Decimal, slip: Decimal) -> Decimal {
    match (side, is_sl) {
        (Side::Long,  false) => level,
        (Side::Long,  true)  => level - slip,
        (Side::Short, false) => level + spread,
        (Side::Short, true)  => level + spread + slip,
    }
}

fn fmt_price(d: Decimal, prec: usize) -> String { format!("{0:.1$}", d, prec) }
fn fmt_pnl(pnl: Decimal) -> String {
    if pnl >= Decimal::ZERO { format!("+{:.2}", pnl) } else { format!("{:.2}", pnl) }
}

// ── open trade ────────────────────────────────────────────────────────────────

struct OpenTrade {
    open_time:      String,
    side:           Side,
    entry_level:    Decimal,
    actual_entry:   Decimal,
    sl:             Decimal,
    tp:             Decimal,
    volume:         Decimal,
    open_candle_idx: usize,
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mt5_base_url = std::env::var("MT5_BASE_URL").context("MT5_BASE_URL missing")?;
    let symbol       = std::env::var("SYMBOL").context("SYMBOL missing")?;
    let tf_str       = std::env::var("TIMEFRAME").unwrap_or_else(|_| "M5".to_string());

    let backtest_candles: u32 = std::env::var("BACKTEST_CANDLES")
        .unwrap_or_else(|_| "50000".to_string()).parse().context("BACKTEST_CANDLES")?;
    let backtest_balance: Decimal = std::env::var("BACKTEST_BALANCE")
        .unwrap_or_else(|_| "600".to_string()).parse().context("BACKTEST_BALANCE")?;
    let risk_pct: Decimal = std::env::var("RISK_PCT")
        .unwrap_or_else(|_| "0.01".to_string()).parse().context("RISK_PCT")?;

    let body_pct_min: Decimal = std::env::var("BODY_PCT_MIN")
        .unwrap_or_else(|_| "0.6".to_string()).parse().context("BODY_PCT_MIN")?;
    let close_pct_min: Decimal = std::env::var("CLOSE_PCT_MIN")
        .unwrap_or_else(|_| "0.8".to_string()).parse().context("CLOSE_PCT_MIN")?;
    let fvg_expiry: usize = std::env::var("FVG_EXPIRY_CANDLES")
        .unwrap_or_else(|_| "10".to_string()).parse().context("FVG_EXPIRY_CANDLES")?;
    let min_fvg_pips: Decimal = std::env::var("MIN_FVG_PIPS")
        .unwrap_or_else(|_| "3".to_string()).parse().context("MIN_FVG_PIPS")?;
    let min_sl_pips: Decimal = std::env::var("MIN_SL_PIPS")
        .unwrap_or_else(|_| "5".to_string()).parse().context("MIN_SL_PIPS")?;
    let sl_buffer: Decimal = std::env::var("SL_BUFFER")
        .unwrap_or_else(|_| "0".to_string()).parse().context("SL_BUFFER")?;
    let min_rr: Decimal = std::env::var("MIN_RR")
        .unwrap_or_else(|_| "1.5".to_string()).parse().context("MIN_RR")?;
    let timeout_candles: usize = std::env::var("TIMEOUT_CANDLES")
        .unwrap_or_else(|_| "0".to_string()).parse().context("TIMEOUT_CANDLES")?;

    let commission_per_lot: Decimal = std::env::var("COMMISSION_PER_LOT")
        .unwrap_or_else(|_| "0".to_string()).parse().context("COMMISSION_PER_LOT")?;
    let slippage_points: Decimal = std::env::var("SLIPPAGE_POINTS")
        .unwrap_or_else(|_| "5".to_string()).parse().context("SLIPPAGE_POINTS")?;
    let spread_override: Option<Decimal> = match std::env::var("SPREAD_OVERRIDE") {
        Ok(s) => Some(s.parse().context("SPREAD_OVERRIDE")?),
        Err(_) => None,
    };

    let ema_period: usize = std::env::var("EMA_PERIOD")
        .unwrap_or_else(|_| "20".to_string()).parse().context("EMA_PERIOD")?;

    let date_from: Option<NaiveDate> = match std::env::var("DATE_FROM") {
        Ok(s) => Some(s.parse().context("DATE_FROM")?),
        Err(_) => None,
    };
    let date_to: Option<NaiveDate> = match std::env::var("DATE_TO") {
        Ok(s) => Some(s.parse().context("DATE_TO")?),
        Err(_) => None,
    };

    // Stop-out level as fraction of initial balance (0.0 = Exness default: equity hits $0).
    // e.g. STOP_OUT_PCT=0.2 stops trading when equity drops to 20% of starting balance.
    let stop_out_pct: Decimal = std::env::var("STOP_OUT_PCT")
        .unwrap_or_else(|_| "0.0".to_string()).parse().context("STOP_OUT_PCT")?;

    let timeframe = tf_str.parse::<domain::Timeframe>().map_err(|e| anyhow::anyhow!("{e}"))?;
    let mt5 = mt5_client::Mt5Client::new(mt5_base_url);

    tracing::info!(symbol = %symbol, tf = %tf_str, backtest_candles, "fetching data");

    let (sym_info, candles) = tokio::try_join!(
        mt5.symbol(&symbol),
        mt5.rates_from_pos(&symbol, timeframe, 0, backtest_candles),
    )?;

    let total          = candles.len();
    let contract_size  = sym_info.trade_contract_size;
    let point          = sym_info.point;
    let prec           = sym_info.digits as usize;
    let spread_price   = spread_override.unwrap_or_else(|| Decimal::from(sym_info.spread) * point);
    let slippage_price = slippage_points * point;
    let profit_is_usd  = sym_info.currency_profit.eq_ignore_ascii_case("USD");
    // for 5- or 3-decimal pairs (odd digit count) 1 pip = 10 points; for 2/4-decimal = 1 point
    let pip_size = if sym_info.digits % 2 == 1 { point * Decimal::from(10u32) } else { point };
    let min_zone_size  = min_fvg_pips * pip_size;
    let min_sl_size    = min_sl_pips  * pip_size;

    let ema_vals: Vec<Option<Decimal>> = if ema_period > 0 {
        let closes: Vec<Decimal> = candles.iter().map(|c| c.close).collect();
        rolling_ema(&closes, ema_period)
    } else {
        vec![None; total]
    };

    tracing::info!(total, "starting walk-forward");

    let stop_out_balance = backtest_balance * stop_out_pct;

    let mut balance        = backtest_balance;
    let mut peak           = balance;
    let mut max_drawdown   = Decimal::ZERO;
    let mut open_trade: Option<OpenTrade>              = None;
    let mut pending_fvg: Option<detector::PendingFvg>  = None;
    let mut margin_called  = false;

    let mut trades         = 0u32;
    let mut wins           = 0u32;
    let mut losses         = 0u32;
    let mut timeouts       = 0u32;
    let mut missed_fills   = 0u32;
    let mut total_pnl      = Decimal::ZERO;
    let mut total_friction = Decimal::ZERO;
    let mut sum_wins       = Decimal::ZERO;
    let mut sum_losses     = Decimal::ZERO;
    let mut max_consec     = 0u32;
    let mut cur_consec     = 0u32;

    for i in 2..total {
        let candle = &candles[i];
        let date   = candle.time.date_naive();

        // ── manage open trade ────────────────────────────────────────────────
        if let Some(ref t) = open_trade {
            // timeout: force close after N candles
            if timeout_candles > 0 && (i - t.open_candle_idx) >= timeout_candles {
                let t           = open_trade.take().unwrap();
                let exit_lvl    = candle.close;
                let exit        = actual_exit(t.side, exit_lvl, false, spread_price, slippage_price);
                let commission  = commission_per_lot * t.volume;
                let profit_rate = if profit_is_usd || exit <= Decimal::ZERO { Decimal::ONE } else { Decimal::ONE / exit };
                let pnl = (match t.side {
                    Side::Long  => (exit - t.actual_entry) * t.volume * contract_size,
                    Side::Short => (t.actual_entry - exit) * t.volume * contract_size,
                }) * profit_rate - commission;
                balance += pnl;
                if balance > peak { peak = balance; }
                let dd = balance - peak;
                if dd < max_drawdown { max_drawdown = dd; }
                timeouts  += 1;
                trades    += 1;
                total_pnl += pnl;
                if pnl >= Decimal::ZERO { wins += 1; sum_wins += pnl; cur_consec = 0; }
                else { losses += 1; sum_losses += pnl.abs(); cur_consec += 1; if cur_consec > max_consec { max_consec = cur_consec; } }
                println!(
                    "[{} {}] {} {} entry={} sl={} tp={} vol={:.2} → TIMEOUT exit={} pnl={} bal={:.2}",
                    t.open_time, tf_str, symbol,
                    if t.side == Side::Long { "LONG " } else { "SHORT" },
                    fmt_price(t.actual_entry, prec), fmt_price(t.sl, prec), fmt_price(t.tp, prec), t.volume,
                    fmt_price(exit, prec), fmt_pnl(pnl), balance,
                );
                continue;
            }
            // ── stop-out check: worst-case equity on this candle ─────────────
            if stop_out_pct > Decimal::ZERO {
                let worst_price = match t.side {
                    Side::Long  => candle.low,
                    Side::Short => candle.high,
                };
                let pr_w = if profit_is_usd || worst_price <= Decimal::ZERO {
                    Decimal::ONE
                } else {
                    Decimal::ONE / worst_price
                };
                let unrealized_w = (match t.side {
                    Side::Long  => (worst_price - t.actual_entry) * t.volume * contract_size,
                    Side::Short => (t.actual_entry - worst_price) * t.volume * contract_size,
                }) * pr_w - commission_per_lot * t.volume;
                if balance + unrealized_w <= stop_out_balance {
                    let t       = open_trade.take().unwrap();
                    let exit    = actual_exit(t.side, worst_price, true, spread_price, slippage_price);
                    let commission = commission_per_lot * t.volume;
                    let pnl = (match t.side {
                        Side::Long  => (exit - t.actual_entry) * t.volume * contract_size,
                        Side::Short => (t.actual_entry - exit) * t.volume * contract_size,
                    }) * pr_w - commission;
                    balance += pnl;
                    if balance > peak { peak = balance; }
                    let dd = balance - peak;
                    if dd < max_drawdown { max_drawdown = dd; }
                    trades  += 1;
                    losses  += 1;
                    sum_losses += pnl.abs();
                    cur_consec += 1;
                    if cur_consec > max_consec { max_consec = cur_consec; }
                    total_pnl += pnl;
                    println!(
                        "[{} {}] {} {} entry={} → STOP-OUT exit={} pnl={} bal={:.2}",
                        t.open_time, tf_str, symbol,
                        if t.side == Side::Long { "LONG " } else { "SHORT" },
                        fmt_price(t.actual_entry, prec),
                        fmt_price(exit, prec),
                        fmt_pnl(pnl), balance,
                    );
                    margin_called = true;
                    break;
                }
            }

            let (sl_hit, tp_hit) = match t.side {
                Side::Long  => (candle.low  <= t.sl, candle.high >= t.tp),
                Side::Short => (candle.high >= t.sl, candle.low  <= t.tp),
            };
            if sl_hit || tp_hit {
                let t        = open_trade.take().unwrap();
                let is_sl    = sl_hit;
                let exit_lvl = if is_sl { t.sl } else { t.tp };
                let label    = if is_sl { "SL" } else { "TP" };
                let exit     = actual_exit(t.side, exit_lvl, is_sl, spread_price, slippage_price);

                let commission  = commission_per_lot * t.volume;
                let profit_rate = if profit_is_usd || exit <= Decimal::ZERO {
                    Decimal::ONE
                } else {
                    Decimal::ONE / exit
                };
                let pnl = (match t.side {
                    Side::Long  => (exit - t.actual_entry) * t.volume * contract_size,
                    Side::Short => (t.actual_entry - exit) * t.volume * contract_size,
                }) * profit_rate - commission;

                let fl_rate = if profit_is_usd || exit_lvl <= Decimal::ZERO {
                    Decimal::ONE
                } else {
                    Decimal::ONE / exit_lvl
                };
                let frictionless = (match t.side {
                    Side::Long  => (exit_lvl - t.entry_level) * t.volume * contract_size,
                    Side::Short => (t.entry_level - exit_lvl) * t.volume * contract_size,
                }) * fl_rate;
                let friction = frictionless - pnl;

                balance += pnl;
                if balance > peak { peak = balance; }
                let dd = balance - peak;
                if dd < max_drawdown { max_drawdown = dd; }

                if is_sl {
                    losses     += 1;
                    sum_losses += pnl.abs();
                    cur_consec += 1;
                    if cur_consec > max_consec { max_consec = cur_consec; }
                } else {
                    wins       += 1;
                    sum_wins   += pnl;
                    cur_consec  = 0;
                }
                trades         += 1;
                total_pnl      += pnl;
                total_friction += friction;

                println!(
                    "[{} {}] {} {} entry={} sl={} tp={} vol={:.2} → {label} exit={} friction={} pnl={} bal={:.2}",
                    t.open_time, tf_str, symbol,
                    if t.side == Side::Long { "LONG " } else { "SHORT" },
                    fmt_price(t.actual_entry, prec),
                    fmt_price(t.sl, prec),
                    fmt_price(t.tp, prec),
                    t.volume,
                    fmt_price(exit, prec),
                    fmt_pnl(-friction),
                    fmt_pnl(pnl), balance,
                );
            }
            continue;
        }

        // ── date filter ───────────────────────────────────────────────────────
        if date_from.is_some_and(|d| date < d) { continue; }
        if date_to.is_some_and(|d| date > d)   { continue; }

        // ── expire stale FVG ──────────────────────────────────────────────────
        if pending_fvg.as_ref().is_some_and(|f| i >= f.expiry_idx) {
            missed_fills += 1;
            pending_fvg = None;
        }

        // ── try to fill pending FVG ───────────────────────────────────────────
        if let Some(ref fvg) = pending_fvg {
            if fvg.is_touched(candle) {
                let ema_ok = if ema_period > 0 {
                    match ema_vals.get(i).copied().flatten() {
                        Some(ema) => match fvg.side {
                            Side::Long  => candle.close > ema,
                            Side::Short => candle.close < ema,
                        },
                        None => false,
                    }
                } else {
                    true
                };

                if ema_ok {
                    // SL placed at impulse candle's structural extreme, not zone edge
                    let sl = match fvg.side {
                        Side::Long  => fvg.impulse_sl - sl_buffer,
                        Side::Short => fvg.impulse_sl + sl_buffer,
                    };
                    let sl_dist = (fvg.entry - sl).abs();
                    if sl_dist < min_sl_size {
                        pending_fvg = None;
                        continue;
                    }
                    let tp = match fvg.side {
                        Side::Long  => fvg.entry + sl_dist * min_rr,
                        Side::Short => fvg.entry - sl_dist * min_rr,
                    };

                    let fill_ok = match fvg.side {
                        Side::Long  => candle.low  <= fvg.entry,
                        Side::Short => candle.high >= fvg.entry,
                    };
                    if !fill_ok {
                        // zone touched but limit order not reached yet — keep FVG pending
                        continue;
                    }

                    if balance <= stop_out_balance {
                        pending_fvg   = None;
                        margin_called = true;
                        break;
                    }

                    let value_per_lot = if profit_is_usd || candle.close == Decimal::ZERO {
                        contract_size
                    } else {
                        contract_size / candle.close
                    };

                    match size_position(balance, risk_pct, sl_dist, value_per_lot,
                        sym_info.volume_step, sym_info.volume_min, sym_info.volume_max)
                    {
                        None    => { pending_fvg = None; continue; }
                        Some(v) => {
                            let ae = actual_entry(fvg.side, fvg.entry, spread_price);
                            open_trade = Some(OpenTrade {
                                open_time:       candle.time.format("%Y-%m-%d %H:%M").to_string(),
                                side:            fvg.side,
                                entry_level:     fvg.entry,
                                actual_entry:    ae,
                                sl,
                                tp,
                                volume:          v,
                                open_candle_idx: i,
                            });
                            pending_fvg = None;
                        }
                    }
                } else {
                    pending_fvg = None;
                }
            }
            continue;
        }

        // ── detect new momentum FVG ───────────────────────────────────────────
        pending_fvg = detector::detect(
            &candles[i - 2],
            &candles[i - 1],
            candle,
            body_pct_min,
            close_pct_min,
            min_zone_size,
            i,
            fvg_expiry,
        );
    }

    // ── end-of-data timeout ───────────────────────────────────────────────────
    if let Some(t) = open_trade.take() {
        let exit_lvl    = candles.last().unwrap().close;
        let exit        = actual_exit(t.side, exit_lvl, false, spread_price, slippage_price);
        let commission  = commission_per_lot * t.volume;
        let profit_rate = if profit_is_usd || exit <= Decimal::ZERO {
            Decimal::ONE
        } else {
            Decimal::ONE / exit
        };
        let pnl = (match t.side {
            Side::Long  => (exit - t.actual_entry) * t.volume * contract_size,
            Side::Short => (t.actual_entry - exit) * t.volume * contract_size,
        }) * profit_rate - commission;

        balance += pnl;
        timeouts  += 1;
        trades    += 1;
        total_pnl += pnl;

        println!(
            "[{} {}] {} {} entry={} → TIMEOUT exit={} pnl={} bal={:.2}",
            t.open_time, tf_str, symbol,
            if t.side == Side::Long { "LONG " } else { "SHORT" },
            fmt_price(t.actual_entry, prec),
            fmt_price(exit, prec),
            fmt_pnl(pnl), balance,
        );
    }

    // ── summary ───────────────────────────────────────────────────────────────
    let win_pct     = if trades > 0 { wins     as f64 / trades as f64 * 100.0 } else { 0.0 };
    let loss_pct    = if trades > 0 { losses   as f64 / trades as f64 * 100.0 } else { 0.0 };
    let timeout_pct = if trades > 0 { timeouts as f64 / trades as f64 * 100.0 } else { 0.0 };
    let avg_win     = if wins   > 0 { sum_wins   / Decimal::from(wins)   } else { Decimal::ZERO };
    let avg_loss    = if losses > 0 { sum_losses / Decimal::from(losses) } else { Decimal::ZERO };
    let expectancy  = if trades > 0 { total_pnl  / Decimal::from(trades) } else { Decimal::ZERO };
    let pf          = if sum_losses > Decimal::ZERO { sum_wins / sum_losses } else { Decimal::MAX };
    let ret_pct     = (balance - backtest_balance) / backtest_balance * Decimal::from(100u32);

    println!("─────────────────────────────────────────");
    println!("Ares Scalper: {} {} | {} candles", symbol, tf_str, total);
    let timeout_str = if timeout_candles > 0 { format!("  timeout={timeout_candles}c") } else { String::new() };
    println!("Strategy : Momentum FVG  body≥{body_pct_min}  close≥{close_pct_min}  expiry={fvg_expiry}c  min_fvg={min_fvg_pips}pip  min_sl={min_sl_pips}pip  min_rr={min_rr}{timeout_str}");
    println!("Friction : spread={}  slip={}  commission/lot={}", fmt_price(spread_price, prec), fmt_price(slippage_price, prec), commission_per_lot);
    println!("─────────────────────────────────────────");
    println!("Trades         : {trades}");
    println!("Win            : {wins}  ({win_pct:.1}%)");
    println!("Loss           : {losses}  ({loss_pct:.1}%)");
    println!("Timeout        : {timeouts}  ({timeout_pct:.1}%)");
    println!("Missed fills   : {missed_fills}");
    println!("Max consec loss: {max_consec}");
    println!("─────────────────────────────────────────");
    println!("Avg win        : +{avg_win:.2}");
    println!("Avg loss       : -{avg_loss:.2}");
    println!("Expectancy     : {}", fmt_pnl(expectancy));
    println!("Profit factor  : {pf:.2}");
    println!("Total friction : {}", fmt_pnl(-total_friction));
    println!("─────────────────────────────────────────");
    println!("Total PnL      : {}", fmt_pnl(total_pnl));
    println!("Max Drawdown   : {max_drawdown:.2}");
    println!("Return         : {ret_pct:.1}%");
    println!("Final Balance  : {balance:.2}");
    if margin_called {
        println!("*** MARGIN CALL — stop-out triggered at {:.1}% of initial balance ***", stop_out_pct * Decimal::from(100u32));
    }
    println!("─────────────────────────────────────────");

    Ok(())
}
