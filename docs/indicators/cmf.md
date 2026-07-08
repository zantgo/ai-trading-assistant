# Chaikin Money Flow (CMF) (20)

## 1. Introduction — Trading Function
Chaikin Money Flow measures buying vs selling pressure by combining price position within the daily range with volume. Positive CMF indicates accumulation (closing near the high on strong volume); negative CMF indicates distribution (closing near the low on strong volume). Traders use it for confirming trend strength and spotting divergences between money flow and price.

## 2. Mathematical Formula
```
MFM = ((Close - Low) - (High - Close)) / (High - Low)
MFV = MFM × Volume
CMF = Sum(MFV, period) / Sum(Volume, period)
```
CMF is already expressed in [-1, 1].

## 3. Normalization
```
norm = clamp(cmf × 3.0)   // amplify then clamp
```
The amplifier (×3.0) means CMF saturates at ±1.0 for CMF values above 0.33 or below -0.33. Labels: `CMF_STRONG_BUYING` (≥0.2), `CMF_BUYING_PRESSURE` (0.05–0.2), `CMF_NEUTRAL_FLOW`, `CMF_SELLING_PRESSURE` (-0.2 to -0.05), `CMF_STRONG_SELLING` (≤-0.2).

## 4. Signals
| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | CMF_STRONG_BUYING / BUYING_PRESSURE | CMF indicates net buying pressure | Bullish |
| Threshold | CMF_STRONG_SELLING / SELLING_PRESSURE | CMF indicates net selling pressure | Bearish |
| ZeroLineCross | CMF zero cross | CMF crosses 0 (flows turn positive/negative). Transition-only via prev-bar CMF comparison in engine. | Bullish/Bearish |
| Divergence | BULLISH/BEARISH_DIVERGENCE | Price-vs-CMF divergence via SeriesDivergence (20-bar). | Bullish/Bearish |

## 5. Configuration
```toml
[indicators]
cmf_period = 20
```
