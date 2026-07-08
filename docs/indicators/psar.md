# Parabolic SAR (AF 0.02 → 0.20)

## 1. Introduction — Trading Function
Parabolic SAR (Stop and Reverse) is a trend-following trailing-stop overlay developed by J. Welles Wilder. The SAR dot sits below price in an uptrend and above price in a downtrend. When price crosses the SAR, the trend is considered reversed — the dot flips to the opposite side and the acceleration factor resets. It is used for:
- Trend identification (price above SAR = bullish, below = bearish)
- Dynamic trailing stop-loss (the SAR level itself)
- Entry/exit signals (SAR flip = TrendFlip; price crossing SAR = Crossover)
Parabolic SAR complements Supertrend, Ichimoku, and EMA overlays as the standard institutional trailing-stop system.

## 2. Mathematical Formula
```
Uptrend:   SAR(t+1) = SAR(t) + AF × (EP - SAR(t))
Downtrend: SAR(t+1) = SAR(t) + AF × (EP - SAR(t))

AF starts at 0.02, increases by 0.02 with each new extreme price, up to max 0.20.

EP = highest high (uptrend) or lowest low (downtrend) since the trend began.

Flip condition: SAR crosses price → trend reverses, SAR jumps to opposite EP, AF resets to 0.02.
```

## 3. Normalization
The normalized score in [-1, 1] follows the same pattern as Supertrend — conviction scales with distance from the SAR line:
```
dist = |price - sar| / |sar|
mag = 0.5 + 0.5 × tanh(dist × 15)
norm = direction × mag   [direction = +1 uptrend, -1 downtrend]
```
Labels: `PSAR_UPTREND`, `PSAR_DOWNTREND`. The `values` sub-map carries `sar` (SAR level) and `direction` (±1).

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| TrendFlip | PSAR_BULLISH_FLIP / BEARISH_FLIP | SAR direction changes (the dot flips sides). Structured push from engine when `flipped == true`. | Bullish / Bearish |
| Crossover | PSAR_PRICE_CROSS_BULLISH / BEARISH | Price crosses the SAR line. Transition-only via prev-bar price/SAR comparison. Distinct from TrendFlip which is the flip of the SAR itself. | Bullish / Bearish |

## 5. Scoring & AI Context
`psar` is `directional: true`. Contributes to confluence scoring. The AI treats PSAR as a trailing-stop reference and trend-confirmation tool alongside Supertrend.

## 6. Configuration
```toml
[indicators]
psar_af_step = 0.02
psar_af_max = 0.20
```
