# Trigger Engine

> Reference spec for the trigger dispatch and automation event system. Code: `crates/engine/src/trigger_engine.rs`, `crates/engine/src/event_detector.rs`, `crates/engine/src/automation.rs`.

---

## Purpose

The trigger engine is the event dispatch layer between the analytical pipeline and the execution layer. It translates analytical events (indicator signals, regime changes, risk threshold breaches) into actionable trigger messages consumed by the paper trading engine and automation loop.

---

## Core Events

| Event Type | Source | Action |
|-----------|--------|--------|
| `SignalTriggered` | ITIL signal emission | Entry evaluation by IASL Trader Agent |
| `RegimeChanged` | IRCL regime transition | Recalculate ICSL weights, adjust IEPL stop multipliers |
| `SqueezeReleased` | Squeeze ON→OFF | Elevated entry priority in Expansion regime |
| `DivergenceConfirmed` | RSI/MACD/Oscillator divergence | Position invalidation or entry signal |
| `SRFlipOccurred` | S/R role reversal | Update ISML level hierarchy |
| `RiskThresholdBreached` | IRML category score > threshold | Exposure reduction or Emergency Stop |
| `DrawingEscalated` | Drawdown state machine transition | Trade Permission downgrade |
| `OppositeConfluenceExceeded` | ICSL opposite score > 60% | Full position close |
| `DecisiveCloseBeyondStop` | 1m close beyond invalidation | Full position close |

---

## Trigger Lifecycle

```
Analytical Event Detected
  → Trigger raised (with context: pair, timestamp, severity)
    → TriggerEngine evaluates: is this pair active? is gate open?
      → If gated: dispatch TriggerMessage to automation.rs
        → Automation: check IRML permission, regime gates
          → Execute paper trade (open/close/modify)
          → Log to trade_telemetry_history
```

---

## Integration

- **ITIL:** Signal events → trigger engine evaluates for action
- **IRML:** Risk threshold breaches → immediate exposure adjustment
- **IEPL:** Decisive close / opposite score → position invalidation
- **IPEL:** All triggers logged for post-trade audit
