# Relative Volume (RVOL)

## 1. Introduction — Trading Function

Relative Volume (RVOL) measures current volume against its rolling average to classify participation intensity. It is a **non-directional gate** — it never contributes a bullish or bearish directional score, but acts as a multiplier/filter for other indicators' signals. Traders use it to:
- Validate breakouts (institutional-level volume confirms the move)
- Identify exhaustion (climax volume at trend extremes)
- Gauge market participation (normal vs elevated vs depressed)

## 2. Mathematical Formula

```
RVOL = Volume / AverageVolume(20)
```

## 3. Normalization

RVOL maps to 4 discrete bands (non-symmetric, non-spanning):

- RVOL < 1.0: norm = −0.5 (LOW_PARTICIPATION_VOLUME)
- RVOL 1.0–1.5: norm = 0.2 (NORMAL_PARTICIPATION_VOLUME)
- RVOL 1.5–3.0: norm = 0.8 (INSTITUTIONAL_BREAKOUT_VOLUME)
- RVOL ≥ 3.0: norm = −1.0 (EXHAUSTION_CLIMAX_VOLUME)

RVOL is a non-directional gate (`directional: false`). The normalized value is used by the confluence engine as a gate multiplier, not a directional contributor.

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| VolumeClimax | EXHAUSTION_CLIMAX_VOLUME | RVOL ≥ 3.0 (label contains "CLIMAX" and key=="rvol") | Neutral |

## 5. Scoring & AI Context

`rvol` is `directional: false`. Acts as a breakout-validation gate for S/R role-reversals, Fibonacci breakouts, and pattern confirmations. The AI treats RVOL as an institutional-participation gauge.

## 6. Configuration

```toml
[indicators]
rvol_threshold_institutional = 1.5
rvol_threshold_climax = 3.0
```
