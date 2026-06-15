# 📈 Bollinger Band Width Percentile (BBWP) Protocol

## 1. Introduction
Bollinger Band Width Percentile (BBWP) is a volatility-strength indicator that measures the relative width of Bollinger Bands compared to their historical baseline. While standard Bollinger Bands show real-time volatility envelopes, raw bandwidth values vary wildly across different price levels. BBWP solves this by normalizing bandwidth into a percentile rank ($0\%$ to $100\%$).

This strategy uses BBWP to identify the boundaries of market volatility cycles: predicting explosive trend breakouts during extreme compression phases and anticipating market flatlining during extreme expansion phases.

---

## 2. Technical Structure and Calculations
BBWP translates raw bandwidth into a historical percentile. This calculation is performed over two primary parameters: a smoothing period (typically 20 bars) and a historical lookback window (typically 252 bars, representing approximately one trading year).

### 2.1 The Mathematical Model

1.  **Calculate Raw Bollinger Band Width ($BBW_t$):** On every closed interval, determine the raw percentage spread of the bands relative to their simple moving average:
    $$BBW_t = \frac{\text{Upper Band}_t - \text{Lower Band}_t}{\text{Middle Band (20 SMA)}_t}$$

2.  **Establish the Historical Lookback Window:** Maintain a rolling array of the last 252 computed raw bandwidth values:
    $$\text{History}_t = [BBW_{t-251}, BBW_{t-250}, \dots, BWW_t]$$

3.  **Compute the Percentile Rank ($BBWP_t$):** Count how many historical bandwidth values in your lookback window are strictly narrower than the current raw bandwidth ($BBW_t$):
    $$N_{\text{below}} = \sum_{i=t-251}^{t-1} \begin{cases} 1 & \text{if } BBW_i < BBW_t \\ 0 & \text{otherwise} \end{cases}$$

    Divide this count by the total historical periods to establish your active percentile score:
    $$BBWP_t = \left( \frac{N_{\text{below}}}{251} \right) \times 100$$

---

## 3. Volatility Regimes & Execution Boundaries

By ranking volatility on a scale of $0\%$ to $100\%$, BBWP allows traders to identify the turning points of market volatility regimes.

[100% Volatility Climax] =========================================== (Exhaustion
Level: >90%) T R E N D F L A T L I N I N G Z O N E
------------------------------------------- (Normal Volatility Territory)

M I D - T E R M S W I N G Z O N E

------------------------------------------- (Normal Volatility Territory) E X P
L O S I V E C O I L I N G Z O N E ===========================================
(Compression Level: <10%) [0% Volatility Minimum]


### 3.1 Volatility Compression (Values < 10% — Stored Energy)
*   **Condition:** $BBWP_t < 10\%$. This indicates the bands are narrower than they have been $90\%$ of the time over the past 252 bars.
*   **Market Context:** The market is coiled in an extreme state of consolidation. Such tight compression is unsustainable and represents high stored energy.
*   **Strategic Application:** This is the pre-breakout zone. It aligns with the **Squeeze ON** phase. Traders do not place trades inside this zone; they monitor the charts closely for an imminent, highly explosive breakout.

### 3.2 Volatility Exhaustion (Values > 90% — Climax Phase)
*   **Condition:** $BBWP_t > 90\%$. This indicates the bands are wider than they have been $90\%$ of the time over the past 252 bars.
*   **Market Context:** The market has experienced extreme volatility expansion. The current trend is in a parabolic climax phase and is structurally overextended.
*   **Strategic Application:** This is the trend flatlining zone. Because momentum is reaching its physical limits, entering new trend-following positions is prohibited. For active positions, traders should tighten stop-losses or execute partial take-profits, as the market is highly likely to enter a mean-reverting or sideways consolidation phase.

---

## 4. Confluence Rules and Filtering

BBWP serves as a filter to validate or invalidate directional indicator signals.

*   **Squeeze Confirmation:** A **Squeeze OFF** (release) breakout signal from the Squeeze Momentum indicator is only valid if BBWP has recently dipped below the $10\%$ compression threshold. This ensures the squeeze has sufficient coiled energy to sustain a directional breakout.
*   **Wedge/Triangle Breakouts:** Chart pattern breakouts (such as triangles or wedges) are significantly more reliable when they occur while BBWP is below $15\%$, confirming that the pattern has consolidated volatility before breaking out.
