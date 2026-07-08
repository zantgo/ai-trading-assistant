# Choppiness Index (14)

## 1. Introduction — Trading Function
The Choppiness Index measures whether the market is trending or ranging by comparing the sum of true ranges over a period against the period's total price range. Values near 100 indicate a choppy, sideways, range-bound market; values near 0 indicate a strong directional trend. It is a **non-directional gate** — it never contributes a bullish or bearish score, but acts as a regime multiplier in the confluence engine: trending markets amplify directional conviction, choppy markets dampen it.

## 2. Mathematical Formula
```
TR_sum = Σ TrueRange(i), i ∈ [1, period]
HH = max(High, period)
LL = min(Low, period)
Choppiness = 100 × log10(TR_sum / (HH - LL)) / log10(period)
```

## 3. Normalization
Choppiness is a non-directional gate (`directional: false`). Its normalized value is always 0.0, but the state label classifies the current regime:
- `CHOP_CONSOLIDATION_RANGE` (≥ 61.8): range-bound/choppy — dampen confidence
- `CHOP_STRONG_TREND` (≤ 38.2): trending — amplify confidence
- `CHOP_TRANSITIONAL`: between extremes

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | CHOP_CONSOLIDATION_RANGE / CHOP_STRONG_TREND / CHOP_TRANSITIONAL | Any regime classification | Neutral |
| CompressionRelease | CHOP_CONSOLIDATION_RANGE | Coiled/range-bound energy precedes volatility expansion — distinct from Threshold signal kind | Neutral |

## 5. Configuration
```toml
[indicators]
chop_period = 14
```
