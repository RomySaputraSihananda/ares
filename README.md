# Ares

M5 momentum FVG scalping bot for MetaTrader 5. Detects 3-candle impulse patterns that leave a Fair Value Gap, enters on limit orders back into the zone, and manages SL/TP automatically.

## Strategy

1. **Impulse detection** — a candle whose body ≥ `BODY_PCT_MIN` of its range, closing in the top/bottom `CLOSE_PCT_MIN` fraction
2. **FVG zone** — the gap between the pre-impulse candle and the post-impulse candle must be ≥ `MIN_FVG_PIPS`
3. **EMA filter** — trade only in the direction of the `EMA_PERIOD`-bar trend
4. **Entry** — limit order at the FVG zone edge; expires after `FVG_EXPIRY_CANDLES` bars if unfilled
5. **SL** — at the structural extreme of the impulse candle ± `SL_BUFFER`
6. **TP** — `MIN_RR × SL distance` from entry

## Requirements

- Rust (stable, 2024 edition)
- [MT5 HTTP Bridge](https://github.com/romysaputrasihananda/mt5-bridge) running and reachable at `MT5_BASE_URL`

## Setup

```bash
cp .env.example .env
# edit .env — set MT5_BASE_URL and your preferred params
```

## Usage

### Backtest

```bash
cargo run --release
```

Runs a walk-forward simulation over the last `BACKTEST_CANDLES` M5 bars and prints a summary.

### Live trading

```bash
LIVE=true cargo run --release
```

Polls MT5 every `LIVE_POLL_SECS` seconds. Places a pending limit order when a valid FVG is detected; cancels it when the setup expires. Uses magic number `19730` to identify its own orders and positions.

State for each symbol is persisted in `.ares_state_{symbol}.json` so the bot survives restarts without orphaning orders.

### Telegram notifications

Set `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` to receive real-time trade updates. Each trade lifecycle is tracked as a single edited message:

| Event | Message |
|---|---|
| Pending order placed | 🟡 PENDING — entry / SL / TP / lot |
| Limit order filled | ⚡ FILLED |
| Position SL/TP modified | 🔄 MODIFIED |
| TP hit | ✅ TP HIT — P&L + balance |
| SL hit | ❌ SL HIT — P&L + balance |
| Order expired | ⏱ EXPIRED |
| Order cancelled externally | 🚫 CANCELLED |

A 📊 PnL summary (today + all-time) is sent automatically every hour.

### Multi-pair

```bash
SYMBOLS=XAUUSDm,USOILm cargo run --release
```

Backtest runs symbols sequentially; live mode runs one async task per symbol.

## Configuration

Copy `.env.example` to `.env` and adjust. All fields are optional except `MT5_BASE_URL` and at least one of `SYMBOL` / `SYMBOLS`.

| Variable | Default | Description |
|---|---|---|
| `MT5_BASE_URL` | — | MT5 bridge base URL (required) |
| `SYMBOL` | — | Single symbol, e.g. `XAUUSDm` |
| `SYMBOLS` | — | Comma-separated list, overrides `SYMBOL` |
| `TIMEFRAME` | `M5` | Candle timeframe |
| `RISK_PCT` | `0.01` | Fraction of balance to risk per trade |
| `BODY_PCT_MIN` | `0.5` | Minimum body/range ratio for impulse candle |
| `CLOSE_PCT_MIN` | `0.8` | Close must be in top/bottom this fraction of range |
| `FVG_EXPIRY_CANDLES` | `10` | Bars before an unfilled setup is cancelled |
| `MIN_FVG_PIPS` | `1` | Minimum FVG zone width in pips |
| `MIN_SL_PIPS` | `5` | Minimum SL distance in pips |
| `SL_BUFFER` | `0` | Extra buffer beyond impulse extreme for SL |
| `MIN_RR` | `1.5` | Minimum reward:risk ratio |
| `EMA_PERIOD` | `20` | EMA trend filter period (0 = disabled) |
| `COMMISSION_PER_LOT` | `7` | Round-trip commission in USD |
| `SLIPPAGE_POINTS` | `2` | SL slippage in MT5 points |
| `SPREAD_OVERRIDE` | `0` | Override spread (0 = use live spread) |
| `BACKTEST_BALANCE` | `600` | Starting balance for backtest |
| `BACKTEST_CANDLES` | `50000` | Number of M5 bars to fetch |
| `DATE_FROM` | — | Optional backtest start date `YYYY-MM-DD` |
| `DATE_TO` | — | Optional backtest end date `YYYY-MM-DD` |
| `TIMEOUT_CANDLES` | `0` | Force-close open trade after N bars (0 = disabled) |
| `LIVE` | `false` | Set to `true` to enable live mode |
| `LIVE_POLL_SECS` | `30` | Poll interval for live mode |
| `TELEGRAM_BOT_TOKEN` | — | Bot token from @BotFather (optional) |
| `TELEGRAM_CHAT_ID` | — | Target chat/group ID |
| `TELEGRAM_THREAD_ID` | — | Forum thread ID (optional) |

## Backtest results

XAUUSDm M5 · 50 000 bars · $600 start · 1% risk · EMA20 · $7/lot commission

| Metric | Value |
|---|---|
| Trades | 1 309 |
| Win rate | 50.9% |
| Profit factor | 1.24 |
| Return | +345% |
| Max drawdown | −$200 |

> Results are in-sample. The dataset covers a period of elevated XAU volatility. Out-of-sample validation is recommended before live deployment.

## Project layout

```
ares/
├── src/
│   ├── main.rs        # entry point, env parsing, mode dispatch
│   ├── backtest.rs    # walk-forward simulation engine
│   ├── live.rs        # live trading loop + SSE position listener
│   ├── detector.rs    # momentum FVG detection
│   ├── telegram.rs    # Telegram Bot API client
│   ├── helpers.rs     # shared math utilities
│   └── bin/
│       └── sim.rs     # Telegram notification simulator (no MT5 needed)
└── crates/
    ├── domain/        # shared types (Candle, Symbol, Timeframe, Side…)
    └── mt5-client/    # async HTTP client for the MT5 bridge
```
