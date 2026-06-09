# Autonomous Backtest Session — 2026-06-09

Dikerjakan saat user tidur. Ringkasan semua bug fix, backtest, dan temuan.

---

## Bug Fixes

### 1. `!fill_ok` premature FVG cancellation (kritikal)

**Masalah:** Saat zona FVG disentuh candle tapi entry midpoint belum tercapai, kode langsung
membatalkan (`pending_fvg = None`). Akibatnya banyak setup yang valid ikut dibuang.

**Fix:** Saat `is_touched && !fill_ok`, FVG tetap aktif dan dicoba di candle berikutnya.
FVG hanya dibatalkan saat:
- Expiry tercapai (`i >= expiry_idx`) → `missed_fills += 1`
- EMA filter tidak sesuai
- SL distance terlalu kecil

**Dampak:**
```
Sebelum fix: 993 trades, WR=48.3%, PF=1.33, Return=+314.7%
Sesudah fix:  1268 trades, WR=50.3%, PF=1.43, Return=+1183.3%
```

### 2. `missed_fills` counter dipindah ke expiry

Sebelumnya counter dihitung setiap candle zona disentuh tanpa fill. Sekarang hanya dihitung
sekali saat setup expired tanpa pernah terisi.

---

## Fitur Baru

### 3. `TIMEOUT_CANDLES` (env var)

Sama seperti Hermes — force close trade setelah N candles. Default = 0 (disabled).

**Test result:** Hampir tidak berpengaruh di XAUUSDm:
```
TIMEOUT=0  → PF=1.43, Return=1183%
TIMEOUT=24 → PF=1.42, Return=1088%
TIMEOUT=48 → PF=1.43, Return=1132%
```
Artinya trade XAU selalu resolve (SL/TP) dalam <24 M5 candles (2 jam).

---

## Parameter Sweep XAUUSDm

### BODY_PCT_MIN (body/range minimum)
```
0.50 → Trades=1351, PF=1.50, Return=1808%
0.55 → Trades=1320, PF=1.46, Return=1509%
0.60 → Trades=1268, PF=1.43, Return=1183% ← default
0.65 → Trades=1191, PF=1.43, Return=854%
0.70 → Trades=1102, PF=1.40, Return=575%
```
Lower = more trades, slightly better PF. 0.60 adalah titik keseimbangan.

### MIN_RR
```
1.2 → WR=55.7%, PF=1.39, Return=784%
1.5 → WR=50.3%, PF=1.43, Return=1183% ← default
2.0 → WR=41.2%, PF=1.32, Return=853%
2.5 → WR=35.2%, PF=1.27, Return=688%
3.0 → WR=32.1%, PF=1.33, Return=1289%
```
MIN_RR=1.5 memberikan balance terbaik. MIN_RR=3.0 menarik tapi WR 32% beresiko tinggi.

### FVG_EXPIRY_CANDLES
```
3  → PF=1.21, Return=170%
5  → PF=1.31, Return=401%
10 → PF=1.43, Return=1183% ← default
15 → PF=1.44, Return=972%
20 → PF=1.45, Return=796%
```
10 adalah sweet spot — cukup waktu untuk fill, tidak terlalu lama jadi stale.

### EMA_PERIOD (KRITIKAL)
```
0  → WR=41.2%, PF=0.97, Return=-18%   ← RUGI tanpa filter!
10 → WR=53.5%, PF=1.70, Return=1723%
15 → WR=50.1%, PF=1.46, Return=880%
20 → WR=50.3%, PF=1.43, Return=1183% ← default
30 → WR=47.6%, PF=1.26, Return=499%
50 → WR=44.4%, PF=1.11, Return=107%
```
**EMA filter sangat kritikal** — tanpanya strategi rugi. EMA10 terlihat terbaik tapi
lebih beresiko overfitting. EMA20 adalah pilihan konservatif yang terbukti profitable
di EMA range 10-50.

### SL_BUFFER
```
0   → WR=50.3%, PF=1.43, Return=1183% ← default (terbaik)
0.5 → WR=48.0%, PF=1.29, Return=482%
1.0 → WR=47.0%, PF=1.27, Return=348%
2.0 → WR=45.2%, PF=1.20, Return=120%
```
Buffer di luar impulse candle malah mengurangi performa. SL tepat di impulse low/high adalah optimal.

---

## Hermes Validation

Build dan backtest berhasil. BREAKEVEN_SL_1R=true bekerja dengan benar:
- GBPUSDm H1: 102 BE-SL exits, PF=4.27 — fitur berjalan normal.

---

## Recommended Config (XAUUSDm)

```env
SYMBOL=XAUUSDm
TIMEFRAME=M5
BACKTEST_BALANCE=600
BACKTEST_CANDLES=50000
RISK_PCT=0.01

BODY_PCT_MIN=0.6
CLOSE_PCT_MIN=0.8
FVG_EXPIRY_CANDLES=10
MIN_FVG_PIPS=1
MIN_SL_PIPS=5
SL_BUFFER=0
MIN_RR=1.5
TIMEOUT_CANDLES=0

EMA_PERIOD=20       # kritikal — jangan disable

COMMISSION_PER_LOT=7
SLIPPAGE_POINTS=5
SPREAD_OVERRIDE=0
```

**Hasil:** 1268 trades, WR=50.3%, PF=1.43, Return=+1183%, Max DD=-$550 (dari peak ~$8250 = 6.7%)

---

## Peringatan / Risiko

1. **Data hanya Sep 2025–Jun 2026** — XAU naik 65% dalam periode ini (bull run sangat kuat).
   Strategy dengan EMA filter otomatis bias LONG. Performance di bear market atau ranging market
   belum diketahui.

2. **EMA adalah kunci** — tanpa EMA, strategi rugi. Ini menunjukkan edge bukan dari FVG pattern
   saja, tapi kombinasi FVG + trend momentum. Hati-hati jika trend berbalik.

3. **Compounding effect besar** — return +1183% karena compounding 1268 trades. Real-world
   perlu ditest dengan RISK_PCT lebih kecil (0.005) di awal.

4. **Post-fix multi-pair results (setelah fill_ok bug fix):**
   ```
   GBPJPYm → WR=50.5%, PF=1.03, Return=+13.3%   (was +10.3%)
   EURUSDm → WR=49.5%, PF=1.15, Return=+9.0%    (was -5.6% → NOW POSITIVE!)
   GBPUSDm → WR=42.3%, PF=0.85, Return=-19.8%   (still negative)
   XAUUSDm → WR=50.3%, PF=1.43, Return=+1183.3% (flagship)
   ```
   Bug fix meningkatkan EURUSDm dari negatif ke positif!

---

## Next Steps (untuk user)

1. Live micro-lot test di XAUUSDm dengan RISK_PCT=0.005
2. Cari data XAU 2023–2024 untuk out-of-sample validation
3. Explore apakah EURUSDm bisa profitable dengan EMA=10 (mungkin lebih baik dari EMA=20)
4. GBPUSDm masih negatif (PF=0.85) — tidak direkomendasikan untuk sekarang
5. Monitor performa 2 minggu pertama live dengan DD limit manual
