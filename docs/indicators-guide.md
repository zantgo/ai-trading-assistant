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
