# CLI Launch Mode — Flow & Rationale

**Version:** 7.1 (2026-08-18) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Audience:** Operators using `execution-daemon --mode cli` for terminal-only monitoring.

---

## 1. Why a CLI mode?

The dashboard's Launch Setup wizard is the canonical setup path for GUI deployments. But one
operator workflow doesn't have a browser handy:

1. **Terminal-only / lightweight** — a fresh VPS, a tmux pane, or a container with no GUI.
   The operator runs `execution-daemon --mode cli`, answers a few prompts, and the instances
   just run in the terminal.
2. **Less overhead** — no Axum HTTP server, no WebSocket broadcast, no static asset serving.
   The same DIE/MME pipelines, the same SQLite telemetry DB, and the same L7 overview
   aggregation run — the CLI renders that overview as box-drawing tables instead of JSON.

The old `setup` subcommand and `--mode headless` were retired in v7.2: `--mode headless` now
maps to `--mode cli` with a deprecation notice.

---

## 2. Flow

The interactive launch mirrors the GUI Launch Setup wizard (observe-only for now):

```
1. Exchange             [hyperliquid | bitget]      ← pre-filled from --exchange / config.toml
2. Settlement currency  forced (HL=USDC, Bitget=USDT)
3. Instances            base symbol + 4 per-TF timeframe_secs, repeated until blank
                        ← pre-seeded from workspace.instances[] (keep with Enter)
4. Summary + confirm    → session init (observe) → instances just run
```

Every prompt is *non-blocking* — pressing Enter accepts the bracketed default. Timeframe
inputs are validated against `[10, 86400]` seconds.

### 2.1 What happens after confirm

1. The session defaults are pinned FIRST (`mode = "observe"` via `set_session_defaults`)
   and the session is initialised with the chosen exchange/currency — the same ordering as
   the web handler `POST /api/session/init`. No orders are ever dispatched.
2. The launch plan is written into the workspace config (per-instance TF durations, mode
   `observe`, operational mode `advisory`) so `registry::add_instance` resolves the exact
   pipeline durations.
3. Each instance is spawned through the registry with the boot retry policy (20 attempts ×
   30 s backoff). Instances persist into `config.toml` via the registry's normal
   `save_workspace` path — the next launch pre-fills from them.
4. The terminal monitor starts: clear-screen + redraw every `--interval` seconds.

The interactive flow mirrors the GUI wizard's Review step: the summary is printed, then
**"Start the monitor now? [Y/n]"** confirms before anything starts (declining exits without
touching `config.toml`).

### 2.2 Default timeframe ladder

The default per-instance durations derive from the **same ladder the registry falls back
to** (`WorkspaceConfig::tf_ladder_defaults`): micro 60 s, fast 180 s, slow/macro from the
workspace config (`slow_timeframe` / `macro_timeframe`, shipped defaults 300 / 900). The GUI
Launch Setup wizard reads the same values from `GET /api/config`, so every surface agrees on
the default pipeline durations.

### 2.3 Terminal monitor output

The renderer (`crates/execution-daemon/src/cli_renderer.rs`, plain ANSI, no TUI deps)
renders the **same server-computed payload** the GUI Market Overview panel consumes
(`OverviewMatrix` + the v7.2 panel fields — hero verdict, per-instance rows, signal-quality
and direction buckets, market-health bars — produced by the L7 aggregation task's
`build_overview_panel`):

- Header: market bias, health, synchronization, breadth, systemic risk, cascade risk
  (+confidence), TF agreement, alignment consensus + alignment distribution
- MARKET STATUS hero: TRADE / WAIT / STAND ASIDE with the actionable count and best setup
- Trade opportunities (best pair, direction, R:R, confidence, score)
- Signal quality + direction distribution buckets
- Market health sub-dimension bars (trend strength / liquidity / volatility / signal stability)
- Instances table: symbol, exchange, mode, price, status, micro slot
- Asset rankings: full 12-column table (symbol, price, bias, signal, direction, R:R, score,
  confidence, MTF score+label, risk, updated)
- Market summary: global summary sentence, active symbols, low coverage

The parity contract (13 checks, one producer / one payload / two renderers) is pinned in
[`01-10-cli-gui-parity.md`](01-10-cli-gui-parity.md) and enforced by `test-doc` (gate G18).

### 2.4 Saving snapshots

`--save` enables the existing snapshot-export task (JSON dumps per instance/tab to
`<output_path>`, default `./snapshots`) at boot, even when `[snapshot_export]` is disabled
in `config.toml`. Durable history without the GUI.

---

## 3. Flags

```bash
# Interactive terminal monitor (observe-only). Pre-filled from config.toml.
execution-daemon --mode cli

# Pre-fill exchange/currency, redraw every 10 s, enable snapshot JSON dumps.
execution-daemon --mode cli --exchange bitget --currency USDT --interval 10 --save

# Point at a non-default config.toml.
execution-daemon --mode cli --config /path/to/config.toml

# Scripted: existing config.toml instances are kept by pressing Enter through
# the prompts (exchange, base, 4 TF durations, next-base, then the Review
# confirm 'y').
printf 'hyperliquid\n\n\n\n\n\n\ny\n' | execution-daemon --mode cli

# Wrapper.
./manage.sh run-cli
```

---

## 4. Convergence with the GUI

The CLI and the GUI share the same engine boot path:

| Surface | Session init | Instance creation | L7 overview | Snapshot export | HTTP server |
|---|---|---|---|---|---|
| `--mode web` | Launch Setup wizard (`/api/session/init`) | `POST /api/instances` + `/config` | 5 s aggregation → dashboard JSON | task (config-driven) | Axum on :3000 |
| `--mode cli` | interactive plan → `set_session_defaults` | registry `add_instance` | 5 s aggregation → terminal renderer | task (`--save` enables) | none |

Both write the same `config.toml` shape via `save_workspace`, both persist to the same
`telemetry.db`, and both run the identical DIE/MME pipelines. The CLI is deliberately
lighter: no WS broadcast, no static assets, no API surface.

---

## 5. Why hand-rolled (vs `inquire` / `dialoguer`)?

The flow has ~6 prompts and no multi-select. The complexity premium of a new dependency
isn't justified; the prompts read from stdin and tolerate EOF gracefully (returning the
default), which keeps scripted use (`printf ... | execution-daemon --mode cli`) working.

---

## 6. Future work

- **Paper/live parity** — `--mode cli --trading paper|live` to mirror the wizard's
  Simulate/Execute paths (session defaults + `ExecutionMode` already support it; only the
  prompt surface is observe-only today).
- **Keyboard interaction** — pause/stop instances, toggle `--save` at runtime, jump
  between per-instance and overview frames.
- **Rich rendering** — migrate to `ratatui` if the monitor grows interactive frames.
