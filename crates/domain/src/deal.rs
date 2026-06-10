use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::serde_helpers;

#[derive(Debug, Clone, Deserialize)]
pub struct Deal {
    pub ticket: u64,
    #[serde(with = "serde_helpers::naive_utc_secs")]
    pub time: DateTime<Utc>,
    pub entry: u8,   // 0 = open, 1 = close
    pub magic: u64,
    #[serde(deserialize_with = "serde_helpers::de_decimal")]
    pub volume: Decimal,
    #[serde(deserialize_with = "serde_helpers::de_decimal")]
    pub price: Decimal,
    #[serde(deserialize_with = "serde_helpers::de_decimal")]
    pub profit: Decimal,
    #[serde(deserialize_with = "serde_helpers::de_decimal")]
    pub commission: Decimal,
    #[serde(deserialize_with = "serde_helpers::de_decimal")]
    pub swap: Decimal,
    pub symbol: String,
    pub comment: String,
}
