# Stochastic Oscillator (18, 5, 9)

## 1. Introduction — Trading Function

The Stochastic Oscillator compares a closing price to its price range over a given period. It oscillates between 0 and 100, with readings above 80 indicating overbought conditions and below 20 indicating oversold conditions. The %K line (fast) and %D line (slow/signal) produce crossover signals that are the indicator's primary trading trigger. Traders use it for:
- Overbought/oversold identification
- %K/%D crossover entries and exits
- Divergence detection against price

## 2. Mathematical Formula

```
%K = (Close - LowestLow(n)) / (HighestHigh(n) - LowestLow(n)) × 100
%D = SMA(%K, d_period)
Slow %K (s_line) = SMA(%K, s_period)
```

## 3. Normalization

The normalized score in [-1, 1] is a linear mapping from %K:

```
norm = (%K − 50) / 50   clamped to [-1, 1]
```

Labels include: `STOCH_OVERBOUGHT` (%K ≥ 80), `STOCH_OVERSOLD` (%K ≤ 20), `STOCH_BULLISH_BIAS`, `STOCH_BEARISH_BIAS`, `STOCH_NEUTRAL`. The `values` sub-map carries `k_line` and `d_line`.

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Crossover | STOCH_BULLISH/BEARISH_CROSSOVER | %K crosses %D. Structured push from engine using previous-bar %K/%D values (transition-only on the crossover bar). | Bullish / Bearish |
| Threshold | OVERBOUGHT_DISTRIBUTION | %K ≥ 80 | Bearish |
| Threshold | OVERSOLD_ACCUMULATION | %K ≤ 20 | Bullish |
| Threshold | STOCH_BULLISH_BIAS / BEARISH_BIAS | %K between 20-80 with directional momentum bias | Bullish / Bearish |
| Divergence | BULLISH/BEARISH_DIVERGENCE | Price-vs-stochastic divergence via SeriesDivergence (20-bar lookback). S/R gating upgrades Potential → Confirmed. | Bullish / Bearish |

## 5. Scoring & AI Context

`stochastic` is `directional: true`. Contributes to confluence scoring. A dedicated `stochastic_divergence` key provides independent divergence scoring.

## 6. Configuration

```toml
[indicators]
stoch_k_period = 18
stoch_d_period = 5
stoch_s_period = 9
```
