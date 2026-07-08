# 🧭 Technical Analysis Indicator Reference & AI Rulebook

**Detailed human-readable guides**: See `docs/indicators/` for in-depth documentation per indicator (RSI divergence spotting, chart annotations, confirmation walkthroughs).

This condensed reference is loaded by the LLM engine. It provides the minimum mathematical rules and thresholds needed for AI agent reasoning. Keep sections dense — avoid verbose background to minimize API token costs.

---

## 1. Relative Strength Index (RSI - 14)

### Description

RSI measures the speed and change of price movements using Wilder's smoothing
method. It oscillates between 0 and 100. Detailed divergence spotting rules
are in `docs/indicators/rsi.md`.

### AI Input Schema

```json
{
  "rsi_value": 58.4,
  "rsi_divergence_status": "none | potential_bullish | potential_bearish | confirmed_bullish | confirmed_bearish",
  "recent_closes": [3120.0, 3122.5, 3124.0, 3123.5]
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `rsi_value` is above 50 and rising, OR `rsi_value` has crossed back
    above 30 from oversold territory.
  - Description: The momentum is building upwards, favoring longs.
- **BEARISH**
  - Rule: `rsi_value` is below 50 and falling, OR `rsi_value` has crossed back
    below 70 from overbought territory.
  - Description: Bearish momentum is dominant, favoring shorts.
- **SIDEWAYS**
  - Rule: `rsi_value` oscillates tightly between 45 and 55 with no clear
    directional slope over the last 10 intervals.
  - Description: Price is in a range, showing low directional momentum.

### Divergence Status Rules

- **Potential Bullish**: Price makes Lower Low, RSI makes Higher Low. Signal is UNCONFIRMED.
- **Confirmed Bullish**: Potential bullish + candle close breaks BELOW the nearest active Support level (S₁/S₂) with a 0.2% tolerance buffer.
- **Potential Bearish**: Price makes Higher High, RSI makes Lower High. Signal is UNCONFIRMED.
- **Confirmed Bearish**: Potential bearish + candle close breaks ABOVE the nearest active Resistance level (R₁/R₂) with a 0.2% tolerance buffer.
- DO NOT treat a potential divergence as a confirmed signal. Reference it only as secondary confluence unless the structural S/R break has occurred.
- In strong trending regimes (ADX > 30), shift overbought to 80 and oversold to 20 to avoid premature reversal signals.

---

## 2. MACD (12, 26, 9)

### Description

Moving Average Convergence Divergence tracks the relationship between two moving
averages of the asset's price to determine trend strength and momentum shifts.
Detailed histogram divergence rules are in `docs/indicators/macd.md`.

### AI Input Schema

```json
{
  "macd_line": 1.25,
  "signal_line": 0.95,
  "histogram_value": 0.30,
  "histogram_trend": "accelerating | decelerating",
  "histogram_peak": 0.45,
  "macd_divergence_status": "none | potential_bullish | potential_bearish | confirmed_bullish | confirmed_bearish",
  "crossover_detected": false,
  "crossover_direction": "BULLISH | BEARISH"
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `macd_line` is above the `signal_line` (bullish crossover) AND `macd_line` is below zero (negative territory). Histogram positive and expanding.
  - Description: Valid bullish momentum reversal from oversold conditions.
- **BEARISH**
  - Rule: `macd_line` is below the `signal_line` (bearish crossover) AND `macd_line` is above zero (positive territory). Histogram negative and expanding.
  - Description: Valid bearish momentum reversal from overbought conditions.
- **SIDEWAYS**
  - Rule: Both `macd_line` and `signal_line` are flatly converged near the zero line, with negligible histogram oscillation. OR a crossover occurred but does not meet zero-line filter criteria.
  - Description: Lines have flattened or crossover is unfiltered.

### Zero-Line Filter Rules (CRITICAL)

- **Bullish crossover below zero** (macd_line < 0): Valid reversal entry from deep oversold.
- **Bullish crossover above zero** (macd_line > 0): REJECT — momentum continuation, not reversal. Poor risk/reward for fresh entry.
- **Bearish crossover above zero** (macd_line > 0): Valid reversal entry from overbought distribution.
- **Bearish crossover below zero** (macd_line < 0): REJECT — momentum continuation, not reversal.
- **Extreme High Rejection**: A bullish crossover at extreme positive values (above macd_extreme_high_threshold, default +1000 or scaled to asset price) signals late-stage FOMO / liquidation spikes. Flag as high-risk and REJECT as a valid entry setup.

### Histogram Contraction Exit Rule

- Track the `histogram_peak` (maximum histogram value since the last crossover or trade entry).
- If `current_histogram < histogram_peak × (1 − contraction_threshold)`, momentum has decelerated. Recommend early position close BEFORE an opposite crossover prints. Default contraction threshold = 0.30 (30%).
- Contracting histograms signal trend exhaustion even when the MACD line remains above signal.

### Divergence Status Rules

- **Potential Bullish**: Price makes Lower Low, MACD histogram makes Higher Low. UNCONFIRMED until structural S/R break.
- **Confirmed Bullish**: Potential + candle close breaks below active Support with 0.2% tolerance.
- **Potential Bearish**: Price makes Higher High, MACD histogram makes Lower High. UNCONFIRMED until structural S/R break.
- **Confirmed Bearish**: Potential + candle close breaks above active Resistance with 0.2% tolerance.

### Confluence Requirements

- Bullish crossover + RSI crossing above 30 (exiting oversold) OR Confirmed Bullish RSI Divergence → bonus confidence.
- Bullish crossover + price breaking/holding Support (S₁/S₂) → entry valid. Without S/R hold → restrict to Wait.
- Entry direction must align with Macro (15m) trend: bullish entry requires price > 200 EMA on macro chart.

---

## 3. Squeeze Momentum (John Carter / LazyBear)

### Description

Squeeze Momentum tracks Bollinger Bands compression relative to Keltner Channels
to identify volatility compression (Squeeze ON) and explosive momentum releases
(Squeeze OFF). Detailed mechanics in `docs/indicators/squeeze_momentum.md`.

### AI Input Schema

```json
{
  "squeeze_on": true,
  "momentum_value": 0.045,
  "squeeze_duration": 7,
  "squeeze_release_trigger": false,
  "momentum_direction": "BullishAcceleration"
}
```

### Momentum Direction Phases (CRITICAL)

| Phase | Above/Below Zero | Bar Size Relative to Prior | Action |
|-------|-----------------|---------------------------|--------|
| **BullishAcceleration** | Above zero | Growing (light green) | Enter/Hold Long |
| **BullishDeceleration** | Above zero | Shrinking (dark green) | EXIT Long immediately |
| **BearishAcceleration** | Below zero | Growing more negative (dark red) | Enter/Hold Short |
| **BearishDeceleration** | Below zero | Shrinking (bright red) | EXIT Short immediately |
| **Flat** | Near zero | Stagnant gray | No action |

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `squeeze_release_trigger` is true (just released from coiling) AND `momentum_direction` is `BullishAcceleration` AND `squeeze_duration` >= minimum (default 5 bars). OR squeeze is OFF and momentum is BullishAcceleration (holding phase).
  - Description: Validated volatility breakout to the upside with accelerating momentum.
- **BEARISH**
  - Rule: `squeeze_release_trigger` is true AND `momentum_direction` is `BearishAcceleration` AND `squeeze_duration` >= minimum. OR squeeze is OFF and momentum is BearishAcceleration.
  - Description: Validated volatility breakout to the downside with accelerating momentum.
- **SIDEWAYS**
  - Rule: `squeeze_on` is true (compression state — energy coiling, no trade). OR `squeeze_release_trigger` is true but `squeeze_duration` < minimum ("Premature Breakout" — reject). OR momentum direction is Flat.
  - Description: Coiled compression, premature breakout, or no directional impulse.

### Release Trigger & Duration Gate

- Entry ONLY on the first candle after squeeze_release_trigger fires (dot changes from red to green).
- Reject breakouts where `squeeze_duration < squeeze_min_duration` (default 5) — these are head fakes.
- Longer squeeze duration (>8 bars) = more violent breakout, higher allocation confidence.

### Deceleration Exit Rule

- When momentum direction shifts from `Acceleration` to `Deceleration` while holding a position, exit immediately. Do NOT wait for a crossover — the histogram contraction is a leading signal of momentum exhaustion.

---

## 4. ADX (Average Directional Index - 14)

### Description

ADX quantifies trend strength without regard to trend direction, while +DI and
-DI define the prevailing direction. Detailed regime mechanics in `docs/indicators/adx.md`.

### AI Input Schema

```json
{
  "adx_line": 28.5,
  "di_plus": 24.2,
  "di_minus": 15.1,
  "adx_slope": 1.2,
  "adx_regime": "strong",
  "di_crossover_detected": false,
  "di_crossover_direction": "NONE"
}
```

### Trend Strength Regimes (CRITICAL)

| Regime | ADX Range | Action |
|--------|-----------|--------|
| **Congestion** | < 20 | BLOCK all trend-following entries. Return SIDEWAYS. |
| **Emerging** | 20–25 | Entries allowed at REDUCED allocation. Validate with slope. |
| **Strong** | 25–40 | MAX confidence. Full allocation trend-following. |
| **Extreme / Exhaustion** | > 40 | BLOCK new entries. Monitor open trades for Hard Hook exit. |

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `di_plus` is above `di_minus` (or bullish DI crossover detected) AND `adx_line` > 20 AND `adx_slope` > 0 (positive, accelerating). ADX must be in EMERGING or STRONG regime.
  - Description: Validated bullish trend inception with accelerating momentum.
- **BEARISH**
  - Rule: `di_minus` is above `di_plus` (or bearish DI crossover detected) AND `adx_line` > 20 AND `adx_slope` > 0. ADX must be in EMERGING or STRONG regime.
  - Description: Validated bearish trend inception with accelerating momentum.
- **SIDEWAYS**
  - Rule: `adx_line` < 20 (Congestion regime) regardless of DI crosses, OR `adx_line` > 40 (Extreme regime), OR `adx_slope` ≤ 0 (flat/declining trend strength).
  - Description: Ranging market, exhausted trend, or fading momentum — do not trade.

### Slope Validation Gate

- A DI crossover is only valid when `adx_slope` > 0 (ADX accelerating over last 3 bars).
- Reject any crossover where ADX is flat or declining — this signals a false breakout in fading conditions.
- The slope is computed as `ADX_t - ADX_{t-slope_lookback}` (default lookback = 3).

### Volatility Exhaustion Exit (Hard Hook)

- If ADX > exhaustion threshold (default 40) AND adx_slope turns negative for 2 consecutive periods → trend climax detected. Exit position immediately — do NOT wait for DI crossover.

---

## 5. Bollinger Bands (20, 2) & ATR (14)

### Description

Bollinger Bands plot standard deviation envelope channels while ATR measures
systemic market volatility. Detailed ATR risk mechanics in `docs/indicators/atr.md`.

### AI Input Schema

```json
{
  "mid_price": 3125.0,
  "bb_upper": 3140.0,
  "bb_middle": 3120.0,
  "bb_lower": 3100.0,
  "atr_value": 12.5,
  "atr_slope": 0.3,
  "atr_volatility_regime": "expanding"
}
```

### BB Signal Threshold Matrix

- **BULLISH**
  - Rule: `mid_price` is riding above the `bb_middle` line, closing near the
    `bb_upper` band, with a rising `atr_value`.
  - Description: Expansion volatility is driving price to the upside.
- **BEARISH**
  - Rule: `mid_price` is riding below the `bb_middle` line, closing near the
    `bb_lower` band, with a rising `atr_value`.
  - Description: Volatility-supported breakout to the downside.
- **SIDEWAYS**
  - Rule: Price is bouncing off upper/lower boundaries, failing to sustain
    outside closes, while bands compress and `atr_value` declines.
  - Description: Mean-reverting behavior within contraction envelope.

### ATR Volatility Regime Rules (CRITICAL)

| Regime | Condition | Strategy |
|--------|-----------|----------|
| **Expanding** | ATR > 5-period SMA × 1.02 | Favor breakouts. Wider stops (2.5-3× ATR). Boost Squeeze/ADX breakout confidence. |
| **Contracting** | ATR < 5-period SMA × 0.98 | Favor mean-reversion. Tighter stops (1-1.5× ATR). Penalize breakout signals. |
| **Stable** | Within ±2% of SMA | Standard stops (2× ATR). Either strategy valid. |

### Dynamic Stop-Loss Formula

- **Long**: `SL = Entry − (ATR × Multiplier)` — place stop outside normal noise
- **Short**: `SL = Entry + (ATR × Multiplier)`
- **Position Size**: `(Capital × Risk%) / (ATR × Multiplier)` — keeps risk constant
- **Take Profit**: `Entry ± (ATR × Multiplier × Target R:R)` — scales with volatility
- Default multiplier = 2.0, default target R:R = 2.5

---

## 6. Volume & Price Action (EMAs)

### Description

Evaluates basic market structural health by matching price relative to major
Exponential Moving Averages (10, 50, 100, 200) and volume expansion.
Detailed RVOL mechanics in `docs/indicators/volume.md`.

### AI Input Schema

```json
{
  "close": 3125.0,
  "ema_fast": 3130.0,
  "ema_medium": 3120.0,
  "ema_slow": 3100.0,
  "ema_long": 3080.0,
  "volume": 450.5,
  "average_volume": 320.0,
  "rvol": 1.41,
  "ema_stack_state": "bullish"
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `close` is structured cleanly above all EMAs, which are stacked
    sequentially (`ema_fast` > `ema_medium` > `ema_slow` > `ema_long`), supported by
    volume exceeding `average_volume` AND `rvol` >= 1.0.
  - Description: Pure structural uptrend with high institutional volume commitment.
- **BEARISH**
  - Rule: `close` is structured below all EMAs, which are stacked in reverse
    (`ema_fast` < `ema_medium` < `ema_slow` < `ema_long`), supported by volume
    exceeding `average_volume` AND `rvol` >= 1.0.
  - Description: Structural downtrend on solid distribution volume.
- **SIDEWAYS**
  - Rule: EMAs are tangled, wrapping closely around the current `close`, with a
    flat slope, supported by declining volume below `average_volume` OR `rvol` < 1.0.
  - Description: Flat price distribution with declining network volume.

### EMA Stacking Rules (CRITICAL)

- **Bullish Stack**: `Price > EMA(10) > EMA(50) > EMA(100) > EMA(200)` — Long only. Buy pullbacks to EMA10-50 zone.
- **Bearish Stack**: `Price < EMA(10) < EMA(50) < EMA(100) < EMA(200)` — Short only. Short rallies to EMA10-50 zone.
- **Tangled**: EMAs crossing, flattening, wrapping around price — pause ALL trend-following entries.
- EMA 200 is the ULTIMATE MACRO FILTER: Long rejected if price < EMA200, Short rejected if price > EMA200.
- Structural Invalidation: Close below EMA100/200 on Long → exit. Close above on Short → exit.
- Detailed EMA mechanics in `docs/indicators/ema.md`.

### RVOL Confirmation Gate (CRITICAL)

| Regime | RVOL Range | Action |
|--------|-----------|--------|
| **Consolidation** | < 1.0 | Reject ALL breakout/trend signals. Low participation = high fakeout risk. |
| **Normal** | 1.0 – 1.5 | Standard entry execution at normal allocation. |
| **Institutional** | ≥ 1.5 | REQUIRED to validate S/R breaks, Squeeze releases, MACD crossovers. Scale allocation up. |
| **Exhaustion Climax** | ≥ 3.0 | No new entries. Consider immediate exit. Trend climax approaching. |

- An S/R breakout candle with RVOL < 1.5 is a "head fake" — reject it.
- MACD crossover below zero requires RVOL ≥ 1.2 for validation.
- Squeeze release trigger requires RVOL ≥ 1.5 for validation.
- RVOL ≥ 3.0: tighten existing stops, no new positions.

---

## 7. Volume Weighted Average Price (VWAP)

### Description

VWAP represents the true intraday average price weighted by cumulative execution
volume. Resets daily (86,400s). Detailed mechanics in `docs/indicators/vwap.md`.

### AI Input Schema

```json
{
  "close": 3125.0,
  "vwap": 3122.0,
  "vwap_bias": "premium"
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `close` > `vwap` × 1.001 (premium territory — >0.1% above). OR `vwap_bias` is "premium".
  - Description: Bullish intraday bias. Buyers dominant. Look for VWAP pullback entries during bullish EMA stack.
- **BEARISH**
  - Rule: `close` < `vwap` × 0.999 (discount territory — >0.1% below). OR `vwap_bias` is "discount".
  - Description: Bearish intraday bias. Sellers dominant. Look for VWAP rally shorts during bearish EMA stack.
- **SIDEWAYS**
  - Rule: Price within ±0.1% of VWAP (equilibrium). OR `vwap_bias` is "equilibrium".
  - Description: Value equilibrium. Institutions executing at fair value. Mean-reversion target.

### VWAP Institutional Rules

- VWAP is the primary institutional cost benchmark. Buy below VWAP, sell above VWAP.
- Bullish EMA stack + price > VWAP → pullback to VWAP touch/wick + close above EMA10 = LONG entry.
- Bearish EMA stack + price < VWAP → rally to VWAP touch/wick + close below EMA10 = SHORT entry.
- Ranging regime (tangled EMAs, ADX < 20) → VWAP is the TP₁ mean-reversion target.

---

## 8. Fibonacci Retracements, Extensions & Golden Pocket

### Description

Fibonacci levels (0.236, 0.382, 0.500, 0.618, 0.660, 0.786) are mathematical ratios derived from the Fibonacci sequence. They serve as structural boundaries for pullbacks and extensions.
Detailed mechanics in `docs/indicators/fibonacci.md`.

### Golden Pocket (61.8% – 66.0%)

The zone between 61.8% and 66.0% retracement is the **Golden Pocket** — the highest-probability institutional accumulation/distribution zone:

- **Bullish GP (Long):** Price pulls back from swing high into the 61.8%-66.0% zone → enter Long when RSI/Squeeze confirm reversal.
- **Bearish GP (Short):** Price rallies from swing low into the 61.8%-66.0% zone → enter Short when indicators confirm.
- GP_Top = Anchor ± (D × 0.618), GP_Bottom = Anchor ± (D × 0.660)

### Extension Targets

- **1.618 Extension (TP₁/TP₂):** Primary algorithmic take-profit zone. Sharp pauses/reversals cluster here.
- **2.618 Extension (TP₃):** Ultimate parabolic climax target. Used during high-volatility (expanding ATR) regimes.
- Extensions carry more weight when ATR volatility is EXPANDING.

## 9. BBWP (Bollinger Band Width Percentile)

### Description

BBWP normalizes Bollinger Band width into a percentile (0%-100%) over a 252-bar historical lookback.
Identifies volatility compression (<10% = stored energy, pre-breakout) and exhaustion (>90% = climax, trend flatlining).
Detailed mechanics in `docs/indicators/bbwp.md`.

### Volatility Regimes

| Regime | BBWP Range | Action |
|--------|-----------|--------|
| **Compression (Coiling)** | < 10% | Extreme consolidation. Stored energy. Boost Squeeze release / breakout signals. |
| **Normal** | 10% – 90% | Standard volatility territory. Normal entry rules apply. |
| **Exhaustion (Climax)** | > 90% | Parabolic overextension. Block new entries. Tighten stops on active positions. |

### Confluence Rules
- Squeeze OFF release ONLY valid if BBWP recently registered below 10% (confirms coiled energy).
- Chart pattern breakouts (triangles/wedges) more reliable when BBWP < 15%.

## 10. Chart Patterns (Triangles, Wedges, Channels)

### Description
Chart patterns are detected mathematically using linear regression through pivot highs/lows.
Triangles = converging boundaries. Wedges = same-direction sloping boundaries converging.
Channels = parallel boundaries containing price action. Detailed in `docs/indicators/chart_patterns.md`.

### Pattern Classification
- **Symmetrical Triangle**: Highs descending, lows ascending → neutral breakout setup.
- **Rising Wedge**: Both slopes positive, lower steeper → bearish bias.
- **Falling Wedge**: Both slopes negative, upper steeper → bullish bias.
- **Ascending Channel**: Parallel positive slopes → buy near lower boundary.
- **Descending Channel**: Parallel negative slopes → short near upper boundary.

### Breakout Validation
- Candle close above/below regression line + RVOL ≥ 1.5 = confirmed breakout.
- Without volume confirmation, breakout is a potential fakeout.

## 11. The 8-Factor Point-Scoring & Capital Allocation Protocol

This execution protocol uses the 5-minute chart for execution timing, while deriving its primary trend direction from the 15-minute chart.

### Directional Trend Bias Selection
- **Bullish Bias:** If the 15m trend is bullish, only Long entries are considered.
- **Bearish Bias:** If the 15m trend is bearish, only Short entries are considered.

### Pre-Trade Level Mapping
Before any position is opened, the system must identify and mark exactly 2 Support levels ($S_1, S_2$) and 2 Resistance levels ($R_1, R_2$) on the chart. These coordinates serve as your scaling entries (averaging), take-profit (TP), and stop-loss (SL) points.

### S/R Role-Reversal Rules
- Resistance broken decisively above (>flip_tolerance, default 0.3%) → flipped to Support.
- Support broken decisively below (>flip_tolerance) → flipped to Resistance.
- Flip events are tracked with timestamps and flip counts. Merge protocol preserves flip memory across pivot detection cycles.
- Price within 0.5% of active Support → bullish confluence. Within 0.5% of active Resistance → bearish confluence.
- S/R breakout entries require RVOL ≥ 1.5 (institutional volume confirmation). Low-volume breaks = head fakes.
- Detailed mechanics in `docs/indicators/support_resistance.md`.

### The 8-Factor Point System
The system evaluates eight distinct indicator and structural dimensions, assigning 1 point for each signal that aligns with your trading bias:
1. **RSI (Overbought / Oversold):** Point awarded if RSI is aligned with the bias (oversold for Long, overbought for Short).
2. **MACD Crossovers:** Point awarded if MACD is crossing, rising, or falling in the direction of the bias.
3. **Divergences:** Point awarded if an RSI or MACD divergence is active.
4. **Support Level Alignment:** Point awarded if price is testing or reacting off a support level.
5. **Resistance Level Alignment:** Point awarded if price is testing or reacting off a resistance level.
6. **Trend (15m Macro Chart):** Point awarded if the 15m trend aligns with your trading bias.
7. **200 EMA Position:** Point awarded if price is positioned correctly relative to the 200 EMA (above for Long, below for Short).
8. **Patterns (Triangles, Wedges, Channels):** Point awarded if a structural chart pattern confirms the setup.

### Dynamic Capital Allocation & Leverage
- **Leverage:** Trade exclusively at **20x Cross Leverage**.
- **Portion Size Allocation:** Position sizing is determined dynamically based on the total point score:
  - **Score < 5 Points:** Use a base allocation of **1% of capital**.
  - **Score of 5 to 6 Points:** Use an allocation of **2% of capital**.
  - **Score >= 7 Points:** Use an allocation of **3% of capital**.

### Exit & Stop-Loss Trailing Rules
- **Stop Loss:** Exit the position immediately if the stop-loss is hit.
- **Opposite Signals Exit:** Close the entire position immediately via a market order if **more than 5 opposite signals** are detected (e.g., if holding a Long trade and 6 bearish signals are registered).
- **Break-Even Protection:** The moment Take-Profit 1 ($TP_1$) is hit, the stop-loss for the remaining position is moved to your entry price.

---

## 12. Stochastic Oscillator (%K / %D)

### Description
Momentum oscillator ranking the close within its recent high-low range. Slowed %K
with a %D signal line. **Leading** — identifies overbought/oversold pivots before price
confirms. Class: Leading · Group: Momentum · Directional · Divergence-capable.

### AI Input Schema
- `stochastic.k_line`, `stochastic.d_line` (0–100), `stochastic.normalized` ([-1,1], `(k-50)/50`).

### Signal Threshold Matrix
- `%K >= 80` → OVERBOUGHT_DISTRIBUTION (bearish reversion watch).
- `%K <= 20` → OVERSOLD_ACCUMULATION (bullish reversion watch).
- `%K > %D` → BULLISH_MOMENTUM_ALIGNMENT; `%K < %D` → BEARISH_MOMENTUM_ALIGNMENT.
- Signals: Crossover (%K/%D), Threshold (OB/OS), Divergence.

## 13. Chande Momentum Oscillator (CMO)

### Description
Raw momentum ratio of summed gains vs losses over the lookback, natively bounded
`[-100, 100]`, with no intermediate smoothing. Class: Leading · Group: Momentum ·
Directional · Divergence-capable.

### AI Input Schema
- `chandemo.raw_value` (-100..100), `chandemo.normalized` (`cmo/100`).

### Signal Threshold Matrix
- `>= +50` → CLIMACTIC_BULL_EXHAUSTION; `<= -50` → CLIMACTIC_BEAR_EXHAUSTION.
- `> 0` → EMERGING_BULL_MOMENTUM; `< 0` → EMERGING_BEAR_MOMENTUM.
- Signals: ZeroLineCross, Threshold, Divergence.

## 14. Supertrend

### Description
ATR-based trailing-stop / trend-direction indicator. The line flips sides as trend
reverses. Class: Lagging · Group: Trend · Directional · Render: price overlay.

### AI Input Schema
- `supertrend.line` (price), `supertrend.direction` (+1 up / -1 down), `supertrend.normalized` (dir × distance conviction).

### Signal Threshold Matrix
- `direction = +1` → SUPERTREND_BULLISH (line is trailing support).
- `direction = -1` → SUPERTREND_BEARISH (line is trailing resistance).
- Signals: TrendFlip (on direction change), Crossover (price vs line). Use the line as a dynamic stop.

## 15. Keltner Channels

### Description
EMA middle band ± (multiplier × ATR). Volatility envelope; complements TTM Squeeze
(BB-inside-KC). Class: Lagging · Group: Trend · Directional · Render: price overlay.

### AI Input Schema
- `keltner.upper`, `keltner.middle`, `keltner.lower` (prices), `keltner.normalized` (price position within/beyond channel).

### Signal Threshold Matrix
- `price >= upper` → KELTNER_UPPER_BREAKOUT (+1). `price <= lower` → KELTNER_LOWER_BREAKOUT (-1).
- Between bands → scaled position (KELTNER_UPPER_HALF / KELTNER_LOWER_HALF).
- Signals: Breakout, BandTouch.

## 16. Donchian Channels

### Description
Highest-high / lowest-low over N bars (Turtle breakout system). Class: Lagging ·
Group: Trend · Directional · Render: price overlay.

### AI Input Schema
- `donchian.upper`, `donchian.middle`, `donchian.lower` (prices), `donchian.normalized`.

### Signal Threshold Matrix
- `price >= upper` → DONCHIAN_UPPER_BREAKOUT (+1). `price <= lower` → DONCHIAN_LOWER_BREAKOUT (-1).
- Signals: Breakout, BandTouch. Breakouts require RVOL confirmation before acting.

## 17. On-Balance Volume (OBV)

### Description
Running cumulative volume signed by close direction; detects accumulation/distribution
before price. Normalized off its slope vs a smoothed baseline (unbounded raw). Class:
Lagging · Group: Volume · Directional · Divergence-capable.

### AI Input Schema
- `obv.raw_value` (cumulative), `obv.values.obv_sma` (smoothed), `obv.normalized` (slope, tanh-scaled).

### Signal Threshold Matrix
- `obv > obv_sma` → OBV_ACCUMULATION; `obv < obv_sma` → OBV_DISTRIBUTION.
- Signals: Divergence (price/OBV), TrendFlip (slope). OBV divergence is a strong early reversal cue.

## 18. Chaikin Money Flow (CMF)

### Description
Volume-weighted accumulation/distribution over N bars, natively `[-1, 1]`. Class:
Hybrid · Group: Volume · Directional · Divergence-capable.

### AI Input Schema
- `cmf.raw_value` (-1..1), `cmf.normalized` (amplified ×3, clamped).

### Signal Threshold Matrix
- `>= +0.20` → CMF_STRONG_BUYING; `+0.05..0.20` → CMF_BUYING_PRESSURE.
- `<= -0.20` → CMF_STRONG_SELLING; `-0.20..-0.05` → CMF_SELLING_PRESSURE; else NEUTRAL_FLOW.
- Signals: ZeroLineCross, Divergence.

## 19. Money Flow Index (MFI)

### Description
Volume-weighted RSI over N bars, bounded `[0, 100]`. Class: Hybrid · Group: Volume ·
Directional · Divergence-capable.

### AI Input Schema
- `mfi.raw_value` (0..100), `mfi.normalized` (RSI-style mapping).

### Signal Threshold Matrix
- `>= 80` → MFI_OVERBOUGHT_DISTRIBUTION; `<= 20` → MFI_OVERSOLD_ACCUMULATION.
- `>= 50` → MFI_BULLISH_FLOW; `< 50` → MFI_BEARISH_FLOW.
- Signals: Threshold (OB/OS), Divergence.

## 20. Historical Volatility (HV)

### Description
Annualized standard deviation of log returns over N bars (statistical volatility,
distinct from ATR's average range). **Non-directional** — used as a volatility gate,
never a directional score. Class: Lagging · Group: Volatility · Gate.

### AI Input Schema
- `hv.raw_value` (annualized %), `hv.normalized` (0.0 — non-directional).

### Signal Threshold Matrix
- `>= 100%` → EXTREME_VOLATILITY; `>= 60%` → HIGH_VOLATILITY; `<= 20%` → LOW_VOLATILITY; else NORMAL_VOLATILITY.
- Use to widen stops / down-size in high-HV regimes; no directional bias.

## 21. Aroon

### Description
Measures the number of periods since the highest high / lowest low over the window,
signalling trend emergence vs consolidation. Aroon Oscillator = Up − Down ∈ [-100, 100].
Class: Hybrid · Group: Market Regime · Directional.

### AI Input Schema
- `aroon.values.up`, `aroon.values.down` (0–100), `aroon.raw_value` (oscillator), `aroon.normalized` (`osc/100`).

### Signal Threshold Matrix
- `Up >= 70 & Down <= 30` → AROON_STRONG_UPTREND; mirror → AROON_STRONG_DOWNTREND.
- `osc > 0` → AROON_BULLISH_BIAS; `< 0` → AROON_BEARISH_BIAS; else CONSOLIDATION.
- Signals: Crossover (Up/Down), Threshold (>70 strong), TrendFlip.

## 22. Choppiness Index

### Description
Quantifies whether the market is trending (low) or ranging/choppy (high) over N bars.
Bounded `[0, 100]`. **Non-directional** — a regime gate that dampens directional
conviction when high. Class: Hybrid · Group: Market Regime · Gate.

### AI Input Schema
- `choppiness.raw_value` (0–100), `choppiness.normalized` (0.0 — non-directional).

### Signal Threshold Matrix
- `>= 61.8` → CHOP_CONSOLIDATION_RANGE (avoid trend entries, expect chop/mean-reversion).
- `<= 38.2` → CHOP_STRONG_TREND (trend-following favored). Else CHOP_TRANSITIONAL.
- Use as a confidence multiplier on the directional score.

## 23. Linear Regression Slope

### Description
Slope of the least-squares regression line over the last N closes, scaled by price into
a per-bar percentage. Positive = uptrend, negative = downtrend. Class: Lagging · Group:
Market Regime · Directional.

### AI Input Schema
- `linreg_slope.raw_value` (slope, price/bar), `linreg_slope.normalized` (tanh of %/bar).

### Signal Threshold Matrix
- `normalized > 0.1` → LINREG_RISING_TREND; `< -0.1` → LINREG_FALLING_TREND; else FLAT.
- Signals: ZeroLineCross (trend flip), Threshold (steep slope).

## 24. Z-Score

### Description
Number of standard deviations the close sits from its N-bar mean. **Mean-reversion**
oriented — a stretched-high reading is bearish (distribution), stretched-low is bullish
(accumulation). Class: Leading · Group: Market Regime · Directional.

### AI Input Schema
- `zscore.raw_value` (σ from mean), `zscore.normalized` (`clamp(-z/3)`, mean-reversion sign).

### Signal Threshold Matrix
- `>= +2` → ZSCORE_OVEREXTENDED_HIGH (fade / expect pullback).
- `<= -2` → ZSCORE_OVEREXTENDED_LOW (expect bounce). Else ABOVE/BELOW/AT mean.
- Signals: Threshold (±2/±3 extremes), ZeroLineCross (mean).

---

## 25. Meta-Intelligence Layer

Built on top of the 29 indicators (not additional indicators). All are backend-authoritative
and surfaced in the **Terminal Monitor** tab.

### Indicator Confidence
Every `NormalizedIndicatorValue` carries `confidence ∈ [0,1]`: base = `|normalized|`, boosted
by confirmed/active discrete signals. Displayed as `N%` in the telemetry matrix.

### Signal Age / Freshness
Every `IndicatorSignal` carries `age_bars` (completed bars since first appearance; 0 = fresh).
Resets on first appearance or direction flip. Fresher signals are stronger.

### Market Context (`MarketContext`)
Per-snapshot synthesis attached as `MarketSnapshot.context`: `trend`, `momentum`, `volatility`,
`volume`, `liquidity` (each `{score, confidence, label}`) + `regime`
(TRENDING | RANGE | EXPANSION | COMPRESSION) + `overall_score` [-100,100] + `overall_label`.

### Multi-Timeframe Confirmation
`GET /api/monitor?symbol=` returns a per-indicator agreement matrix across
micro/fast/slow/macro plus a trend-agreement %, structural trend, and per-timeframe
confluence + market context.

### Regime-Aware Weighting
`calculate_registry_confluence` multiplies each indicator's weight by the active regime's
multiplier (`ScoringConfig.regime_weight_multipliers`): trending regimes favor
trend/breakout indicators; ranging regimes favor mean-reversion oscillators.

### Cross-Indicator Confluence
`Σ(weight × normalized)` over enabled directional indicators ÷ active weight, dampened by
non-directional gates (Choppiness/ADX) — configurable per-indicator weight/enable via the
  Scoring Weights settings panel and `POST /api/config/scoring-weights`.

---

## 26. CCI (Commodity Channel Index — 20)

### Description

Measures deviation of typical price from statistical mean. Oscillates around zero.

### AI Input Schema

```json
{
  "cci_value": 125.4,
  "cci_normalized": -0.78,
  "state_label": "CCI_OVERBOUGHT | CCI_OVERSOLD | CCI_CLIMACTIC_BULL_EXHAUSTION | etc"
}
```

### Signal Rules

- **OVERBOUGHT (≥ +100)**: Reading above +100 warns of overbought conditions. Do not enter longs here — wait for reversion below +100. At ≥ +200, climactic exhaustion is probable (Threshold signal, Bearish).
- **OVERSOLD (≤ −100)**: Reading below −100 warns of oversold conditions. Do not enter shorts here — wait for reversion above −100. At ≤ −200, climactic exhaustion is probable (Threshold signal, Bullish).
- **NEUTRAL (−100 to +100)**: CCI in normal range — no extreme signal. Directional bias from sign alone is weak. Look for zero-cross confirmation.
- Use CCI alongside RSI and Stochastic for multi-oscillator confluence. CCI is faster than RSI for detecting cyclical turns.

---

## 27. Williams %R (14)

### Description

Measures close relative to highest-high range on [-100,0] scale. Inverse of Fast Stochastic.

### Signal Rules

- **OVERBOUGHT (≥ −20)**: Above −20 → price near top of range — bearish. Do not buy at extremes (Threshold signal, Bearish).
- **OVERSOLD (≤ −80)**: Below −80 → price near bottom of range — bullish. Do not sell at extremes (Threshold signal, Bullish).
- **MIDLINE (−50)**: %R crossing −50 = momentum flip (ZeroLineCross). Combine with RSI 50-cross for confirmation.

---

## 28. Awesome Oscillator (AO)

### Description

SMA(5)-SMA(34) of median price. Bill Williams' momentum indicator.

### Signal Rules

- **ZERO CROSS (AO crosses 0)**: Bullish momentum flip when AO goes from negative to positive (ZeroLineCross, Bullish). Bearish when positive to negative.
- **TWIN PEAKS**: Bullish twin peaks = AO makes a higher low (second trough above first) while still negative — bullish divergence. Bearish twin peaks = lower high while positive.
- **SAUCER**: AO bars change from red to green while AO is above zero and price is above the cloud — continuation confirmation.

---

## 29. Force Index (13)

### Description

(Close − PrevClose) × Volume, smoothed by EMA(13). Elder's volume×momentum.

### Signal Rules

- **ZERO CROSS**: FI crossing from negative to positive = money flowing in (ZeroLineCross, Bullish). Positive to negative = money flowing out (Bearish).
- **EXTREME READINGS**: Large |FI| with price direction = confirmed institutional flow. Divergence between FI direction and price direction warns of trend exhaustion.
- FI is strongest when confirming price trends: rising FI + rising price = healthy uptrend; falling FI + rising price = bearish divergence.

---

## 30. Hull MA (16)

### Description

Near-zero-lag WMA designed to reduce lag while maintaining smoothness. Price overlay.

### Signal Rules

- **CROSSOVER**: Price crossing above HMA = bullish entry (Crossover signal). Price crossing below = bearish exit.
- HMA is a chart overlay with normalized=0.0 (does not influence confluence scoring). Use it as a trend reference for entry/exit timing, not for directional conviction.

---

## 31. StdDev Channel (20)

### Description

Linear regression centerline ±2σ bands. Trend-aware volatility envelope.

### Signal Rules

- **UPPER BREAKOUT**: Price ≥ upper band = bullish breakout (Breakout signal).
- **LOWER BREAKOUT**: Price ≤ lower band = bearish breakout (Breakout signal).
- **BAND TOUCH**: Price near band edge (±15% of band range) without breaking out = mean-reversion signal (BandTouch signal). Use in range markets; ignore in strong trends.
- **CENTERLINE**: Price crossing the regression centerline = trend direction confirmation.

---

## 32. PSAR (Parabolic SAR) — AF 0.02→0.20

### Description

Trailing stop-loss dot. Below price in uptrend, above in downtrend. Flips on reversal.

### Signal Rules

- **TRENDFLIP**: SAR dot flips from below to above price = trend changed from bullish to bearish. This IS the primary exit signal (TrendFlip signal).
- **CROSSOVER**: Price crosses the SAR line (distinct from the dot flip itself). Price crossing above SAR = bullish; below = bearish (Crossover signal).
- **DISTANCE**: Large distance between price and SAR = strong trend. SAR closing in on price = trend weakening — prepare for potential flip.

---

## 33. Volume Profile (OHLCV-based)

### Description

100-bar rolling window; 30-bin volume distribution. POC/VAH/VAL/HVN/LVN from OHLCV.

### Signal Rules

- **ABOVE VAH**: Price trading above Value Area High = bullish breakout (Breakout signal). The market has moved outside the previous session's accepted value.
- **BELOW VAL**: Price trading below Value Area Low = bearish breakdown (Breakout signal).
- **POC RETEST**: Price returning to the Point of Control from either direction = potential support/resistance reaction (LevelTest signal). POC acts as a magnet level.
- **VALUE ACCEPTANCE/REJECTION**: Price inside value area = acceptance (equilibrium, trending less likely). Price closing outside after being inside = rejection.

---

## 34. Anchored VWAP

### Description

Multi-session VWAP with daily/weekly/monthly/swing anchors. Closest to price = active anchor.

### Signal Rules

- **CROSSOVER**: Price crossing the active anchored VWAP = directional entry (Crossover signal).
- **DISCOUNT ZONE**: Price significantly below the active anchor = bullish (LevelTest signal). Price at discount to institutional fair value.
- **PREMIUM ZONE**: Price significantly above the active anchor = bearish (LevelTest signal). Price at premium to institutional fair value.
- **MULTI-ANCHOR**: Price above ALL anchors = strong bullish conviction. Price below ALL = strong bearish. Between anchors = neutral/uncertain.

---

## 35. Smart Money Concepts (4 indicators: smc_structure, smc_liquidity, smc_fvg, smc_order_blocks)

### Description

OHLCV-based institutional order-flow analysis. BOS/CHoCH for market structure. Liquidity sweeps for stop-hunting. Fair Value Gaps for imbalance zones. Order Blocks for institutional supply/demand zones.

### Signal Rules — SMC Structure

- **BOS (Break of Structure)**: Higher high forming after lower highs = bullish BOS (Breakout signal). Lower low after higher lows = bearish BOS. BOS confirms the current trend structure.
- **CHoCH (Change of Character)**: Lower high after higher high in a bull trend = bearish CHoCH (TrendFlip signal). Higher low after lower low in a bear trend = bullish CHoCH. CHoCH warns of a potential trend reversal or range shift.

### Signal Rules — SMC Liquidity

- **BUY-SIDE SWEEP**: Price wicks below a recent swing low, then closes ABOVE it = long stops hunted, bullish reversal expected (PatternForming signal, Bullish). The sweep confirms the low was "taken."
- **SELL-SIDE SWEEP**: Price wicks above a recent swing high, then closes BELOW it = short stops hunted, bearish reversal expected (PatternForming signal, Bearish).

### Signal Rules — SMC FVG

- **BULLISH FVG**: 3-candle gap where Low[3] > High[1] = bullish imbalance (LevelTest signal). The gap zone acts as a magnet — expect price to return and fill.
- **BEARISH FVG**: 3-candle gap where High[3] < Low[1] = bearish imbalance. Same magnet behavior downward.
- **MITIGATED FVG**: Price has traded through the gap = the imbalance is resolved. A mitigated bullish FVG that later holds as support = inverse FVG (structural support formed).

### Signal Rules — SMC Order Blocks

- **BULLISH OB**: Last bearish candle before a bullish BOS = demand zone (LevelTest signal when tested, Bullish). Price returning to this zone is a potential long entry.
- **BEARISH OB**: Last bullish candle before a bearish BOS = supply zone (LevelTest signal when tested, Bearish). Price returning here is a potential short entry.
- **MITIGATED OB**: Price closes beyond the OB zone = the block is broken (TrendFlip signal if role inverts). A mitigated bullish OB becomes potential resistance; a mitigated bearish OB becomes potential support.

---

## 36. DecisionContext

### Description

Read-only quantitative decision-support metrics computed from the full 51-indicator map. 17 fields covering probability, consensus, expected ranges, forward-looking volatility, risk, quality, reward/risk, stop recommendation, regime confidence, trend persistence, and trade readiness. No new indicators — it reads what already exists.

### AI Input Schema

```json
{
  "decision_context": {
    "bullish_probability": 0.82,
    "bearish_probability": 0.18,
    "directional_bias": 0.64,
    "consensus": 0.91,
    "expected_range_1bar": 0.008,
    "expected_range_5bar": 0.018,
    "expected_range_20bar": 0.036,
    "expected_volatility": 22.5,
    "confluence": 62.0,
    "risk_level": 0.22,
    "reward_risk_ratio": 2.8,
    "recommended_stop": 48750.0,
    "trade_quality": 0.78,
    "market_quality": 0.85,
    "regime_confidence": 0.92,
    "trend_persistence": 0.73,
    "trade_readiness": 0.81
  }
}
```

### Signal Rules — Directional & Consensus

- **High-conviction bullish**: P(bullish) > 0.80 AND Consensus > 0.85. Standard position sizing. Directional bias strongly positive.
- **High-conviction bearish**: P(bearish) > 0.80 AND Consensus > 0.85. Standard position sizing.
- **Fragmented market**: Consensus < 0.55 regardless of P(bullish). Reduce or avoid directional positions.
- **Dead zone / equilibrium**: P(bullish) ≈ 0.50 AND Consensus < 0.55. Range-bound behavior expected.

### Signal Rules — Risk & Stop

- **High risk**: risk_level > 0.70. Do NOT open new positions. Reduce existing exposure.
- **Low risk**: risk_level < 0.25. Standard sizing acceptable.
- **Stop placement**: Use `recommended_stop` as context-aware stop. It prioritizes institutional levels (order blocks) over statistical levels (ATR). Always returns a value — hierarchical fallback chain.
- **Asymmetric opportunity**: reward_risk_ratio ≥ 3.0. Target is 3× further than stop — favorable.
- **Unfavorable asymmetry**: reward_risk_ratio < 1.0. Stop is wider than target — avoid this setup.

### Signal Rules — Quality

- **Trade Quality vs Market Quality**: They are DISTINCT. Market Quality = "is this market tradable?" (clean trend, low noise). Trade Quality = "is THIS directional setup good?" (confluence, probability, volume, consensus, confirmation).
- **Trade Quality > 0.75**: Strong setup. All factors aligned in one direction.
- **Trade Quality < 0.30**: Weak or no setup. Do not trade directionally.
- **Contradiction penalty**: When SMC CHoCH, liquidity sweeps, or MACD divergence oppose the directional bias, trade_quality is HALVED (×0.5). Example: P(bullish)=0.84 but a bearish CHoCH exists → trade quality drops sharply.

### Signal Rules — Regime & Trend

- **Regime confidence > 0.85**: 6 regime indicators strongly agree on the current regime. Weighted: ADX 25%, Choppiness 25%, Ichimoku 20%, Aroon 15%, Supertrend 10%, EMA 5%.
- **Regime confidence < 0.40**: Transitional market. Avoid directional bets.
- **Trend persistence > 0.70**: 7+ of 9 confirmations support trend continuation. Strong trend.
- **Trend persistence < 0.30**: Trend is weakening. Reduce or exit trend positions. Divergences present or CHoCH active.

### Signal Rules — Expected Ranges & Volatility

- **Expected ranges**: Use 1-bar for immediate noise, 5-bar for swing stop/target, 20-bar for position sizing. Trending regimes produce wider ranges; choppy regimes narrower.
- **Volatility context**: Expected Volatility > 1.5× current HV = regime change likely (squeeze/coil). Adjust sizing.

### Signal Rules — Trade Readiness (Synthesis)

- **ACT (0.75–1.0)**: All systems aligned — low risk, high quality, persistent trend, confident regime. Execute standard sizing.
- **READY (0.50–0.75)**: Favorable conditions. Proceed with caution — some factors may be neutral.
- **PREPARE (0.25–0.50)**: Mixed signals. Wait for improvement or reduce size significantly.
- **WAIT (0.0–0.25)**: Do not act. Risk is high, quality is low, or the market is fragmented.
- Trade Readiness synthesizes 5 factors: 30% trade_quality + 25% (1−risk) + 20% market_quality + 10% regime_confidence + 15% trend_persistence.
