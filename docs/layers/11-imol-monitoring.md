# IMOL — Institutional Monitoring Layer

> **Layer 11 in the Institutional Trading Strategy Decision Pipeline.**
>
> **Purpose:** The IMOL is a dedicated trade-surveillance layer that monitors active
> positions after execution. It consolidates the fragmented monitoring capabilities
> (scale in/out, trailing stops, exit signals, safety state) into a unified
> dashboard. IMOL answers "how is the active trade performing?" while IRML answers
> "how much capital should be exposed?"

## 1. Purpose

After a trade enters the execution phase, the system needs continuous visibility
into position health. The IMOL provides:

- **Trade Management** — Active position state, slot-by-slot P&L, average entry pricing
- **Scaling In/Out** — Portion-based position building and reduction
- **Trailing Stop Status** — Break-even trail activation and current trail price
- **Partial Take Profits** — TP tier status across active slots
- **Exit Signals** — Opposite-signal confluence scores, decisive-close invalidation,
  CHoCH structural breakdown alerts
- **Safety State** — Consecutive losses, drawdown state, trade permission

IMOL is a **read-only surveillance layer** — it never executes trades. Execution
commands route through the existing paper trading API (`POST /api/paper/...`).
IMOL displays, alerts, and recommends; the user or automation acts.

## 2. Inputs

| Input | Source | Format |
|-------|--------|--------|
| Active position | `paper_trades` DB | `ActivePaperPosition` — direction, avg entry, size, allocated USD |
| Position slots | `position_slots` table | Per-slot entry price, size, allocated, realized PnL |
| Scale-in portions | `scale_in_portions` table | Portion entry prices and sizes |
| Take-profit orders | `open_orders` table | TP price levels per slot |
| Stop-loss orders | `open_orders` table | SL price levels per slot |
| Break-even trail | `paper_balances.break_even_trail_enabled` | Boolean + computed trail price |
| Exit signals | Live `MarketSnapshot` | Opposite-scores (registry-weighted), invalidation level, decisive-close status |
| Safety state | `SafetyManager` | Consecutive losses, caution thresholds, drawdown state |
| Current price | WebSocket snapshot | `mid_price` from latest completed candle |

## 3. Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Active trades summary | `ActiveTradesResponse` JSON | Frontend MonitoringPanel |
| Exit signal alerts | Scores ≥ threshold flagged with severity | Frontend MonitoringPanel (color-coded) |
| Position health | Unrealized PnL, ROI %, margin used | MonitoringPanel + StatePanel |
| Scale slot status | Per-slot entry/PnL/TP/SL | MonitoringPanel |

## 4. Sub-Components

### A. Position Dashboard

Unified view of all active trading slots:

- **Summary Row** — Direction badge (LONG/SHORT), average entry price, mark price,
  unrealized PnL with color coding (green/red), ROI %, margin used
- **Slot Detail Cards** — Per-slot: entry price, current PnL, SL price, TP tier status
  (untouched/partial fill/full fill), size allocated
- **Scale-in Timeline** — Chronological list of scale-in portions with entry price
  and percentage of total position

### B. Exit Signal Monitor

Continuous evaluation of position invalidation conditions:

- **Opposite-Signal Score** — Registry-weighted cumulative opposing-indicator score.
  Displayed as a progress bar: 0–100 scale with threshold marker at calibrated
  conviction bar (currently 60). Score ≥ threshold = exit warning.
- **Decisive Close Status** — 1-minute candle close vs final invalidation level.
  Flags: SAFE (price above/below level by >2%), WARNING (within 2%), VIOLATED.
- **CHoCH Structural Breakdown** — Change of Character detection with RVOL
  confirmation. Flags when structural trend shifts with institutional volume.
- **Composite Exit Severity** — None / Advisory / Warning / Critical based on
  the highest individual signal severity.

### C. Trailing Stop Status

Break-even trail state visualization:

- **Trail Enabled** — Boolean indicator
- **Trail Price** — Current trailing stop level (avg entry of remaining active
  slots after a TP fill)
- **Distance to Trail** — Price distance from mark to trail price as percentage
- **Next Trigger** — Price level at which the trail would advance next (if
  partial-TP-driven trail)

### D. Scale Controls

Quick-access position adjustment controls (delegating to paper trading API):

- **Scale In** — [+25%] [+50%] buttons for adding to position
- **Scale Out** — [−25%] [−50%] buttons for reducing position
- **Set TP** — Quick TP price input per slot
- **Set SL** — Quick SL price input per slot

### E. Safety State Display

Real-time safety metrics from the Safety Manager:

- Consecutive losses counter
- Caution / Suspend threshold bars
- Drawdown state (Normal / Recovery / Defensive / Critical / Shutdown)
- Trade permission badge

## 5. Integration

```
IEPL (Execution)
  └─► paper_trading (order placement & matching)
       └─► IMOL (monitoring)
            ├─► MonitoringPanel (frontend dashboard)
            └─► StatePanel (compact inline summary in Decision Flow)
```

IMOL reads live data from three sources:
1. **Paper trading DB** — Position, slots, orders, balance (via `GET /api/paper/status`)
2. **WebSocket snapshots** — Current price, indicator values for exit signal computation
3. **Safety manager** — Consecutive losses, drawdown (via `GET /api/paper/status`)

The dedicated endpoint `GET /api/monitor/active-trades?symbol=` provides a
consolidated response for the monitoring panel.

## 6. Frontend Rendering

Two display surfaces:

### MonitoringPanel (full panel)
- All sub-components (A–E) in a dashboard layout
- Position summary + exit signals side by side
- Slot details in expandable rows
- Scale controls as action buttons
- Auto-refreshes on a 5-second interval

### StatePanel (inline summary)
- Compact card in the MONITORING stage section showing:
  - Active? badge (YES/NO) with direction
  - Unrealized PnL with color
  - Opposite-signal exit score progress bar
  - "View Full Monitor" link → switches to MonitoringPanel

## 7. API Endpoint

### `GET /api/monitor/active-trades?symbol=`

Returns `ActiveTradesResponse`:

```json
{
  "symbol": "BTC",
  "has_active_position": true,
  "direction": "LONG",
  "average_entry_price": 87250.50,
  "total_size": 0.15,
  "unrealized_pnl": 340.25,
  "unrealized_roi_pct": 2.6,
  "margin_used": 1396.00,
  "account_value": 10436.00,
  "slots": [
    {
      "slot_id": 1,
      "direction": "LONG",
      "entry_price": 87000.00,
      "size": 0.10,
      "allocated_usd": 930.00,
      "unrealized_pnl": 250.30,
      "unrealized_pnl_pct": 2.87,
      "stop_loss_price": 86200.00,
      "take_profit_prices": [88500.00, 89500.00]
    }
  ],
  "break_even_trail": {
    "enabled": true,
    "trail_price": 87100.00
  },
  "exit_signals": {
    "opposite_score_long": 18,
    "opposite_score_short": 72,
    "opposite_exit_threshold": 60,
    "invalidation_level": 84500.00
  },
  "safety_state": {
    "consecutive_losses": 1,
    "caution_threshold": 3,
    "suspend_threshold": 7
  }
}
```

## 8. Expected Outcome

The IMOL completes the trade lifecycle in the frontend. The 5-stage pipeline
(Setup → Trigger → Confirmation → Execution → Monitoring) gives users full
visibility from market analysis through active trade surveillance. By separating
Risk Management into its own panel and adding a dedicated Monitoring stage, the
flow becomes:

1. **Pre-trade**: Setup, Trigger, Confirmation answer "should I trade?"
2. **Trade entry**: Execution answers "how do I enter?"
3. **Active trade**: Monitoring answers "how is it going?"
4. **Risk**: The standalone Risk Management panel answers "how should I size
   and protect?"

This mirrors professional trading desks where position monitoring is a distinct
function from risk management and trade generation.
