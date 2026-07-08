# Bollinger Bands (20, 2.0)

## 1. Introduction — Trading Function
Bollinger Bands are volatility-based envelopes plotted at a standard deviation level above and below a simple moving average. They are the most widely used volatility overlay in technical analysis. Traders use them for:
- **Breakout detection:** price closing outside the bands signals a volatility expansion breakout.
- **Band-touch reversals:** in range markets, price touching a band edge suggests mean reversion.
- **Squeeze:** bandwidth at extreme lows signals an impending volatility expansion (the foundation of the TTM Squeeze indicator).
- **Trend riding:** price consistently walking the upper band signals a strong uptrend.

## 2. Mathematical Formula
```
Middle = SMA(close, 20)
Upper = Middle + 2.0 × σ(close, 20)
Lower = Middle - 2.0 × σ(close, 20)
%B = (Price - Lower) / (Upper - Lower)   // position within bands [0, 1]
```

## 3. Normalization
Bollinger was previously always 0.0 normalized (chart-only overlay). As of the deferred-indicator build-out, it now produces a real directional normalized value:
```
norm = clamp(((price - middle) / (upper - middle)))   // [-1, +1] scaled position
```
Stored in the `values` sub-map: `upper`, `middle`, `lower`. State labels determine signal firing.

## 4. Signals
| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Breakout | BOLLINGER_UPPER_BREAKOUT | Price ≥ upper band | Bullish |
| Breakout | BOLLINGER_LOWER_BREAKOUT | Price ≤ lower band | Bearish |
| BandTouch | BOLLINGER_UPPER_BAND_TOUCH | Price inside bands, %B > 0.90 (near upper edge). Structured push from engine. | Bearish |
| BandTouch | BOLLINGER_LOWER_BAND_TOUCH | Price inside bands, %B < 0.10 (near lower edge). Structured push from engine. | Bullish |

Both Breakout and BandTouch fire from distinct detection sources (structured engine push + label-based trigger) — duplicate badges from both paths are intentional.

## 5. Configuration
```toml
# Bollinger uses fixed 20-period, 2.0-std configuration
# (default is applied in the BollingerBands::new(20) constructor)
```
