# ADX (Average Directional Index — 14)

## Indicator Foundations

The ADX is constructed from three underlying components, all derived from **price bars** (High, Low, Close):

### 1. True Range (TR)
```
TR_t = max( High_t - Low_t,  |High_t - Close_{t-1}|,  |Low_t - Close_{t-1}| )
```
Captures the full range of price movement including overnight/interval gaps.

### 2. Directional Movement (+DM and -DM)
```
+DM_t = High_t - High_{t-1}   (if positive and greater than -DM_t, else 0)
-DM_t = Low_{t-1} - Low_t     (if positive and greater than +DM_t, else 0)
```
Measures the upward and downward directional force separately.

### 3. Smoothing (Wilder's EMA, period = 14)
All three raw values (TR, +DM, -DM) are smoothed using Wilder's smoothing (an EMA variant with α = 1/period). The smoothed values produce:

```
+DI = 100 × smoothed_+DM / smoothed_TR
-DI = 100 × smoothed_-DM / smoothed_TR
DX  = 100 × |+DI - -DI| / (+DI + -DI)
ADX = Wilder's_EMA(DX, 14)
```

The ADX oscillates between **0 and 100**, purely measuring trend intensity regardless of direction.

---

## Trend Strength Regimes

Rather than viewing ADX as a binary on/off trigger, classify market conditions into four structural regimes:

| Regime | ADX Value | Interpretation | Strategy |
|--------|-----------|----------------|----------|
| **Congestion** | < 20 | No directional participation. Price is ranging or consolidating. | **Prohibit all trend-following entries.** Mean-reversion or scalping only. |
| **Emerging** | 20 – 25 | A breakout is attempting to form. Early momentum building. | Trades allowed at **reduced allocation** (1% rule). Validate with slope. |
| **Strong** | 25 – 40 | Fully supported institutional trend. High participation. | Maximum confidence. Execute at **full allocation** (up to 3%). |
| **Extreme / Exhaustion** | > 40 | Trend is overextended. Volatility reaching climax. | **Block new entries.** Monitor open trades for exhaustion exit. |

### Regime Adjustment Note
For very low-volatility assets (stablecoins, low-cap altcoins during consolidation), the thresholds may be reduced proportionally. For high-volatility assets (crypto majors during macro events), the exhaustion threshold may be raised to 50.

---

## Directional Crossover Identification

The direction of the trend is determined by the relationship between +DI and -DI:

### Bullish Trend Inception (Long Entry Signal)
- **Condition**: +DI crosses **above** -DI (the green line crosses above the red line).
- **Interpretation**: Upward directional force has overtaken downward force. Bullish bias established.

### Bearish Trend Inception (Short Entry Signal)
- **Condition**: -DI crosses **above** +DI (the red line crosses above the green line).
- **Interpretation**: Downward directional force has overtaken upward force. Bearish bias established.

**Critical**: DI crossovers are lagging signals. They must pass the strength and slope validation gates below to be actionable.

---

## The Trend Acceleration Slope Gate

A DI crossover alone is not sufficient. The ADX line must confirm that trend momentum is **actively accelerating**:

**Slope Formula** (3-bar lookback):
```
ADX Slope = ADX_t - ADX_{t-3}
```

Or for stricter validation:
```
ADX_t > ADX_{t-1} > ADX_{t-2}
```
Both conditions confirm that trend strength is genuinely building rather than oscillating.

**Gate Rules:**
- **Positive slope**: Trend momentum is accelerating → Validate the DI crossover entry.
- **Zero/flat slope**: Trend strength is static → Treat crossover with caution, reduce allocation.
- **Negative slope**: Trend strength is declining → **Reject the DI crossover**. The directional signal is a false breakout in fading momentum.

---

## The Volatility Exhaustion Exit ("Hard Hook")

When a trend becomes overextended, it can reverse violently. Waiting for an opposing DI crossover to exit can surrender a large portion of profits. The **Hard Hook** exit anticipates the reversal:

### Trigger Conditions (both must be met concurrently):

1. **Extreme Threshold Breached**: ADX line has risen above the exhaustion threshold (default: **40**).
2. **Slope Reversal**: The ADX line turns **downward** for 2 consecutive periods:
   ```
   ADX_{t} < ADX_{t-1}  AND  ADX_{t-1} < ADX_{t-2}
   ```

### Action
Close the position **immediately** via market order. This captures profits near the trend climax, well before an opposing DI crossover would print.

### Example
```
ADX:  38 → 42 → 44 → 43 → 39
                      ↑     ↑
                   crossed   slope confirmed negative
                   40 mark  for 2 bars → EXIT NOW
```

---

## Structural Invalidation

Regardless of ADX state, close the position immediately if:
- An opposing +DI/-DI crossover occurs (trend direction invalidated)
- Price breaks the nearest Support level (for a long) or Resistance level (for a short)

---

## Chart Annotation Reference

```
ADX Chart:
  50 ┤·········🔴 Extreme Threshold ·····
     │        ╱╲
  40 ┤·····🔴╱··╲······🟠··········
     │      ╱    ╲    ╱  ╲
  30 ┤····🟡······╲··╱····╲······
     │   ╱         ╲╱      ╲
  20 ┤·⚪ Trend ·················
     │ ╱                    ╲
  10 ┤⚪······················╲··

DI Lines:
  ── Green (+DI)  ── Red (-DI)
  Crossover up = Bullish inception
  Crossover down = Bearish inception

Colors:
  ⚪ Gray   = Congestion (ADX < 20)
  🟡 Gold   = Accelerating (slope positive, ADX > 20)
  🟠 Orange = Decelerating (slope negative, ADX > 20)
  🔴 Red    = Extreme exhaustion (ADX > 40)
```
