use domain::{Candle, Side};
use rust_decimal::Decimal;

/// A momentum FVG setup pending entry fill.
#[derive(Debug, Clone)]
pub struct PendingFvg {
    pub side:        Side,
    pub zone_high:   Decimal,
    pub zone_low:    Decimal,
    pub entry:       Decimal,   // FVG midpoint — limit order level
    pub impulse_sl:  Decimal,   // impulse candle's low (long) or high (short) — structural SL
    pub expiry_idx:  usize,     // invalidate if not filled by this walk-forward index
}

impl PendingFvg {
    /// Returns true if `c` touches the FVG zone (limit-order fill semantics).
    pub fn is_touched(&self, c: &Candle) -> bool {
        match self.side {
            Side::Long  => c.low  <= self.zone_high,
            Side::Short => c.high >= self.zone_low,
        }
    }
}

/// Classify a candle as momentum (Long/Short) or None.
///
/// Criteria:
/// - body/range >= `body_pct_min` (e.g. 0.6 = 60% body)
/// - close is in the top `close_pct_min` fraction for bullish
///   (or bottom for bearish); e.g. 0.8 means close ≥ 80% of range from low
pub fn momentum_side(c: &Candle, body_pct_min: Decimal, close_pct_min: Decimal) -> Option<Side> {
    let range = c.high - c.low;
    if range == Decimal::ZERO {
        return None;
    }
    let body_pct  = (c.close - c.open).abs() / range;
    if body_pct < body_pct_min {
        return None;
    }
    let close_pos = (c.close - c.low) / range; // 0 = at low, 1 = at high
    if c.close > c.open && close_pos >= close_pct_min {
        Some(Side::Long)
    } else if c.close < c.open && close_pos <= (Decimal::ONE - close_pct_min) {
        Some(Side::Short)
    } else {
        None
    }
}

/// Attempt to build a PendingFvg from three consecutive candles `[pre, impulse, post]`.
///
/// Rules:
/// - `impulse` must qualify as a momentum candle
/// - There must be a price gap between `pre` and `post` matching the momentum side
///   (bullish: post.low > pre.high; bearish: post.high < pre.low)
/// - FVG zone must be at least `min_zone_size` wide (rejects micro-gaps)
///
/// `post_idx` is the walk-forward index of `post` (used to set expiry).
pub fn detect(
    pre:            &Candle,
    impulse:        &Candle,
    post:           &Candle,
    body_pct_min:   Decimal,
    close_pct_min:  Decimal,
    min_zone_size:  Decimal,
    post_idx:       usize,
    expiry_candles: usize,
) -> Option<PendingFvg> {
    let side = momentum_side(impulse, body_pct_min, close_pct_min)?;

    let (zone_low, zone_high) = match side {
        Side::Long  if post.low  > pre.high => (pre.high, post.low),
        Side::Short if post.high < pre.low  => (post.high, pre.low),
        _ => return None,
    };

    if zone_high - zone_low < min_zone_size {
        return None;
    }

    let entry = (zone_high + zone_low) / Decimal::from(2u32);
    let impulse_sl = match side {
        Side::Long  => impulse.low,
        Side::Short => impulse.high,
    };

    Some(PendingFvg {
        side,
        zone_high,
        zone_low,
        entry,
        impulse_sl,
        expiry_idx: post_idx + expiry_candles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn candle(o: &str, h: &str, l: &str, c: &str) -> Candle {
        Candle {
            time: Utc::now(),
            open:        o.parse().unwrap(),
            high:        h.parse().unwrap(),
            low:         l.parse().unwrap(),
            close:       c.parse().unwrap(),
            tick_volume: 100,
            spread:      2,
            real_volume: 0,
        }
    }

    #[test]
    fn momentum_bullish_detected() {
        // body = 0.0080 / range = 0.0090 = 89%; close at top
        let c = candle("1.1000", "1.1095", "1.1005", "1.1080");
        let side = momentum_side(&c, "0.6".parse().unwrap(), "0.8".parse().unwrap());
        assert_eq!(side, Some(Side::Long));
    }

    #[test]
    fn momentum_bearish_detected() {
        let c = candle("1.1080", "1.1085", "1.0995", "1.1005");
        let side = momentum_side(&c, "0.6".parse().unwrap(), "0.8".parse().unwrap());
        assert_eq!(side, Some(Side::Short));
    }

    #[test]
    fn small_body_rejected() {
        // doji — body is tiny
        let c = candle("1.1040", "1.1090", "1.1000", "1.1045");
        let side = momentum_side(&c, "0.6".parse().unwrap(), "0.8".parse().unwrap());
        assert_eq!(side, None);
    }

    #[test]
    fn bullish_fvg_detected() {
        let pre     = candle("1.1000", "1.1020", "1.0990", "1.1010");
        let impulse = candle("1.1010", "1.1110", "1.1005", "1.1100"); // big bull
        let post    = candle("1.1090", "1.1130", "1.1070", "1.1120"); // low > pre.high

        let min_zone: Decimal = "0.0001".parse().unwrap(); // 1 pip min — zone is 50 pips, passes
        let fvg = detect(&pre, &impulse, &post, "0.6".parse().unwrap(), "0.8".parse().unwrap(), min_zone, 10, 5);
        assert!(fvg.is_some());
        let fvg = fvg.unwrap();
        assert_eq!(fvg.side, Side::Long);
        assert_eq!(fvg.zone_low,  "1.1020".parse::<Decimal>().unwrap()); // pre.high
        assert_eq!(fvg.zone_high, "1.1070".parse::<Decimal>().unwrap()); // post.low
    }

    #[test]
    fn no_gap_returns_none() {
        // post.low <= pre.high — no gap
        let pre     = candle("1.1000", "1.1060", "1.0990", "1.1050");
        let impulse = candle("1.1050", "1.1110", "1.1040", "1.1100");
        let post    = candle("1.1090", "1.1130", "1.1055", "1.1120"); // post.low=1.1055 < pre.high=1.1060

        let min_zone: Decimal = "0.0001".parse().unwrap();
        let fvg = detect(&pre, &impulse, &post, "0.6".parse().unwrap(), "0.8".parse().unwrap(), min_zone, 10, 5);
        assert!(fvg.is_none());
    }
}
