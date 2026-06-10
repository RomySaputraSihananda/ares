mod backtest;
mod detector;
mod helpers;
mod live;

use anyhow::Context;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::sync::Arc;

pub use helpers::{d2f, rolling_ema, size_position};

// ── env helpers ───────────────────────────────────────────────────────────────

fn env_str(key: &str, default: &str) -> String {
    let v = std::env::var(key).unwrap_or_default();
    if v.is_empty() { default.to_string() } else { v }
}

fn env_dec(key: &str, default: &str) -> anyhow::Result<Decimal> {
    env_str(key, default).parse().with_context(|| key.to_string())
}

fn env_usize(key: &str, default: &str) -> anyhow::Result<usize> {
    env_str(key, default).parse().with_context(|| key.to_string())
}

fn env_u32(key: &str, default: &str) -> anyhow::Result<u32> {
    env_str(key, default).parse().with_context(|| key.to_string())
}

fn env_u64(key: &str, default: &str) -> anyhow::Result<u64> {
    env_str(key, default).parse().with_context(|| key.to_string())
}

fn env_date(key: &str) -> anyhow::Result<Option<NaiveDate>> {
    match std::env::var(key) {
        Ok(s) if !s.is_empty() => Ok(Some(s.parse().with_context(|| key.to_string())?)),
        _ => Ok(None),
    }
}

fn env_spread_override() -> anyhow::Result<Option<Decimal>> {
    match std::env::var("SPREAD_OVERRIDE") {
        Ok(s) if !s.is_empty() && s != "0" => Ok(Some(s.parse().context("SPREAD_OVERRIDE")?)),
        _ => Ok(None),
    }
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
    let tf_str       = env_str("TIMEFRAME", "M5");
    let timeframe    = tf_str.parse::<domain::Timeframe>().map_err(|e| anyhow::anyhow!("{e}"))?;

    // ── symbol list ───────────────────────────────────────────────────────────
    // SYMBOLS=XAUUSDm,XAGUSDm,BTCUSDm  or  SYMBOL=XAUUSDm
    let symbols: Vec<String> = {
        let multi = std::env::var("SYMBOLS").unwrap_or_default();
        if !multi.is_empty() {
            multi.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        } else {
            let single = std::env::var("SYMBOL").context("SYMBOL or SYMBOLS missing")?;
            vec![single]
        }
    };

    // ── shared config ─────────────────────────────────────────────────────────
    let risk_pct         = env_dec("RISK_PCT",          "0.01")?;
    let body_pct_min     = env_dec("BODY_PCT_MIN",       "0.6")?;
    let close_pct_min    = env_dec("CLOSE_PCT_MIN",      "0.8")?;
    let fvg_expiry       = env_usize("FVG_EXPIRY_CANDLES", "10")?;
    let min_fvg_pips     = env_dec("MIN_FVG_PIPS",       "3")?;
    let min_sl_pips      = env_dec("MIN_SL_PIPS",        "5")?;
    let sl_buffer        = env_dec("SL_BUFFER",          "0")?;
    let min_rr           = env_dec("MIN_RR",             "1.5")?;
    let commission       = env_dec("COMMISSION_PER_LOT", "0")?;
    let slippage_points  = env_dec("SLIPPAGE_POINTS",    "5")?;
    let spread_override  = env_spread_override()?;
    let ema_period       = env_usize("EMA_PERIOD",       "20")?;

    let mt5 = Arc::new(mt5_client::Mt5Client::new(mt5_base_url));

    // ── live mode ─────────────────────────────────────────────────────────────
    let live_mode = std::env::var("LIVE").map(|v| v == "true" || v == "1").unwrap_or(false);
    if live_mode {
        let poll_secs = env_u64("LIVE_POLL_SECS", "30")?;
        let base_cfg = live::LiveConfig {
            symbol:            String::new(), // filled per-spawn
            timeframe,
            risk_pct,
            body_pct_min,
            close_pct_min,
            fvg_expiry_candles: fvg_expiry,
            min_fvg_pips,
            min_sl_pips,
            sl_buffer,
            min_rr,
            slippage_points,
            spread_override,
            ema_period,
            poll_secs,
        };

        let mut handles = Vec::new();
        for symbol in symbols {
            let mt5  = Arc::clone(&mt5);
            let mut cfg = base_cfg.clone();
            cfg.symbol = symbol;
            handles.push(tokio::spawn(async move {
                if let Err(e) = live::run(&*mt5, &cfg).await {
                    tracing::error!(symbol = %cfg.symbol, "live loop error: {e:#}");
                }
            }));
        }
        for h in handles { h.await.ok(); }
        return Ok(());
    }

    // ── backtest mode ─────────────────────────────────────────────────────────
    let cfg = backtest::BacktestConfig {
        timeframe,
        candles:         env_u32("BACKTEST_CANDLES",  "50000")?,
        balance:         env_dec("BACKTEST_BALANCE",  "600")?,
        risk_pct,
        body_pct_min,
        close_pct_min,
        fvg_expiry,
        min_fvg_pips,
        min_sl_pips,
        sl_buffer,
        min_rr,
        timeout_candles: env_usize("TIMEOUT_CANDLES", "0")?,
        commission,
        slippage_points,
        spread_override,
        ema_period,
        date_from:       env_date("DATE_FROM")?,
        date_to:         env_date("DATE_TO")?,
        stop_out_pct:    env_dec("STOP_OUT_PCT",      "0.0")?,
        tf_str:          tf_str.clone(),
    };

    for symbol in &symbols {
        backtest::run(&mt5, symbol, &cfg).await?;
    }

    Ok(())
}
