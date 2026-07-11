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
| Threshold | CMF_STRONG_BUYING | CMF ≥ +0.20 — strong institutional buying pressure | Bullish |
| Threshold | CMF_BUYING_PRESSURE | CMF between +0.05 and +0.20 — moderate accumulation | Bullish |
| Threshold | CMF_SELLING_PRESSURE | CMF between −0.20 and −0.05 — moderate distribution | Bearish |
| Threshold | CMF_STRONG_SELLING | CMF ≤ −0.20 — strong institutional selling pressure | Bearish |
| ZeroLineCross | CMF zero cross | CMF crosses 0 (flows turn positive/negative). Transition-only via prev-bar CMF comparison in engine. | Bullish/Bearish |
| Divergence | BULLISH_DIVERGENCE | Price makes lower low, CMF makes higher low — hidden accumulation. Detected via 20-bar SeriesDivergence. Potential → Confirmed when nearest Support level broken with 0.2% tolerance. | Bullish |
| Divergence | BEARISH_DIVERGENCE | Price makes higher high, CMF makes lower high — distribution exhaustion. Potential → Confirmed when nearest Resistance level broken with 0.2% tolerance. | Bearish |

**Divergence Lifecycle:** Potential (oscillator disagrees with price, no structural break) → Confirmed (candle close breaks nearest S/R boundary by >0.2% tolerance). Dedicated `cmf_divergence` scoring key.

## 5. Configuration
```toml
[indicators]
cmf_period = 20
```
