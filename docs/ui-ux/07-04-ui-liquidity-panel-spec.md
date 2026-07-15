# LiquidityPanel UI Specification (Phase 4)

**Component:** `crates/frontend/src/components/LiquidityPanel.svelte`
**View key:** `liquidity` (in `CurrentView` enum)
**Mounted under:** workspace tab bar (instance view), alongside Alignment/Opportunities/Risks/Analysis/Decision
**Data sources:** `instance.microTerm.{liquidity, cluster, liquiditySignals}` on `TimeframeTelemetry`

## Purpose

The LiquidityPanel is the user-facing window into the Liquidity
Intelligence subsystem (Phases 0-3). It exposes three sub-views,
each answering a different question:

| Tab | Question it answers | Data source |
|---|---|---|
| **Flow** | What is the exchange telling us RIGHT NOW about forced closes? | `LiquidityFlow` (per-bar real event aggregate) |
| **Cluster** | Where do we BELIEVE the next cascade will come from? | `LiquidationClusterMatrix` (5-min refreshed estimator) |
| **Context** | What discrete signals are firing for this symbol? | `LiquiditySignal[]` (computed server-side per snapshot) |

## Visual design

The panel follows the platform's existing monochrome dark theme
(see `crates/frontend/src/components/styles/app-workspace.css`):

- Background: `rgba(255, 255, 255, 0.02-0.06)` (subtle elevation)
- Text: `#f5f5f7` primary, `rgba(255, 255, 255, 0.5-0.6)` secondary
- Bullish: `#26a69a` (teal)
- Bearish: `#ef5350` (red)
- Magnet: gradient from yellow to orange (intensity visualization)

No emojis. No colors that don't conform to the existing palette.

## Flow tab layout

```
┌─────────────────────────────────────────────────────────────┐
│ Flow | Cluster | Context                                     │
├─────────────────────────────────────────────────────────────┤
│ Real Liquidation Flow (per bar)                              │
│                                                              │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│ │ Long Liq │ │ Short Liq│ │ Net Flow│ │ Events   │           │
│ │  $50.0K  │ │  $10.0K  │ │  $40.0K  │ │    3     │           │
│ └──────────┘ └──────────┘ └──────────┘ └──────────┘           │
│                                                              │
│ CASCADE STATE                                                │
│ ┌──────────┐ ▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░  Intensity: 65/100      │
│ │ DETECTED │                                                │
│ └──────────┘                                                │
│                                                              │
│ LARGEST EVENT                                                │
│ Notional: $30.0K  Price: $49,500  Side: Long               │
└─────────────────────────────────────────────────────────────┘
```

The 4 stat cards update in real-time. The cascade badge color reflects
state: green for None, amber for Detected, red for Sustained, blue for
Exhausted.

## Cluster tab layout

```
┌─────────────────────────────────────────────────────────────┐
│ Flow | Cluster | Context                                     │
├─────────────────────────────────────────────────────────────┤
│ Estimated Liquidation Heatmap                                │
│                                                              │
│ Assumptions                                                  │
│ Source: FUNDING_ADAPTIVE  Buckets: 1, 3, 5, 10, 20, 50, 100│
│ Modulation: on  Confidence: 85%                            │
│                                                              │
│ CASCADE ASYMMETRY                                            │
│ Sign: -0.400  Direction: SHORT_SQUEEZE_RISK                │
│                                                              │
│ Short Clusters (above mid)                                   │
│ ┌─────────┐ ┌──────────────┐ ┌─────┐ ┌─────┐ ┌────────┐ ┌──┐│
│ │ $50,250 │ │ [$50,100-$50,500] │ │$1.5M│ │0.50%│ │ABOVE   │ │75││
│ └─────────┘ └──────────────┘ └─────┘ └─────┘ └────────┘ └──┘│
│ ... more rows ...                                            │
│                                                              │
│ Long Clusters (below mid)                                    │
│ (empty if no significant clusters)                           │
│                                                              │
│ OI Split                                                     │
│ Long: $30.0M  Short: $20.0M                                  │
└─────────────────────────────────────────────────────────────┘
```

Each cluster row is sortable. The magnet-strength bar at the right
is a thin colored bar whose width corresponds to the strength value.

## Context tab layout

```
┌─────────────────────────────────────────────────────────────┐
│ Flow | Cluster | Context                                     │
├─────────────────────────────────────────────────────────────┤
│ Active Liquidity Signals                                     │
│                                                              │
│ ┌─ BULLISH ──────────────────────────────────────────┐      │
│ │ CASCADE_SUSTAINED                                    │      │
│ │ 5 events in last 5 candles                          │      │
│ │ str 80  conf 90%                                    │      │
│ └─────────────────────────────────────────────────────┘      │
│                                                              │
│ ┌─ BEARISH ──────────────────────────────────────────┐      │
│ │ FUNDING_EXTREME                                      │      │
│ │ Funding rate 0.1200% (above extreme threshold)     │      │
│ │ str 95  conf 95%                                    │      │
│ └─────────────────────────────────────────────────────┘      │
│ ... more signals ...                                         │
└─────────────────────────────────────────────────────────────┘
```

Each signal is a card with a colored left border (green for bullish,
red for bearish, white for neutral) listing the kind, direction,
strength, confidence, and free-form evidence strings.

## Empty states

When no data is available yet:
- Flow: "Awaiting first completed bar with liquidation data…"
- Cluster: "Cluster matrix refreshes every 5 minutes. Awaiting first
  computation…"
- Context: "No active signals."

These placeholders are styled with `rgba(255, 255, 255, 0.4)` italic
text, consistent with the rest of the platform.

## Accessibility

- All interactive elements are real `<button>` elements (not styled
  divs) so screen readers handle them correctly.
- Color is never the sole carrier of meaning: cascade state, signal
  direction, and cluster type are all conveyed via text labels in
  addition to color.
- The magnet strength bar has a numeric value next to it.

## Performance

- All three views are pure-function renderings of immutable data.
  No expensive computations happen in the render path.
- The Svelte 5 `$derived` runes are used for all derived state; the
  component only re-renders when the underlying telemetry changes.
- The CSS module is statically generated; no runtime CSS injection.