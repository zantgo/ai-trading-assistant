# 🔄 Divergence Detection Protocol

## 1. Introduction — Trading Function

Divergence occurs when price action and an oscillator move in opposite directions, signalling a potential trend exhaustion or reversal. The system detects both **regular divergence** (reversal signal) across 8 oscillator keys using two distinct detector types:

- **DivergenceDetector** (full): RSI + MACD — tracks peak/trough extremes over a 20-bar lookback, produces coordinates for chart rendering, and supports confirmation gating via S/R level breakouts.
- **SeriesDivergence** (lightweight): Stochastic, ChandeMO, MFI, CMF, OBV, Squeeze Momentum — compares current price-direction vs oscillator-direction over the same lookback, producing a direction signal without coordinate tracking.

Divergence signals follow a **Potential → Confirmed** status lifecycle. The confirmation is gated by the nearest support/resistance level: a Potential bullish divergence becomes Confirmed when a candle close decisively breaks **below** the nearest active Support level; a Potential bearish divergence becomes Confirmed when a candle close breaks **above** the nearest active Resistance level. The tolerance buffer is 0.2% of the level price.

## 2. Covered Indicators (8 divergence keys)

| Registry Key | Display Name | Detector | SignalKind | Parent Oscillator |
|-------------|-------------|---------|-----------|------------------|
| rsi_divergence | RSI Divergence | DivergenceDetector (full) | Divergence | RSI |
| macd_divergence | MACD Divergence | DivergenceDetector (full) | Divergence | MACD |
| stochastic_divergence | Stoch Divergence | SeriesDivergence | Divergence | Stochastic |
| chandemo_divergence | CMO Divergence | SeriesDivergence | Divergence | ChandeMO |
| mfi_divergence | MFI Divergence | SeriesDivergence | Divergence | MFI |
| cmf_divergence | CMF Divergence | SeriesDivergence | Divergence | CMF |
| obv_divergence | OBV Divergence | SeriesDivergence | Divergence | OBV |
| squeeze_divergence | Squeeze Divergence | SeriesDivergence | Divergence | TTM Squeeze |

Each divergence key emits a single `Divergence` signal on its parent oscillator and stores its own normalized score as a separate registry entry (contributing independently to the confluence score).

## 3. Detection Algorithm

### DivergenceDetector (RSI + MACD — Full)

Maintains rolling 20-bar histories of price, RSI, and MACD histogram. On each bar, scans for:
- **Bullish divergence**: Price makes a lower low vs the previous trough, but RSI/MACD histogram makes a higher low.
- **Bearish divergence**: Price makes a higher high vs the previous peak, but RSI/MACD histogram makes a lower high.

Produces `DivergenceCoords` (first_extreme, second_extreme) with price, indicator value, and index for chart rendering. Status starts as `Potential`; the `check_divergence_confirmation()` method upgrades to `Confirmed` when the nearest S/R level is decisively broken.

### SeriesDivergence (6 Generalized Keys — Lightweight)

Maintains rolling 20-bar histories of price and a single oscillator value. On each bar, compares the signs of recent price change vs oscillator change. A mismatch in direction signals `PotentialBullish` (price down, oscillator up) or `PotentialBearish` (price up, oscillator down). Confirmation uses the same S/R gate as the full detector via `series_divergence_confirmed()`.

## 4. Signal Status Lifecycle

| Status | Normalized | SignalStatus | Condition |
|--------|-----------|-------------|-----------|
| None | 0.0 | — | No divergence detected; indicator fill is INACTIVE |
| PotentialBullish | +0.5 | Potential | Divergence detected but S/R level not yet broken |
| ConfirmedBullish | +1.0 | Confirmed | Candle close decisively broke below nearest support level |
| PotentialBearish | −0.5 | Potential | Divergence detected but S/R level not yet broken |
| ConfirmedBearish | −1.0 | Confirmed | Candle close decisively broke above nearest resistance level |

## 5. Scoring & AI Context

Each divergence key is `directional: true` in the registry, contributing its `weight × normalized` value to the confluence engine. The parent oscillators (RSI, MACD, etc.) also receive the Divergence signal as a badge. The AI Structure Agent and Master Orchestrator receive the divergence state with explicit Potential/Confirmed classification and S/R proximity context.

## 6. Configuration

All divergence detectors use a fixed 20-bar lookback. The S/R confirmation tolerance is 0.2% of the level price. No separate config section — the divergence keys inherit from the parent oscillator's config parameters.
