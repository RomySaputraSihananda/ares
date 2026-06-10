/// Simulate: pending → filled → TP/SL result (single trade lifecycle)
use anyhow::Result;
use ares::telegram::TelegramConfig;
use reqwest::Client;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let token     = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN");
    let chat_id   = std::env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID").parse::<i64>()?;
    let thread_id = std::env::var("TELEGRAM_THREAD_ID").ok().and_then(|s| s.parse::<i64>().ok());

    let tg   = TelegramConfig { token, chat_id, thread_id };
    let http = Client::new();

    // ── Flow: pending → filled → TP hit ──────────────────────────────────────
    let msg = tg.send(&http,
        "🟡 <b>PENDING</b>\nXAUUSDm · Long\n\nEntry  <code>3325.50</code>\nTP     <code>3333.75</code>\nSL     <code>3320.00</code>\nVol 0.03 lot  ·  RR 1:1.5"
    ).await?;
    tracing::info!(msg_id = msg, "PENDING sent");
    sleep(Duration::from_secs(3)).await;

    tg.edit(&http, msg,
        "⚡ <b>FILLED</b>\nXAUUSDm · Long\n\nEntry  <code>3325.50</code>\nTP     <code>3333.75</code>\nSL     <code>3320.00</code>\nVol 0.03 lot"
    ).await?;
    tracing::info!(msg_id = msg, "FILLED edited");
    sleep(Duration::from_secs(3)).await;

    tg.edit(&http, msg,
        "✅ <b>TAKE PROFIT  +12.68</b>\nXAUUSDm · Long\n\n<code>3325.50</code> → <code>3333.75</code>\nVol 0.03 lot  ·  Bal $5012.68"
    ).await?;
    tracing::info!(msg_id = msg, "TP HIT edited");
    sleep(Duration::from_secs(3)).await;

    // ── PnL summary ───────────────────────────────────────────────────────────
    tg.send(&http,
        "📊 <b>PnL Summary</b>\n\n<b>Today</b>  ·  XAUUSDm  ·  2026-06-11\n2 trades  ·  1W 1L  ·  WR 50%\nNet  <b>+$5.18</b>\n\n<b>All-time</b>\n17 trades  ·  10W 7L  ·  WR 59%\nNet  <b>+$482.41</b>"
    ).await?;
    tracing::info!("PnL summary sent");

    println!("\nDone ✅  check Telegram");
    Ok(())
}
