use serde::{Deserialize, Serialize};

// ── public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub mt5_connected: bool,
    pub mt5_version: String,
    pub api_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeRequest {
    pub action: u32,
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub order_type: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sl: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magic: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Ticket number — required for TRADE_ACTION_REMOVE (cancel pending order)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<u64>,
    /// Allowed price deviation in points
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deviation: Option<u32>,
}

impl TradeRequest {
    pub fn limit(
        side: domain::Side,
        symbol: impl Into<String>,
        volume: f64,
        price: f64,
        sl: f64,
        tp: f64,
        magic: u64,
        comment: impl Into<String>,
    ) -> Self {
        // TRADE_ACTION_PENDING = 5; BUY_LIMIT = 2, SELL_LIMIT = 3
        let order_type = match side {
            domain::Side::Long  => 2,
            domain::Side::Short => 3,
        };
        Self {
            action: 5,
            symbol: symbol.into(),
            volume: Some(volume),
            order_type: Some(order_type),
            price: Some(price),
            sl: Some(sl),
            tp: Some(tp),
            magic: Some(magic),
            comment: Some(comment.into()),
            order: None,
            deviation: None,
        }
    }

    pub fn cancel(symbol: impl Into<String>, ticket: u64) -> Self {
        // TRADE_ACTION_REMOVE = 8 — bridge requires volume/type/price even though
        // MT5 ignores them for remove actions; send minimal valid values.
        Self {
            action: 8,
            symbol: symbol.into(),
            volume: Some(0.01),
            order_type: Some(0),
            price: Some(0.0),
            sl: None,
            tp: None,
            magic: None,
            comment: None,
            order: Some(ticket),
            deviation: None,
        }
    }
}

/// A pending (unfilled) limit order in MT5.
#[derive(Debug, Clone, Deserialize)]
pub struct PendingOrder {
    pub ticket: u64,
    pub symbol: String,
    #[serde(rename = "type")]
    pub order_type: u32,
    pub volume_initial: f64,
    pub price_open: f64,
    pub sl: f64,
    pub tp: f64,
    pub magic: u64,
    pub comment: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderCheckResult {
    pub retcode: u32,
    pub balance: f64,
    pub equity: f64,
    pub profit: f64,
    pub margin: f64,
    pub margin_free: f64,
    pub margin_level: f64,
    pub comment: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TradeResult {
    pub retcode: u32,
    pub order: u64,
    pub comment: String,
}

// ── internal types (crate-visible only) ───────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct DataVec<T> {
    pub(crate) data: Vec<T>,
}

#[derive(Deserialize)]
pub(crate) struct DataOne<T> {
    pub(crate) data: T,
}

#[derive(Deserialize)]
pub(crate) struct ApiErrorBody {
    pub(crate) detail: String,
}
