# Donchian Channels (20)

**Version:** 6.4 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.


## 1. Introduction — Trading Function

Donchian Channels display the highest high and lowest low over a rolling window, forming an upper and lower envelope around the midpoint. They are used for:

- **Breakout detection:** a close above the upper band signals a bullish breakout; below the lower band signals a bearish breakout.
- **Mean-reversion proximity:** when price approaches a band from inside without breaking out, a BandTouch signal fires (reversal expectation in range markets).
- **Trend confirmation:** price consistently near the upper band confirms an uptrend; near the lower band confirms a downtrend.

## 2. Mathematical Formula

```
Upper = max(high, period)
Lower = min(low, period)
Middle = (Upper + Lower) / 2
```

## 3. Normalization

The normalized score in [-1, 1] is computed from price position within the channel:

```
If price ≥ Upper Band:     norm = 1.0  (DONCHIAN_UPPER_BREAKOUT)
If price ≤ Lower Band:     norm = -1.0 (DONCHIAN_LOWER_BREAKOUT)
Else (inside channel):
  half = (Upper - Middle)
  n = (price - Middle) / half
  norm = clamp(n × 0.7)   // [-0.7, 0.7] range with gap at ±0.7
```

Labels: `DONCHIAN_UPPER_BREAKOUT`, `DONCHIAN_LOWER_BREAKOUT`, `DONCHIAN_UPPER_RANGE`, `DONCHIAN_LOWER_RANGE`. The `values` sub-map carries `upper`, `middle`, `lower`.

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Breakout | DONCHIAN_UPPER_BREAKOUT | Price ≥ upper band | Bullish |
| Breakout | DONCHIAN_LOWER_BREAKOUT | Price ≤ lower band | Bearish |
| BandTouch | DONCHIAN_UPPER_BAND_TOUCH | Price inside channel, position > 0.85 (near upper edge — mean-reversion proximity). Structured push from engine. | Bearish |
| BandTouch | DONCHIAN_LOWER_BAND_TOUCH | Price inside channel, position < 0.15 (near lower edge). Structured push from engine. | Bullish |
| LevelTest | DONCHIAN_MIDDLE_BAND_TEST | Price approaches the channel midpoint from either direction. Acts as dynamic equilibrium level — rejection signals continuation, crossing signals shift. | Direction depends on approach side |
| LevelTest | DONCHIAN_MIDDLE_BAND_SUPPORT | Price bounces off the middle band after a pullback from a breakout. Confirms breakout integrity. | Aligned with breakout direction |

## 5. Scoring

`donchian` is a `directional: true` indicator. Contributes to confluence scoring with both breakout strength and band-touch reversal signals.

## 6. Configuration

```json
{
  "indicators": {
    "donchian_period": 20
  }
}
```
