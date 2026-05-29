# 🧭 Technical Analysis Indicator Reference & AI Rulebook

This reference manual documents how individual indicators are computed and when
they are classified as Bullish, Bearish, or Sideways. The orchestrator AI refers
to these rulesets to analyze telemetry.

---

## 1. Relative Strength Index (RSI - 14)

### Description

RSI measures the speed and change of price movements using Wilder's smoothing
method. It oscillates between 0 and 100.

### AI Input Schema

```json
{
  "rsi_value": 58.4,
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

---

## 2. MACD (12, 26, 9)

### Description

Moving Average Convergence Divergence tracks the relationship between two moving
averages of the asset's price to determine trend strength and momentum shifts.

### AI Input Schema

```json
{
  "macd_line": 1.25,
  "signal_line": 0.95,
  "histogram_value": 0.30,
  "histogram_trend": "increasing"
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `macd_line` is above the `signal_line` AND `histogram_value` is positive
    and growing (or shrinking bearishness).
  - Description: Golden cross confirmed with expanding positive momentum.
- **BEARISH**
  - Rule: `macd_line` is below the `signal_line` AND `histogram_value` is negative
    and expanding downwards.
  - Description: Death cross confirmed with expanding negative momentum.
- **SIDEWAYS**
  - Rule: Both `macd_line` and `signal_line` are flatly converged near the zero
    line, with negligible histogram oscillation.
  - Description: Lines have flattened, confirming range-bound consolidation.

---

## 3. Squeeze Momentum (John Carter / LazyBear)

### Description

Squeeze Momentum tracks Bollinger Bands compression relative to Keltner Channels
to identify volatility compression (Squeeze ON) and explosive momentum releases
(Squeeze OFF).

### AI Input Schema

```json
{
  "squeeze_on": true,
  "momentum_value": 0.045,
  "momentum_trend": "rising"
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `squeeze_on` is false (Bands are expanding outside channels) AND
    `momentum_value` is positive and rising (light green), OR `squeeze_on` is
    true but momentum is recovering upward from a negative extreme.
  - Description: Volatility expansion to the upside is active or preparing.
- **BEARISH**
  - Rule: `squeeze_on` is false AND `momentum_value` is negative and falling
    (bright red), OR `squeeze_on` is true but momentum is rolling over downward.
  - Description: Volatility expansion to the downside is active or preparing.
- **SIDEWAYS**
  - Rule: `squeeze_on` is true (compressive dot state) AND `momentum_value` is
    nearly flat (values near zero).
  - Description: Extreme compression phase; trading is paused in anticipation of
    a breakout.

---

## 4. ADX (Average Directional Index - 14)

### Description

ADX quantifies trend strength without regard to trend direction, while +DI and
-DI define the prevailing direction.

### AI Input Schema

```json
{
  "adx_line": 28.5,
  "di_plus": 24.2,
  "di_minus": 15.1
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `di_plus` is above `di_minus` AND `adx_line` is above 20 (and rising).
  - Description: Strong bullish trend is active.
- **BEARISH**
  - Rule: `di_minus` is above `di_plus` AND `adx_line` is above 20 (and rising).
  - Description: Strong bearish trend is active.
- **SIDEWAYS**
  - Rule: `adx_line` is below 20 (regardless of +DI and -DI crosses).
  - Description: Weak, non-trending environment; sideways range-bound market.

---

## 5. Bollinger Bands (20, 2) & ATR (14)

### Description

Bollinger Bands plot standard deviation envelope channels while ATR measures
systemic market volatility.

### AI Input Schema

```json
{
  "mid_price": 3125.0,
  "bb_upper": 3140.0,
  "bb_middle": 3120.0,
  "bb_lower": 3100.0,
  "atr_value": 12.5,
  "atr_trend": "rising"
}
```

### Signal Threshold Matrix

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

---

## 6. Volume & Price Action (EMAs)

### Description

Evaluates basic market structural health by matching price relative to major
Exponential Moving Averages (10, 50, 100, 200) and volume expansion.

### AI Input Schema

```json
{
  "close": 3125.0,
  "ema_fast": 3130.0,
  "ema_medium": 3120.0,
  "ema_slow": 3100.0,
  "ema_long": 3080.0,
  "volume": 450.5,
  "average_volume": 320.0
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `close` is structured cleanly above all EMAs, which are stacked
    sequentially (`ema_fast` > `ema_medium` > `ema_slow` > `ema_long`), supported by
    volume exceeding `average_volume`.
  - Description: Pure structural uptrend with high institutional volume commitment.
- **BEARISH**
  - Rule: `close` is structured below all EMAs, which are stacked in reverse
    (`ema_fast` < `ema_medium` < `ema_slow` < `ema_long`), supported by volume
    exceeding `average_volume`.
  - Description: Structural downtrend on solid distribution volume.
- **SIDEWAYS**
  - Rule: EMAs are tangled, wrapping closely around the current `close`, with a
    flat slope, supported by declining volume below `average_volume`.
  - Description: Flat price distribution with declining network volume.

---

## 7. Volume Weighted Average Price (VWAP)

### Description

VWAP represents the true intraday average price weighted by cumulative execution
volume.

### AI Input Schema

```json
{
  "close": 3125.0,
  "vwap": 3122.0
}
```

### Signal Threshold Matrix

- **BULLISH**
  - Rule: `close` is comfortably trading above the `vwap` value.
  - Description: Price is showing high value relative to the trading distribution.
- **BEARISH**
  - Rule: `close` is comfortably trading below the `vwap` value.
  - Description: Price is showing low value relative to the trading distribution.
- **SIDEWAYS**
  - Rule: Price is crossing back and forth across `vwap` with minor deviations.
  - Description: Market is trading strictly around value equilibrium.
