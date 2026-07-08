# On-Balance Volume (OBV) (Smoothed, 20)

## 1. Introduction — Trading Function

On-Balance Volume (OBV) is a cumulative volume indicator that adds volume on up-days and subtracts volume on down-days, producing a running total that reveals whether volume is flowing into (accumulation) or out of (distribution) an asset. The OBV trendline compared against a smoothed SMA creates a flip signal. Traders use it for:
- Confirming price trends (rising OBV with rising price = healthy uptrend)
- Spotting divergences (OBV making lower highs while price makes higher highs = bearish divergence)
- Detecting smart-money accumulation/distribution before price moves

## 2. Mathematical Formula

```
OBV[i] = OBV[i-1] + Volume    (if Close[i] > Close[i-1])
OBV[i] = OBV[i-1] - Volume    (if Close[i] < Close[i-1])
OBV[i] = OBV[i-1]             (if Close[i] == Close[i-1])
OBV_SMA = SMA(OBV, smoothing_period)
```

## 3. Normalization

The normalized score in [-1, 1] is computed from OBV's distance from its smoothed baseline:

```
diff = OBV - OBV_SMA
denom = max(|OBV|, |OBV_SMA|, 1)
norm = clamp(tanh((diff / denom) × 2.5))
```

Labels: `OBV_ACCUMULATION` (norm > 0.1), `OBV_DISTRIBUTION` (norm < -0.1), `OBV_NEUTRAL`.

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | OBV_ACCUMULATION | OBV in accumulation phase (norm > 0.1) | Bullish |
| Threshold | OBV_DISTRIBUTION | OBV in distribution phase (norm < -0.1) | Bearish |
| TrendFlip | OBV_TREND_FLIP_BULLISH/BEARISH | OBV crosses above/below its SMA (transition-only). Structured push from engine using prev-bar OBV/SMA comparison. | Bullish / Bearish |
| Divergence | BULLISH/BEARISH_DIVERGENCE | Price-vs-OBV divergence via SeriesDivergence. | Bullish / Bearish |

## 5. Scoring & AI Context

`obv` is `directional: true`. The OBV trendline flip captures smart-money flow shifts; the divergence signal detects exhaustion. Dedicated `obv_divergence` key.

## 6. Configuration

```toml
[indicators]
obv_smoothing = 20
```
