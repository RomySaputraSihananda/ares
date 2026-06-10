/// Simulate: pending → filled → TP/SL result (single trade lifecycle)
use anyhow::Result;
use ares::telegram::TelegramConfig;
use reqwest::Client;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let token   = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN");
    let chat_id = std::env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID").parse::<i64>()?;
    let thread_id = std::env::var("TELEGRAM_THREAD_ID").ok().and_then(|s| s.parse::<i64>().ok());

    let tg   = TelegramConfig { token, chat_id, thread_id };
    let http = Client::new();

    // ── 1. Pending order placed ───────────────────────────────────────────────
    tracing::info!("step 1: PENDING");
    let msg_id = tg.send(&http,
        "🟡 <b>PENDING</b>\nXAUUSDm Long\nEntry: 3325.50\nSL: 3320.00   TP: 3333.75\nVol: 0.03 lot   RR: 1.5",
    ).await?;
    tracing::info!(msg_id, "sent");

    sleep(Duration::from_secs(3)).await;

    // ── 2. Limit order triggered / filled ────────────────────────────────────
    tracing::info!("step 2: FILLED");
    tg.edit(&http, msg_id,
        "⚡ <b>FILLED</b>\nXAUUSDm Long\nEntry: 3325.50\nSL: 3320.00   TP: 3333.75\nVol: 0.03 lot",
    ).await?;
    tracing::info!(msg_id, "edited");

    sleep(Duration::from_secs(3)).await;

    // ── 3a. TP hit ────────────────────────────────────────────────────────────
    tracing::info!("step 3: TP HIT");
    tg.edit(&http, msg_id,
        "✅ <b>TP HIT +12.68</b>\nXAUUSDm Long\n3325.50 → 3333.75\nVol: 0.03 lot\nBal: $5012.68",
    ).await?;
    tracing::info!(msg_id, "edited TP HIT");

    // uncomment to test SL hit instead:
    // tg.edit(&http, msg_id,
    //     "❌ <b>SL HIT -7.50</b>\nXAUUSDm Long\n3325.50 → 3320.00\nVol: 0.03 lot\nBal: $4992.50",
    // ).await?;

    println!("\nDone ✅  check Telegram");
    Ok(())
}
