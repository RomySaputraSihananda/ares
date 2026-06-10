use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{Side, Timeframe};
use futures_util::StreamExt;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio::time::{interval, Duration};

use crate::{
    detector,
    helpers::{d2f, fmt_price, rolling_ema, size_position},
    telegram::TelegramConfig,
};

const MAGIC:        u64 = 19730;
const CANDLE_FETCH: u32 = 100;

// ── config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LiveConfig {
    pub symbol:             String,
    pub timeframe:          Timeframe,
    pub risk_pct:           Decimal,
    pub body_pct_min:       Decimal,
    pub close_pct_min:      Decimal,
    pub fvg_expiry_candles: usize,
    pub min_fvg_pips:       Decimal,
    pub min_sl_pips:        Decimal,
    pub sl_buffer:          Decimal,
    pub min_rr:             Decimal,
    pub slippage_points:    Decimal,
    pub spread_override:    Option<Decimal>,
    pub ema_period:         usize,
    pub poll_secs:          u64,
    pub mt5_base_url:       String,
    pub telegram:           Option<TelegramConfig>,
}

// ── pending order state ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct State {
    ticket:        u64,
    expires_at:    DateTime<Utc>,
    tg_message_id: Option<i64>,
    side:          String,
    entry:         f64,
    sl:            f64,
    tp:            f64,
    volume:        f64,
}

impl State {
    fn path(symbol: &str) -> PathBuf { PathBuf::from(format!(".ares_state_{symbol}.json")) }

    fn load(symbol: &str) -> Option<Self> {
        serde_json::from_str(&std::fs::read_to_string(Self::path(symbol)).ok()?).ok()
    }

    fn save(&self, symbol: &str) -> Result<()> {
        std::fs::write(Self::path(symbol), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn clear(symbol: &str) { let _ = std::fs::remove_file(Self::path(symbol)); }
}

// ── open position state (persisted for SSE notifications after fill) ──────────

#[derive(Debug, Serialize, Deserialize)]
struct PosState {
    ticket:        u64,
    tg_message_id: Option<i64>,
    side:          String,
    entry:         f64,
    sl:            f64,
    tp:            f64,
    volume:        f64,
}

impl PosState {
    fn path(symbol: &str) -> PathBuf { PathBuf::from(format!(".ares_pos_{symbol}.json")) }

    fn load(symbol: &str) -> Option<Self> {
        serde_json::from_str(&std::fs::read_to_string(Self::path(symbol)).ok()?).ok()
    }

    fn save(&self, symbol: &str) -> Result<()> {
        std::fs::write(Self::path(symbol), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn clear(symbol: &str) { let _ = std::fs::remove_file(Self::path(symbol)); }
}

// ── SSE position event ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SsePosition {
    ticket:        u64,
    symbol:        String,
    #[serde(rename = "type")]
    pos_type:      u32,
    volume:        f64,
    price_open:    f64,
    price_current: Option<f64>,
    sl:            f64,
    tp:            f64,
    profit:        f64,
    magic:         u64,
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

    let tf_mins    = timeframe_minutes(cfg.timeframe);
    let expiry_dur = chrono::Duration::minutes(tf_mins * cfg.fvg_expiry_candles as i64);

    // spawn SSE position listener
    if cfg.telegram.is_some() {
        let mt5_arc  = Arc::new(mt5_client::Mt5Client::new(cfg.mt5_base_url.clone()));
        let http_arc = Arc::new(reqwest::Client::new());
        let symbol   = cfg.symbol.clone();
        let tg       = cfg.telegram.clone();
        let base_url = cfg.mt5_base_url.clone();
        tokio::spawn(async move {
            sse_task(mt5_arc, http_arc, base_url, symbol, tg).await;
        });
    }

    let http = reqwest::Client::new();
    let mut ticker = interval(Duration::from_secs(cfg.poll_secs));

    loop {
        ticker.tick().await;
        if let Err(e) = tick(
            mt5, cfg, &http, &sym_info, contract_size, point, prec,
            pip_size, min_sl, min_zone, slip, spread, profit_is_usd, expiry_dur,
        ).await {
            tracing::error!("tick error: {e:#}");
        }
    }
}

// ── single poll tick ──────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn tick(
    mt5:           &mt5_client::Mt5Client,
    cfg:           &LiveConfig,
    http:          &reqwest::Client,
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

    // ── 1. open position? ─────────────────────────────────────────────────────
    let positions = mt5.positions().await.context("fetch positions")?;
    let has_position = positions.iter().any(|p| p.symbol == *symbol && p.magic == MAGIC);

    if has_position {
        // ensure PosState exists so SSE task can find it on close
        if PosState::load(symbol).is_none() {
            if let Some(pos) = positions.iter().find(|p| p.symbol == *symbol && p.magic == MAGIC) {
                let tg_msg_id = State::load(symbol).and_then(|s| s.tg_message_id);
                let ps = PosState {
                    ticket:        pos.ticket,
                    tg_message_id: tg_msg_id,
                    side:          format!("{:?}", pos.side),
                    entry:         d2f(pos.price_open),
                    sl:            d2f(pos.sl),
                    tp:            d2f(pos.tp),
                    volume:        d2f(pos.volume),
                };
                let _ = ps.save(symbol);
            }
        }
        tracing::debug!(%symbol, "position open — skip");
        return Ok(());
    }

    // ── 2. manage pending order ───────────────────────────────────────────────
    if let Some(state) = State::load(symbol) {
        let orders       = mt5.orders(symbol).await.context("fetch orders")?;
        let still_pending = orders.iter().any(|o| o.ticket == state.ticket && o.magic == MAGIC);

        if still_pending {
            if Utc::now() < state.expires_at {
                tracing::debug!(%symbol, ticket = state.ticket, "pending order alive — waiting");
                return Ok(());
            }
            // expired — cancel
            tracing::info!(%symbol, ticket = state.ticket, "FVG setup expired — cancelling");
            match mt5.cancel_order(state.ticket, symbol).await {
                Ok(r)  => tracing::info!(retcode = r.retcode, "cancel ok"),
                Err(e) => tracing::warn!("cancel failed: {e:#}"),
            }
            if let (Some(tg), Some(msg_id)) = (&cfg.telegram, state.tg_message_id) {
                let text = format!(
                    "⏱ <b>EXPIRED</b>\n{} {}\nEntry: {:.5}\nNo fill after {} candles",
                    symbol, state.side, state.entry,
                    cfg.fvg_expiry_candles,
                );
                let _ = tg.edit(http, msg_id, &text).await;
            }
        } else {
            tracing::info!(%symbol, ticket = state.ticket, "order gone from MT5 (filled or cancelled)");
            // SSE task handles "Filled" notification when position opens
        }
        State::clear(symbol);
        return Ok(());
    }

    // ── 3. fetch candles ──────────────────────────────────────────────────────
    let candles = mt5
        .rates_from_pos(symbol, cfg.timeframe, 0, CANDLE_FETCH)
        .await
        .context("fetch candles")?;

    if candles.len() < 5 { return Ok(()); }
    let n = candles.len();

    let pre     = &candles[n - 4];
    let impulse = &candles[n - 3];
    let post    = &candles[n - 2];

    // ── 4. EMA filter ─────────────────────────────────────────────────────────
    let ema_val: Option<Decimal> = if cfg.ema_period > 0 && candles.len() >= cfg.ema_period {
        let closes: Vec<Decimal> = candles.iter().map(|c| c.close).collect();
        rolling_ema(&closes, cfg.ema_period)[n - 2]
    } else {
        Some(Decimal::ZERO)
    };

    // ── 5. detect FVG ─────────────────────────────────────────────────────────
    let fvg = match detector::detect(
        pre, impulse, post,
        cfg.body_pct_min, cfg.close_pct_min, min_zone,
        n - 4, cfg.fvg_expiry_candles,
    ) {
        Some(f) => f,
        None    => return Ok(()),
    };

    let ema_ok = match ema_val {
        Some(ema) => match fvg.side {
            Side::Long  => post.close > ema,
            Side::Short => post.close < ema,
        },
        None => false,
    };
    if !ema_ok { return Ok(()); }

    // ── 6. SL / TP ────────────────────────────────────────────────────────────
    let sl = match fvg.side {
        Side::Long  => fvg.impulse_sl - cfg.sl_buffer,
        Side::Short => fvg.impulse_sl + cfg.sl_buffer,
    };
    let sl_dist = (fvg.entry - sl).abs();
    if sl_dist < min_sl { return Ok(()); }
    let tp = match fvg.side {
        Side::Long  => fvg.entry + sl_dist * cfg.min_rr,
        Side::Short => fvg.entry - sl_dist * cfg.min_rr,
    };

    // ── 7. position size ──────────────────────────────────────────────────────
    let acct    = mt5.account().await.context("fetch account")?;
    let balance = Decimal::try_from(acct.balance).context("balance")?;
    let value_per_lot = if profit_is_usd || post.close == Decimal::ZERO {
        contract_size
    } else {
        contract_size / post.close
    };
    let volume = match size_position(
        balance, cfg.risk_pct, sl_dist, value_per_lot,
        sym_info.volume_step, sym_info.volume_min, sym_info.volume_max,
    ) {
        Some(v) => v,
        None    => return Ok(()),
    };

    // ── 8. place order ────────────────────────────────────────────────────────
    let req = mt5_client::TradeRequest::limit(
        fvg.side, symbol.clone(), d2f(volume), d2f(fvg.entry), d2f(sl), d2f(tp),
        MAGIC, format!("ares-{}", post.time.format("%m%d-%H%M")),
    );

    tracing::info!(
        %symbol, side = ?fvg.side,
        entry = %fmt_price(fvg.entry, prec), sl = %fmt_price(sl, prec), tp = %fmt_price(tp, prec),
        vol = %volume, bal = %balance, "placing limit order",
    );

    let result = mt5.place_order(&req).await.context("place_order")?;
    if result.retcode != 10009 {
        tracing::error!(retcode = result.retcode, comment = %result.comment, "order rejected");
        return Ok(());
    }
    tracing::info!(ticket = result.order, "order placed");

    let side_str = format!("{:?}", fvg.side);
    let rr       = d2f(cfg.min_rr);

    // send Telegram "Pending" message
    let tg_msg_id = if let Some(tg) = &cfg.telegram {
        let text = format!(
            "🟡 <b>PENDING</b>\n{} {}\nEntry: {}\nSL: {}   TP: {}\nVol: {} lot   RR: {:.1}",
            symbol, side_str,
            fmt_price(fvg.entry, prec), fmt_price(sl, prec), fmt_price(tp, prec),
            volume, rr,
        );
        match tg.send(http, &text).await {
            Ok(id)  => { tracing::info!(msg_id = id, "Telegram pending sent"); Some(id) }
            Err(e)  => { tracing::warn!("Telegram send failed: {e:#}"); None }
        }
    } else { None };

    let state = State {
        ticket:        result.order,
        expires_at:    Utc::now() + expiry_dur,
        tg_message_id: tg_msg_id,
        side:          side_str,
        entry:         d2f(fvg.entry),
        sl:            d2f(sl),
        tp:            d2f(tp),
        volume:        d2f(volume),
    };
    state.save(symbol).context("save state")?;
    Ok(())
}

// ── SSE position listener ─────────────────────────────────────────────────────

async fn sse_task(
    mt5:      Arc<mt5_client::Mt5Client>,
    http:     Arc<reqwest::Client>,
    base_url: String,
    symbol:   String,
    tg:       Option<TelegramConfig>,
) {
    loop {
        tracing::debug!(%symbol, "SSE connecting");
        if let Err(e) = sse_loop(&mt5, &http, &base_url, &symbol, &tg).await {
            tracing::warn!(%symbol, "SSE error: {e:#}");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn sse_loop(
    mt5:      &mt5_client::Mt5Client,
    http:     &reqwest::Client,
    base_url: &str,
    symbol:   &str,
    tg:       &Option<TelegramConfig>,
) -> Result<()> {
    let url = format!("{}/positions/stream?symbol={}", base_url, symbol);
    let resp = http.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("SSE stream status {}", resp.status());
    }

    let mut stream     = resp.bytes_stream();
    let mut buf        = String::new();
    let mut event_type = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        loop {
            match buf.find('\n') {
                None => break,
                Some(pos) => {
                    let line = buf[..pos].trim_end_matches('\r').to_string();
                    buf.drain(..=pos);

                    if line.starts_with("event:") {
                        event_type = line[6..].trim().to_string();
                    } else if line.starts_with("data:") && !event_type.is_empty() {
                        let data = line[5..].trim().to_string();
                        handle_sse_event(mt5, http, symbol, &event_type, &data, tg).await;
                        event_type.clear();
                    } else if line.is_empty() {
                        event_type.clear();
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_sse_event(
    mt5:        &mt5_client::Mt5Client,
    http:       &reqwest::Client,
    symbol:     &str,
    event_type: &str,
    data:       &str,
    tg:         &Option<TelegramConfig>,
) {
    let pos: SsePosition = match serde_json::from_str(data) {
        Ok(p)  => p,
        Err(e) => { tracing::warn!("SSE parse error: {e:#}"); return; }
    };

    if pos.magic != MAGIC || pos.symbol != symbol { return; }

    match event_type {
        "position_opened" => on_position_opened(http, symbol, &pos, tg).await,
        "position_closed" => on_position_closed(mt5, http, symbol, &pos, tg).await,
        _ => {}
    }
}

async fn on_position_opened(
    http:   &reqwest::Client,
    symbol: &str,
    pos:    &SsePosition,
    tg:     &Option<TelegramConfig>,
) {
    tracing::info!(%symbol, ticket = pos.ticket, "position opened (SSE)");

    // promote pending state → position state
    let state = State::load(symbol);
    let tg_msg_id = state.as_ref().and_then(|s| s.tg_message_id);

    let ps = PosState {
        ticket:        pos.ticket,
        tg_message_id: tg_msg_id,
        side:          if pos.pos_type == 0 { "Long".to_string() } else { "Short".to_string() },
        entry:         pos.price_open,
        sl:            pos.sl,
        tp:            pos.tp,
        volume:        pos.volume,
    };
    let _ = ps.save(symbol);
    State::clear(symbol);

    if let (Some(tg), Some(msg_id)) = (tg, tg_msg_id) {
        let side_str = &ps.side;
        let text = format!(
            "⚡ <b>FILLED</b>\n{} {}\nEntry: {:.5}\nSL: {:.5}   TP: {:.5}\nVol: {:.2} lot",
            symbol, side_str, pos.price_open, pos.sl, pos.tp, pos.volume,
        );
        let _ = tg.edit(http, msg_id, &text).await;
    }
}

async fn on_position_closed(
    mt5:    &mt5_client::Mt5Client,
    http:   &reqwest::Client,
    symbol: &str,
    pos:    &SsePosition,
    tg:     &Option<TelegramConfig>,
) {
    tracing::info!(%symbol, ticket = pos.ticket, profit = pos.profit, "position closed (SSE)");

    let ps = PosState::load(symbol);
    PosState::clear(symbol);

    let Some(tg) = tg else { return };
    let Some(msg_id) = ps.as_ref().and_then(|s| s.tg_message_id) else { return };

    let (icon, label) = if pos.profit >= 0.0 { ("✅", "TP HIT") } else { ("❌", "SL HIT") };
    let exit = pos.price_current.unwrap_or(0.0);
    let entry = ps.as_ref().map(|s| s.entry).unwrap_or(pos.price_open);
    let side_str = ps.as_ref().map(|s| s.side.as_str()).unwrap_or("?");

    let acct_bal = mt5.account().await.ok().map(|a| a.balance).unwrap_or(Decimal::ZERO);

    let text = format!(
        "{} <b>{} {:+.2}</b>\n{} {}\n{:.5} → {:.5}\nVol: {:.2} lot\nBal: ${}",
        icon, label, pos.profit,
        symbol, side_str,
        entry, exit,
        pos.volume, acct_bal,
    );
    let _ = tg.edit(http, msg_id, &text).await;

    // send PnL summary after close
    let _ = send_pnl_summary(mt5, http, symbol, tg).await;
}

// ── PnL summary ───────────────────────────────────────────────────────────────

async fn send_pnl_summary(
    mt5:    &mt5_client::Mt5Client,
    http:   &reqwest::Client,
    symbol: &str,
    tg:     &TelegramConfig,
) -> Result<()> {
    let now       = Utc::now();
    let today_str = now.format("%Y-%m-%dT00:00:00").to_string();
    let now_str   = now.format("%Y-%m-%dT%H:%M:%S").to_string();
    let epoch_str = "2020-01-01T00:00:00".to_string();

    let (today_deals, all_deals) = tokio::try_join!(
        mt5.history_deals(&today_str, &now_str, Some(symbol)),
        mt5.history_deals(&epoch_str, &now_str, Some(symbol)),
    )?;

    let summary_text = |label: &str, deals: &[domain::Deal]| {
        let closing: Vec<_> = deals.iter()
            .filter(|d| d.entry == 1 && d.magic == MAGIC)
            .collect();
        let total  = closing.len();
        let wins   = closing.iter().filter(|d| d.profit > Decimal::ZERO).count();
        let losses = total - wins;
        let profit: Decimal = closing.iter().map(|d| d.profit + d.commission + d.swap).sum();
        let wr = if total > 0 { wins as f64 / total as f64 * 100.0 } else { 0.0 };
        format!(
            "{}\nTrades: {} ({}W / {}L)   WR: {:.0}%\nProfit: ${:+.2}",
            label, total, wins, losses, wr, profit,
        )
    };

    let today_date = now.format("%Y-%m-%d").to_string();
    let text = format!(
        "📊 <b>Today</b> — {} {}\n{}\n\n📈 <b>All-time</b>\n{}",
        symbol, today_date,
        summary_text("", &today_deals),
        summary_text("", &all_deals),
    );

    tg.send(http, &text).await?;
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
