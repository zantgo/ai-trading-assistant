# TAE Strategy Settings — Spec (v9)

**Version:** 8.0 (2026-08-20) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Locked for implementation
**Engine:** Trade Automation Engine (TAE)

The `tae` section of the strategy JSON is the execution policy: intake
quality, trade lifecycle, sizing, execution mechanics, exit/invalidation
management. **Disable-friendly rule:** `null` / `0` / empty = disabled =
today's behavior.

## Canonical `tae` section (`default`)

```json
"tae": {
  "intake": {
    "min_net_rr": 1.0,
    "min_score": null,
    "min_confidence": null,
    "max_setup_age_bars": null,
    "confirmation_bars": 0,
    "execution_veto": [],
    "direction_policy": "both",
    "trading_hours_utc": null,
    "volatility_gate": null,
    "funding_gate": null
  },
  "lifecycle": {
    "max_open_positions": 10,
    "max_per_setup_type": {},
    "max_per_direction": {},
    "pending_entry_expiry_bars": null,
    "reentry_cooldown_bars": 1,
    "daily": { "max_trades": null, "max_loss_pct": null }
  },
  "sizing": {
    "allocation_pct": 10.0,
    "per_setup_type_multipliers": {},
    "quality_curve": null,
    "after_loss_step_down": null,
    "max_position_size_pct_of_equity": null,
    "max_total_exposure_pct": null,
    "vol_scale": { "mode": "auto", "override": null }
  },
  "execution": {
    "entry_mode": "zone_midpoint",
    "spread_gate_bps": null,
    "slippage_bps": 5.0,
    "instant_fill_policy": "take_better"
  },
  "risk": {
    "invalidate_on": ["direction_flip"],
    "confidence_drop_pct": null,
    "breakeven_at_rr": null,
    "trailing": { "activate_at_rr": null, "atr_mult": null },
    "time_stop_bars": null,
    "signal_exit": "market"
  },
  "recovery": { "stale_state_window_secs": null }
}
```

## Instance controls (operational, not strategy JSON)

`POST /api/instances/:id/lifecycle { action }` — `start` (→ RUNNING),
`pause` (→ **instance PAUSED**: no new entries, pending orders cancelled, open positions
managed normally — close-only), `terminate` (→ STOPPED: cancel all orders,
force-close every open position at market, `exit_reason = "terminated"`).
UI buttons on the TAE dashboard header (paper + live; disabled in observe).

## Exit-reason vocabulary (extended)

`time_stop` · `breakeven` · `trailing_stop` · `expired` · `terminated` ·
`daily_budget` — added to the existing `tp` / `sl` / `invalidated_signal` /
`manual` / `stop_flatten` / `end_of_backtest` set.

## Guarantees

- Intake always evaluates all 4 TF snapshots (no TF-preference knob).
- Params-at-entry freeze: trailing/breakeven/time-stop settings are stamped
  at entry; recharge affects new setups only.
- Safety precedence: the PME soft gate and the L6/L7 strategy gates (risk
  ceiling, breadth floor) veto intake above any TAE setting.
- One position per instance, one side, one SL, one TP (scaled entries
  erased — v9 F-06).
