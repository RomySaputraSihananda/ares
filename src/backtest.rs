use anyhow::Result;
use chrono::NaiveDate;
use domain::Side;
use rust_decimal::Decimal;

use crate::detector;
use crate::helpers::{actual_entry, actual_exit, fmt_price, fmt_pnl, rolling_ema, size_position};

// ── config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub timeframe:        domain::Timeframe,
    pub candles:          u32,
    pub balance:          Decimal,
    pub risk_pct:         Decimal,
    pub body_pct_min:     Decimal,
    pub close_pct_min:    Decimal,
    pub fvg_expiry:       usize,
    pub min_fvg_pips:     Decimal,
    pub min_sl_pips:      Decimal,
    pub sl_buffer:        Decimal,
    pub min_rr:           Decimal,
    pub timeout_candles:  usize,
    pub commission:       Decimal,
    pub slippage_points:  Decimal,
    pub spread_override:  Option<Decimal>,
    pub ema_period:       usize,
    pub date_from:        Option<NaiveDate>,
    pub date_to:          Option<NaiveDate>,
    pub stop_out_pct:     Decimal,
    pub tf_str:           String,
}

// ── open trade ────────────────────────────────────────────────────────────────

struct OpenTrade {
    open_time:       String,
    side:            Side,
    entry_level:     Decimal,
    actual_entry:    Decimal,
    sl:              Decimal,
    tp:              Decimal,
    volume:          Decimal,
    open_candle_idx: usize,
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn run(mt5: &mt5_client::Mt5Client, symbol: &str, cfg: &BacktestConfig) -> Result<()> {
    tracing::info!(%symbol, tf = %cfg.tf_str, candles = cfg.candles, "fetching data");

    let (sym_info, candles) = tokio::try_join!(
        mt5.symbol(symbol),
        mt5.rates_from_pos(symbol, cfg.timeframe, 0, cfg.candles),
    )?;

    let total          = candles.len();
    let contract_size  = sym_info.trade_contract_size;
    let point          = sym_info.point;
    let prec           = sym_info.digits as usize;
    let spread_price   = cfg.spread_override
        .unwrap_or_else(|| Decimal::from(sym_info.spread) * point);
    let slippage_price = cfg.slippage_points * point;
    let profit_is_usd  = sym_info.currency_profit.eq_ignore_ascii_case("USD");
    let pip_size       = if sym_info.digits % 2 == 1 { point * Decimal::from(10u32) } else { point };
    let min_zone_size  = cfg.min_fvg_pips * pip_size;
    let min_sl_size    = cfg.min_sl_pips  * pip_size;

    let ema_vals: Vec<Option<Decimal>> = if cfg.ema_period > 0 {
        let closes: Vec<Decimal> = candles.iter().map(|c| c.close).collect();
        rolling_ema(&closes, cfg.ema_period)
    } else {
        vec![None; total]
    };

    tracing::info!(total, %symbol, "starting walk-forward");

    let stop_out_balance = cfg.balance * cfg.stop_out_pct;

    let mut balance        = cfg.balance;
    let mut peak           = balance;
    let mut max_drawdown   = Decimal::ZERO;
    let mut open_trade: Option<OpenTrade>             = None;
    let mut pending_fvg: Option<detector::PendingFvg> = None;
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

    'outer: for i in 2..total {
        let candle = &candles[i];
        let date   = candle.time.date_naive();

        // ── manage open trade ────────────────────────────────────────────────
        if let Some(ref t) = open_trade {
            if cfg.timeout_candles > 0 && (i - t.open_candle_idx) >= cfg.timeout_candles {
                let t           = open_trade.take().unwrap();
                let exit_lvl    = candle.close;
                let exit        = actual_exit(t.side, exit_lvl, false, spread_price, slippage_price);
                let commission  = cfg.commission * t.volume;
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
                    t.open_time, cfg.tf_str, symbol,
                    if t.side == Side::Long { "LONG " } else { "SHORT" },
                    fmt_price(t.actual_entry, prec), fmt_price(t.sl, prec), fmt_price(t.tp, prec), t.volume,
                    fmt_price(exit, prec), fmt_pnl(pnl), balance,
                );
                continue;
            }

            if cfg.stop_out_pct > Decimal::ZERO {
                let worst_price = match t.side {
                    Side::Long  => candle.low,
                    Side::Short => candle.high,
                };
                let pr_w = if profit_is_usd || worst_price <= Decimal::ZERO { Decimal::ONE } else { Decimal::ONE / worst_price };
                let unrealized_w = (match t.side {
                    Side::Long  => (worst_price - t.actual_entry) * t.volume * contract_size,
                    Side::Short => (t.actual_entry - worst_price) * t.volume * contract_size,
                }) * pr_w - cfg.commission * t.volume;
                if balance + unrealized_w <= stop_out_balance {
                    let t       = open_trade.take().unwrap();
                    let exit    = actual_exit(t.side, worst_price, true, spread_price, slippage_price);
                    let commission = cfg.commission * t.volume;
                    let pnl = (match t.side {
                        Side::Long  => (exit - t.actual_entry) * t.volume * contract_size,
                        Side::Short => (t.actual_entry - exit) * t.volume * contract_size,
                    }) * pr_w - commission;
                    balance += pnl;
                    if balance > peak { peak = balance; }
                    let dd = balance - peak;
                    if dd < max_drawdown { max_drawdown = dd; }
                    trades  += 1; losses += 1;
                    sum_losses += pnl.abs();
                    cur_consec += 1;
                    if cur_consec > max_consec { max_consec = cur_consec; }
                    total_pnl += pnl;
                    println!(
                        "[{} {}] {} {} entry={} → STOP-OUT exit={} pnl={} bal={:.2}",
                        t.open_time, cfg.tf_str, symbol,
                        if t.side == Side::Long { "LONG " } else { "SHORT" },
                        fmt_price(t.actual_entry, prec), fmt_price(exit, prec), fmt_pnl(pnl), balance,
                    );
                    margin_called = true;
                    break 'outer;
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
                let commission  = cfg.commission * t.volume;
                let profit_rate = if profit_is_usd || exit <= Decimal::ZERO { Decimal::ONE } else { Decimal::ONE / exit };
                let pnl = (match t.side {
                    Side::Long  => (exit - t.actual_entry) * t.volume * contract_size,
                    Side::Short => (t.actual_entry - exit) * t.volume * contract_size,
                }) * profit_rate - commission;
                let fl_rate = if profit_is_usd || exit_lvl <= Decimal::ZERO { Decimal::ONE } else { Decimal::ONE / exit_lvl };
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
                    losses += 1; sum_losses += pnl.abs();
                    cur_consec += 1;
                    if cur_consec > max_consec { max_consec = cur_consec; }
                } else {
                    wins += 1; sum_wins += pnl; cur_consec = 0;
                }
                trades += 1; total_pnl += pnl; total_friction += friction;
                println!(
                    "[{} {}] {} {} entry={} sl={} tp={} vol={:.2} → {label} exit={} friction={} pnl={} bal={:.2}",
                    t.open_time, cfg.tf_str, symbol,
                    if t.side == Side::Long { "LONG " } else { "SHORT" },
                    fmt_price(t.actual_entry, prec), fmt_price(t.sl, prec), fmt_price(t.tp, prec), t.volume,
                    fmt_price(exit, prec), fmt_pnl(-friction), fmt_pnl(pnl), balance,
                );
            }
            continue;
        }

        // ── date filter ───────────────────────────────────────────────────────
        if cfg.date_from.is_some_and(|d| date < d) { continue; }
        if cfg.date_to.is_some_and(|d| date > d)   { continue; }

        // ── expire stale FVG ──────────────────────────────────────────────────
        if pending_fvg.as_ref().is_some_and(|f| i >= f.expiry_idx) {
            missed_fills += 1;
            pending_fvg = None;
        }

        // ── try to fill pending FVG ───────────────────────────────────────────
        if let Some(ref fvg) = pending_fvg {
            if fvg.is_touched(candle) {
                let ema_ok = if cfg.ema_period > 0 {
                    match ema_vals.get(i).copied().flatten() {
                        Some(ema) => match fvg.side {
                            Side::Long  => candle.close > ema,
                            Side::Short => candle.close < ema,
                        },
                        None => false,
                    }
                } else { true };

                if ema_ok {
                    let sl = match fvg.side {
                        Side::Long  => fvg.impulse_sl - cfg.sl_buffer,
                        Side::Short => fvg.impulse_sl + cfg.sl_buffer,
                    };
                    let sl_dist = (fvg.entry - sl).abs();
                    if sl_dist < min_sl_size { pending_fvg = None; continue; }
                    let tp = match fvg.side {
                        Side::Long  => fvg.entry + sl_dist * cfg.min_rr,
                        Side::Short => fvg.entry - sl_dist * cfg.min_rr,
                    };
                    let fill_ok = match fvg.side {
                        Side::Long  => candle.low  <= fvg.entry,
                        Side::Short => candle.high >= fvg.entry,
                    };
                    if !fill_ok { continue; }
                    if balance <= stop_out_balance {
                        margin_called = true; break 'outer;
                    }
                    let value_per_lot = if profit_is_usd || candle.close == Decimal::ZERO {
                        contract_size
                    } else {
                        contract_size / candle.close
                    };
                    match size_position(balance, cfg.risk_pct, sl_dist, value_per_lot,
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
                                sl, tp, volume: v,
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
            &candles[i - 2], &candles[i - 1], candle,
            cfg.body_pct_min, cfg.close_pct_min, min_zone_size, i, cfg.fvg_expiry,
        );
    }

    // ── end-of-data timeout ───────────────────────────────────────────────────
    if let Some(t) = open_trade.take() {
        let exit_lvl    = candles.last().unwrap().close;
        let exit        = actual_exit(t.side, exit_lvl, false, spread_price, slippage_price);
        let commission  = cfg.commission * t.volume;
        let profit_rate = if profit_is_usd || exit <= Decimal::ZERO { Decimal::ONE } else { Decimal::ONE / exit };
        let pnl = (match t.side {
            Side::Long  => (exit - t.actual_entry) * t.volume * contract_size,
            Side::Short => (t.actual_entry - exit) * t.volume * contract_size,
        }) * profit_rate - commission;
        balance += pnl;
        timeouts += 1; trades += 1; total_pnl += pnl;
        println!(
            "[{} {}] {} {} entry={} → TIMEOUT exit={} pnl={} bal={:.2}",
            t.open_time, cfg.tf_str, symbol,
            if t.side == Side::Long { "LONG " } else { "SHORT" },
            fmt_price(t.actual_entry, prec), fmt_price(exit, prec), fmt_pnl(pnl), balance,
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
    let ret_pct     = (balance - cfg.balance) / cfg.balance * Decimal::from(100u32);

    println!("─────────────────────────────────────────");
    println!("Ares Scalper: {} {} | {} candles", symbol, cfg.tf_str, total);
    let timeout_str = if cfg.timeout_candles > 0 { format!("  timeout={}c", cfg.timeout_candles) } else { String::new() };
    println!("Strategy : Momentum FVG  body≥{}  close≥{}  expiry={}c  min_fvg={}pip  min_sl={}pip  min_rr={}{}",
        cfg.body_pct_min, cfg.close_pct_min, cfg.fvg_expiry, cfg.min_fvg_pips, cfg.min_sl_pips, cfg.min_rr, timeout_str);
    println!("Friction : spread={}  slip={}  commission/lot={}",
        fmt_price(spread_price, prec), fmt_price(slippage_price, prec), cfg.commission);
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
        println!("*** MARGIN CALL — stop-out at {:.1}% of initial balance ***",
            cfg.stop_out_pct * Decimal::from(100u32));
    }
    println!("─────────────────────────────────────────");

    Ok(())
}
