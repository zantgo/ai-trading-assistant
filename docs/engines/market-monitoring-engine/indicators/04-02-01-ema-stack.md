# 📈 Exponential Moving Averages (EMA 10, 50, 100, 200) Protocol

**Version:** 6.8 (2026-08-03) — see docs/CHANGELOG.md for the canonical version history.


## 1. Introduction
Exponential Moving Averages (EMAs) are trend-following, lagging indicators that apply more weight to the most recent price data. Unlike Simple Moving Averages (SMAs), EMAs react more quickly to price changes, making them useful for identifying immediate momentum shifts while smoothing out short-term market noise.

This strategy uses a structured, four-timeframe EMA stacking model to classify market structure, filter out low-probability entries, locate dynamic support and resistance, and trigger structural position invalidations.

---

## 2. Technical Structure and Calculations
By adjusting the lookback period, each of the four EMAs is assigned a specific structural role in the execution pipeline:

### 2.1 The Stacking Components
1.  **EMA 10 (Fast - Tactical Momentum):** Reacts rapidly to immediate price fluctuations. It serves as your short-term momentum tracker and tactical pullback reference.
2.  **EMA 50 (Medium - Swing Trend):** Establishes the primary trend direction over a medium-term horizon. It acts as the dynamic support/resistance baseline during active trends.
3.  **EMA 100 (Slow - Structural Support):** Represents the secondary structural boundary. It serves as a deeper value-area pullback zone.
4.  **EMA 200 (Long - Macro Trend Filter):** Represents the ultimate structural trend filter. The angle and location of the EMA 200 define the macro market regime.

### 2.2 Mathematical Smoothing Factor
Unlike SMAs, which weight all bars equally, the EMA calculation utilizes a multiplier ($\alpha$) based on the specified period ($P$):
$$\alpha = \frac{2}{P + 1}$$

The current EMA is then calculated by applying this multiplier to the current price ($Price_t$) and adding it to the weighted value of the previous period's EMA ($EMA_{t-1}$):
$$EMA_t = (Price_t \times \alpha) + (EMA_{t-1} \times (1 - \alpha))$$

---

## 3. Structural Regime Classification

Before evaluating any indicator crossover, the four EMAs must be analyzed to determine if the market structure supports a trending or ranging regime.

### 3.1 Full Trend Alignment (The Stack)
To enter trend-following positions, the four EMAs must be stacked in sequential order with positive or negative slopes:

*   **Bullish Stack (Long Only):**
    $$Price_t > EMA(10)_t > EMA(50)_t > EMA(100)_t > EMA(200)_t$$
    *   *Action:* Trend momentum is strong. Only long trades are permitted. Look to buy dynamic support pullbacks.
*   **Bearish Stack (Short Only):**
    $$Price_t < EMA(10)_t < EMA(50)_t < EMA(100)_t < EMA(200)_t$$
    *   *Action:* Downward momentum is strong. Only short trades are permitted. Look to short dynamic resistance rallies.

### 3.2 Tangled Regime (Consolidation)
*   **Condition:** The EMAs are crossing one another frequently, flattening out, and wrapping closely around the current close price.
*   **Action:** Price is range-bound. Trend-following signals are ignored. All trend-based entries are paused until a clean breakout occurs and EMAs begin to fan out.

---

## 4. Entry and Management Rules

### 4.1 The Macro Trend Filter (The 200 EMA Rule)
The EMA 200 is the ultimate trend filter across all execution timeframes:
*   **Long Prohibition:** If price is trading below the EMA 200 on your primary execution or macro chart, all Long entry signals are rejected.
*   **Short Prohibition:** If price is trading above the EMA 200 on your primary execution or macro chart, all Short entry signals are rejected.

### 4.2 Dynamic Support and Resistance Entries (Value Areas)
During a fully aligned trending stack, the area between the EMA 10 and EMA 50 represents the institutional "value area":
*   **Bullish Pullback Entry:** If a bullish stack is confirmed, wait for price to pull back into the zone between the EMA 10 and EMA 50. If price tests the EMA 50, wicks, and closes back above the EMA 10, enter a long position.
*   **Bearish Rally Entry:** If a bearish stack is confirmed, wait for price to rally into the zone between the EMA 10 and EMA 50. If price wicks off the EMA 50 and closes back below the EMA 10, enter a short position.

### 4.3 Structural Invalidation (The Decisive Close)
The EMAs provide early invalidation triggers before your hard stop-losses are hit:
*   **Long Invalidation:** If you are holding a long position and a candle closes decisively below the EMA 100 or EMA 200, the trend structure is compromised. Close the trade via a market order.
*   **Short Invalidation:** If you are holding a short position and a candle closes decisively above the EMA 100 or EMA 200, the trend structure is compromised. Close the trade via a market order.

## Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| StackChange | ESTABLISHED_BULLISH_STACK | Fast > Medium > Slow > Long, price > fast EMA | Bullish |
| StackChange | ESTABLISHED_BEARISH_STACK | Fast < Medium < Slow < Long, price < fast EMA | Bearish |
| StackChange | CONSOLIDATED_TANGLED_STACK | EMAs are interwoven, no clear ordering | Neutral |
| Crossover | EMA_PRICE_CROSS_FAST_BULLISH | Price crosses above the EMA 10 line — transition bar only | Bullish |
| Crossover | EMA_PRICE_CROSS_FAST_BEARISH | Price crosses below the EMA 10 line — transition bar only | Bearish |
| Crossover | EMA_PRICE_CROSS_MEDIUM_BULLISH | Price crosses above the EMA 50 line. Swing trend confirmation. | Bullish |
| Crossover | EMA_PRICE_CROSS_MEDIUM_BEARISH | Price crosses below the EMA 50 line. Swing trend breakdown. | Bearish |

The StackChange and Crossover signals are distinct. StackChange fires on regime-level ribbon alignment; Crossover fires on point-in-time price-vs-fast-EMA crossings.

## Normalization

The EMA Ribbon normalized score in [-1, 1] is binary/tertiary:
- **ESTABLISHED_BULLISH_STACK**: +1.0
- **DYNAMIC_BULLISH_RETEST**: +0.8 (bullish pullback to medium EMA)
- **CONSOLIDATED_TANGLED_STACK**: 0.0
- **DYNAMIC_BEARISH_RETEST**: -0.8 (bearish pullback to medium EMA)
- **ESTABLISHED_BEARISH_STACK**: -1.0

The `values` sub-map carries: `fast`, `medium`, `slow`, `long` (the 4 EMA values). Confidence = |normalized|.
