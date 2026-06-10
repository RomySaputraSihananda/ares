use domain::Side;
use rust_decimal::Decimal;

pub fn rolling_ema(prices: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    let k = Decimal::from(2u32) / Decimal::from((period + 1) as u32);
    let mut out = vec![None; prices.len()];
    if prices.len() < period { return out; }
    let seed: Decimal = prices[..period].iter().sum::<Decimal>() / Decimal::from(period);
    out[period - 1] = Some(seed);
    let mut ema = seed;
    for i in period..prices.len() {
        ema = prices[i] * k + ema * (Decimal::ONE - k);
        out[i] = Some(ema);
    }
    out
}

pub fn size_position(
    balance:       Decimal,
    risk_pct:      Decimal,
    sl_distance:   Decimal,
    value_per_lot: Decimal,
    vol_step:      Decimal,
    min_vol:       Decimal,
    max_vol:       Decimal,
) -> Option<Decimal> {
    if sl_distance == Decimal::ZERO { return None; }
    let raw    = (balance * risk_pct) / (sl_distance * value_per_lot);
    let volume = (raw / vol_step).floor() * vol_step;
    if volume < min_vol { return None; }
    Some(volume.min(max_vol))
}

pub fn actual_entry(side: Side, level: Decimal, spread: Decimal) -> Decimal {
    match side {
        Side::Long  => level + spread,
        Side::Short => level,
    }
}

pub fn actual_exit(side: Side, level: Decimal, is_sl: bool, spread: Decimal, slip: Decimal) -> Decimal {
    match (side, is_sl) {
        (Side::Long,  false) => level,
        (Side::Long,  true)  => level - slip,
        (Side::Short, false) => level + spread,
        (Side::Short, true)  => level + spread + slip,
    }
}

pub fn fmt_price(d: Decimal, prec: usize) -> String { format!("{0:.1$}", d, prec) }

pub fn fmt_pnl(pnl: Decimal) -> String {
    if pnl >= Decimal::ZERO { format!("+{:.2}", pnl) } else { format!("{:.2}", pnl) }
}

pub fn d2f(d: Decimal) -> f64 {
    d.to_string().parse().unwrap_or(0.0)
}
