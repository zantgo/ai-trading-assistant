# Money Flow Index (MFI) (14)

## 1. Introduction — Trading Function
The Money Flow Index is a volume-weighted RSI that oscillates between 0-100. Like RSI, it identifies overbought (>80) and oversold (<20) conditions, but incorporates volume to weigh the strength of price moves. It is often called the "volume-weighted RSI" and is used for divergence detection and overbought/oversold signals with institutional confirmation via volume weighting.

## 2. Mathematical Formula
```
TP = (High + Low + Close) / 3
MF = TP × Volume
Positive MF = sum(MF where TP[i] > TP[i-1])
Negative MF = sum(MF where TP[i] < TP[i-1])
MR = Positive MF / Negative MF
MFI = 100 - (100 / (1 + MR))
```

## 3. Normalization
Same piecewise sigmoid as RSI, with thresholds at 20/80 instead of 30/70:
```
mfi ≥ 80: norm = -0.7 - (mfi-80)/20 × 0.3   → [-1.0, -0.7]
mfi ≤ 20: norm = 0.7 + (20-mfi)/20 × 0.3     → [0.7, 1.0]
mfi ≤ 50: norm = (50-mfi)/30 × 0.7             → [0, 0.7]
mfi > 50: norm = -(mfi-50)/30 × 0.7            → [-0.7, 0)
```
Labels: `MFI_OVERBOUGHT_DISTRIBUTION`, `MFI_OVERSOLD_ACCUMULATION`, `MFI_BULLISH_FLOW`, `MFI_BEARISH_FLOW`, `MFI_NEUTRAL`.

## 4. Signals
| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | MFI_OVERBOUGHT / OVERSOLD | MFI ≥ 80 / ≤ 20 | Bearish/Bullish |
| Threshold | MFI_BULLISH_FLOW / BEARISH_FLOW | MFI between 20-80 with directional bias | Bullish/Bearish |
| ZeroLineCross | MFI midline 50 cross | MFI crosses 50 (bullish/bearish bias flip). Transition-only via prev-bar MFI. | Bullish/Bearish |
| Divergence | BULLISH/BEARISH_DIVERGENCE | Price-vs-MFI divergence via SeriesDivergence. | Bullish/Bearish |

## 5. Configuration
```toml
[indicators]
mfi_period = 14
```
