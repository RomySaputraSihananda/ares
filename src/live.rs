use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{Side, Timeframe};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::time::{interval, Duration};

use crate::{detector, helpers::{d2f, fmt_price, rolling_ema, size_position}};

// Magic number that identifies all Ares orders/positions in MT5.
const MAGIC: u64 = 19730;

// How many recent candles to fetch per tick (must cover EMA warm-up + 3 for FVG).
const CANDLE_FETCH: u32 = 100;

// ── config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LiveConfig {
    pub symbol: String,
    pub timeframe: Timeframe,
    pub risk_pct: Decimal,
    pub body_pct_min: Decimal,
    pub close_pct_min: Decimal,
    pub fvg_expiry_candles: usize,
    pub min_fvg_pips: Decimal,
    pub min_sl_pips: Decimal,
    pub sl_buffer: Decimal,
    pub min_rr: Decimal,
    pub slippage_points: Decimal,
    pub spread_override: Option<Decimal>,
    pub ema_period: usize,
    pub poll_secs: u64,
}

// ── persisted state ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct State {
    ticket: u64,
    expires_at: DateTime<Utc>,
}

impl State {
    fn path(symbol: &str) -> PathBuf {
        PathBuf::from(format!(".ares_state_{symbol}.json"))
    }

    fn load(symbol: &str) -> Option<Self> {
        let content = std::fs::read_to_string(Self::path(symbol)).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save(&self, symbol: &str) -> Result<()> {
        std::fs::write(Self::path(symbol), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn clear(symbol: &str) {
        let _ = std::fs::remove_file(Self::path(symbol));
    }
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn run(mt5: &mt5_client::Mt5Client, cfg: &LiveConfig) -> Result<()> {
    tracing::info!(symbol = %cfg.symbol, tf = ?cfg.timeframe, "live mode starting");

    let sym_info = mt5.symbol(&cfg.symbol).await.context("fetch symbol info")?;
    let point         = sym_info.point;
    let prec          = sym_info.digits as usize;
    let contract_size = sym_info.trade_contract_size;
    let profit_is_usd = sym_info.currency_profit.eq_ignore_ascii_case("USD");
    let pip_size  = if sym_info.digits % 2 == 1 { point * Decimal::from(10u32) } else { point };
    let min_sl    = cfg.min_sl_pips  * pip_size;
    let min_zone  = cfg.min_fvg_pips * pip_size;
    let slip      = cfg.slippage_points * point;
    let spread    = cfg.spread_override
        .unwrap_or_else(|| Decimal::from(sym_info.spread) * point);

    let tf_mins = timeframe_minutes(cfg.timeframe);
    let expiry_dur = chrono::Duration::minutes(tf_mins * cfg.fvg_expiry_candles as i64);

    let mut ticker = interval(Duration::from_secs(cfg.poll_secs));

    loop {
        ticker.tick().await;

        if let Err(e) = tick(
            mt5, cfg, &sym_info, contract_size, point, prec, pip_size, min_sl, min_zone,
            slip, spread, profit_is_usd, expiry_dur,
        )
        .await
        {
            tracing::error!("tick error: {e:#}");
        }
    }
}

// ── single poll tick ──────────────────────────────────────────────────────────

async fn tick(
    mt5:           &mt5_client::Mt5Client,
    cfg:           &LiveConfig,
    sym_info:      &domain::Symbol,
    contract_size: Decimal,
    _point:        Decimal,
    prec:          usize,
    _pip_size:     Decimal,
    min_sl:        Decimal,
    min_zone:      Decimal,
    _slip:         Decimal,
    _spread:       Decimal,
    profit_is_usd: bool,
    expiry_dur:    chrono::Duration,
) -> Result<()> {
    let symbol = &cfg.symbol;

    // ── 1. check for open positions by this bot ───────────────────────────────
    let positions = mt5.positions().await.context("fetch positions")?;
    let has_position = positions
        .iter()
        .any(|p| p.symbol == *symbol && p.magic == MAGIC);

    if has_position {
        tracing::debug!(%symbol, "position already open — skip");
        return Ok(());
    }

    // ── 2. manage pending order state ────────────────────────────────────────
    if let Some(state) = State::load(symbol) {
        let orders = mt5.orders(symbol).await.context("fetch orders")?;
        let still_pending = orders.iter().any(|o| o.ticket == state.ticket && o.magic == MAGIC);

        if still_pending {
            if Utc::now() < state.expires_at {
                tracing::debug!(%symbol, ticket = state.ticket, "pending order alive — waiting");
                return Ok(());
            }
            // expired — cancel
            tracing::info!(%symbol, ticket = state.ticket, "FVG setup expired — cancelling order");
            match mt5.cancel_order(state.ticket, symbol).await {
                Ok(r) => tracing::info!(retcode = r.retcode, "cancel ok"),
                Err(e) => tracing::warn!("cancel failed: {e:#}"),
            }
        } else {
            tracing::info!(%symbol, ticket = state.ticket, "pending order no longer in MT5 (filled/cancelled externally)");
        }
        State::clear(symbol);
        return Ok(());
    }

    // ── 3. fetch recent candles ───────────────────────────────────────────────
    let candles = mt5
        .rates_from_pos(symbol, cfg.timeframe, 0, CANDLE_FETCH)
        .await
        .context("fetch candles")?;

    if candles.len() < 5 {
        tracing::warn!(%symbol, "too few candles");
        return Ok(());
    }

    let n = candles.len();

    // Use last 3 fully-closed bars: [n-4], [n-3], [n-2] — skip [n-1] which may
    // still be forming at poll time.
    let pre      = &candles[n - 4];
    let impulse  = &candles[n - 3];
    let post     = &candles[n - 2];
    let last_idx = n - 4; // detector uses absolute index only for expiry, we don't need it

    // ── 4. EMA trend filter ───────────────────────────────────────────────────
    let ema_val: Option<Decimal> = if cfg.ema_period > 0 && candles.len() >= cfg.ema_period {
        let closes: Vec<Decimal> = candles.iter().map(|c| c.close).collect();
        let emas = rolling_ema(&closes, cfg.ema_period);
        emas[n - 2]
    } else {
        Some(Decimal::ZERO)
    };

    // ── 5. detect momentum FVG ────────────────────────────────────────────────
    let fvg = detector::detect(
        pre,
        impulse,
        post,
        cfg.body_pct_min,
        cfg.close_pct_min,
        min_zone,
        last_idx,
        cfg.fvg_expiry_candles,
    );

    let fvg = match fvg {
        Some(f) => f,
        None    => return Ok(()),
    };

    // EMA filter
    let ema_ok = match ema_val {
        Some(ema) => match fvg.side {
            Side::Long  => post.close > ema,
            Side::Short => post.close < ema,
        },
        None => false,
    };
    if !ema_ok {
        tracing::debug!(%symbol, ?fvg.side, "EMA filter rejected FVG");
        return Ok(());
    }

    // ── 6. compute SL / TP ───────────────────────────────────────────────────
    let sl = match fvg.side {
        Side::Long  => fvg.impulse_sl - cfg.sl_buffer,
        Side::Short => fvg.impulse_sl + cfg.sl_buffer,
    };
    let sl_dist = (fvg.entry - sl).abs();
    if sl_dist < min_sl {
        tracing::debug!(%symbol, %sl_dist, "SL too tight — skip");
        return Ok(());
    }
    let tp = match fvg.side {
        Side::Long  => fvg.entry + sl_dist * cfg.min_rr,
        Side::Short => fvg.entry - sl_dist * cfg.min_rr,
    };

    // ── 7. size position ──────────────────────────────────────────────────────
    let acct = mt5.account().await.context("fetch account")?;
    let balance = Decimal::try_from(acct.balance).context("balance conversion")?;

    let ref_price = post.close;
    let value_per_lot = if profit_is_usd || ref_price == Decimal::ZERO {
        contract_size
    } else {
        contract_size / ref_price
    };

    let volume = match size_position(
        balance, cfg.risk_pct, sl_dist, value_per_lot,
        sym_info.volume_step, sym_info.volume_min, sym_info.volume_max,
    ) {
        Some(v) => v,
        None    => {
            tracing::warn!(%symbol, "position sizing returned None (SL=0 or too small)");
            return Ok(());
        }
    };

    // ── 8. place pending limit order ──────────────────────────────────────────
    let entry_price = fvg.entry;
    let req = mt5_client::TradeRequest::limit(
        fvg.side, symbol.clone(), d2f(volume), d2f(entry_price), d2f(sl), d2f(tp),
        MAGIC, format!("ares-{}", post.time.format("%m%d-%H%M")),
    );

    tracing::info!(
        %symbol, side = ?fvg.side,
        entry = %fmt_price(entry_price, prec),
        sl    = %fmt_price(sl, prec),
        tp    = %fmt_price(tp, prec),
        vol   = %volume,
        bal   = %balance,
        "placing limit order",
    );

    let result = mt5.place_order(&req).await.context("place_order")?;
    if result.retcode != 10009 {
        tracing::error!(retcode = result.retcode, comment = %result.comment, "order rejected");
        return Ok(());
    }

    tracing::info!(ticket = result.order, "order placed");

    let state = State {
        ticket:     result.order,
        expires_at: Utc::now() + expiry_dur,
    };
    state.save(symbol).context("save state")?;

    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn timeframe_minutes(tf: Timeframe) -> i64 {
    match tf {
        Timeframe::M1  => 1,
        Timeframe::M2  => 2,
        Timeframe::M3  => 3,
        Timeframe::M4  => 4,
        Timeframe::M5  => 5,
        Timeframe::M6  => 6,
        Timeframe::M10 => 10,
        Timeframe::M12 => 12,
        Timeframe::M15 => 15,
        Timeframe::M20 => 20,
        Timeframe::M30 => 30,
        Timeframe::H1  => 60,
        Timeframe::H2  => 120,
        Timeframe::H3  => 180,
        Timeframe::H4  => 240,
        Timeframe::H6  => 360,
        Timeframe::H8  => 480,
        Timeframe::H12 => 720,
        Timeframe::D1  => 1440,
        Timeframe::W1  => 10080,
        Timeframe::Mn1 => 43200,
    }
}

