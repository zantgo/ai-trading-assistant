# Aroon (25)
**Group:** Regime

## 1. Introduction — Trading Function
The Aroon indicator measures the time elapsed since the highest high (Aroon Up) and lowest low (Aroon Down) within a lookback window. Values near 100 indicate a recent extreme; values near 0 indicate no recent extreme. The Aroon Oscillator (Up − Down) ranges from -100 to 100. Traders use Aroon to:
- Gauge trend strength (Up near 100, Down near 0 = strong uptrend)
- Detect crossovers (Up crosses Down = trend change signal)
- Identify consolidation (both Up and Down below 50)

## 2. Mathematical Formula
```
Aroon Up = ((period - bars_since_highest_high) / period) × 100
Aroon Down = ((period - bars_since_lowest_low) / period) × 100
Aroon Oscillator = Aroon Up - Aroon Down
```

## 3. Normalization
```
norm = clamp((Up - Down) / 100)   // linear mapping to [-1, 1]
```
Labels: `AROON_STRONG_UPTREND` (Up≥70, Down≤30), `AROON_STRONG_DOWNTREND` (Down≥70, Up≤30), `AROON_BULLISH_BIAS`, `AROON_BEARISH_BIAS`, `AROON_CONSOLIDATION`. The `values` sub-map carries `up` and `down`.

## 4. Signals
| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Crossover | AROON_BULLISH_CROSS / BEARISH_CROSS | Aroon Up crosses Aroon Down (transition-only via prev-bar Up/Down). Structured push from engine. | Bullish/Bearish |
| Threshold | AROON_STRONG_UPTREND | Up ≥ 70, Down ≤ 30 | Bullish |
| Threshold | AROON_STRONG_DOWNTREND | Down ≥ 70, Up ≤ 30 | Bearish |
| TrendFlip | AROON_BULLISH/BEARISH_TREND_FLIP | Up/Down leadership crosses (transition-only, distinct SignalKind from Crossover). Structured push from engine. | Bullish/Bearish |

## 5. Configuration
```toml
[indicators]
aroon_period = 25
```
