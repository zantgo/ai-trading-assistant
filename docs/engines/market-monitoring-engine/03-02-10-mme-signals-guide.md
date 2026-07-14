# MME Signals Guide — Readable Technical Signal Rulebook

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Purpose:** This is the human-readable rulebook for the platform's discrete technical signals. It defines the 12 canonical `SignalKind`s, their detection semantics, and their confirmation lifecycle. Each SignalKind has a detailed specification under [signals/](signals/).

---

## 1. What Is a Signal?

Per the [Ontology](../../conceptual-foundations/01-01-ontology.md) §3.9, a **Signal** is a discrete technical event detected from market telemetry, projected across the 10 Signal Evaluation Axes. Unlike an indicator (a continuous measurement), a signal is a *moment* — it fires, ages, and expires.

Each signal is an `IndicatorSignal`:

```rust
struct IndicatorSignal {
    kind: SignalKind,         // one of 12 (§2)
    direction: SignalDirection, // Bullish | Bearish | Neutral
    status: SignalStatus,     // Potential | Confirmed | Active
    label: String,            // specific event, e.g. BULLISH_DIVERGENCE
    strength: f64,
    age_bars: u32,            // freshness
}
```

---

## 2. The 12 SignalKinds

| SignalKind | Fires When | Spec |
|-----------|-----------|------|
| **Divergence** | Price and an oscillator disagree directionally. | [divergence.md](signals/05-02-01-divergence.md) |
| **Crossover** | Two series cross (line × signal, %K × %D, DI+ × DI−). | [crossover.md](signals/05-02-02-crossover.md) |
| **Threshold** | A value enters a named zone (RSI ≥ 70, CCI ≥ 100). | [threshold.md](signals/05-02-03-threshold.md) |
| **Breakout** | Price breaks a structural boundary (channel, level). | [breakout.md](signals/05-02-04-breakout.md) |
| **BandTouch** | Price contacts a channel/band edge. | [band-touch.md](signals/05-02-05-band-touch.md) |
| **ZeroLineCross** | An oscillator crosses its zero/mid line. | [zero-line-cross.md](signals/05-02-06-zero-line-cross.md) |
| **CompressionRelease** | A volatility squeeze fires. | [compression-release.md](signals/05-02-07-compression-release.md) |
| **LevelTest** | Price tests a horizontal level (S/R, fib, pivot, VWAP). | [level-test.md](signals/05-02-08-level-test.md) |
| **TrendFlip** | A directional regime reverses (Supertrend, PSAR, Aroon). | [trend-flip.md](signals/05-02-09-trend-flip.md) |
| **VolumeClimax** | Abnormal volume surge. | [volume-climax.md](signals/05-02-10-volume-climax.md) |
| **StackChange** | The EMA ribbon reorders. | [stack-change.md](signals/05-02-11-stack-change.md) |
| **PatternForming** | A chart or candlestick pattern is detected. | [pattern-forming.md](signals/05-02-12-pattern-forming.md) |

There are **100 signal-kind declarations** across 50 registry entries — i.e. most indicators declare several SignalKinds.

---

## 3. Signal Status Lifecycle

```
first detection ──► POTENTIAL ──(confirming condition)──► CONFIRMED ──(persists)──► ACTIVE
                        │
                        └──(invalidated)──► dropped
```

| Status | Usage |
|--------|-------|
| `Potential` | Geometry present but unconfirmed; secondary confluence only. |
| `Confirmed` | Confirming trigger fired (e.g. decisive candle close through a level); full weight. |
| `Active` | A confirmed state persisting over bars, tracked via `age_bars`. |

---

## 4. Freshness & Aging

The analyzer's stateful ager stamps `age_bars` on every signal (`0` = fresh this bar). Freshness feeds:

- The **Freshness axis** of the signal.
- The Opportunity Layer's freshness factor `F_fresh`.
- Decay of a signal's contribution to confluence as it ages.

---

## 5. Multi-Timeframe Agreement

A signal appearing on multiple timeframes is far stronger than one appearing on a single timeframe. The Alignment Layer's dimension 5 (Signal) counts cross-timeframe signal confluence, and the Signal's **Multi-Timeframe Agreement axis** reflects this.

---

## 6. Confirmation Patterns

Several SignalKinds require structural confirmation before upgrading `Potential → Confirmed`:

| SignalKind | Confirmation |
|-----------|--------------|
| Divergence | Candle close breaks nearest S/R by tolerance buffer. |
| Breakout | Close beyond the boundary (not just a wick). |
| TrendFlip | Confirmed on candle close, not intrabar. |
| PatternForming | Pattern completes its defining structure. |

---

## 7. Cross-References

- [Signal specifications](signals/) — One file per SignalKind.
- [Indicators Guide](03-02-09-mme-indicators-guide.md) — Indicator rulebook.
- [Metrics Matrix §4](../../matrices/02-07-metrics-matrix.md) — Signal serialization + axes.
- [Ontology — Signal & Evaluation Axis](../../conceptual-foundations/01-01-ontology.md)
