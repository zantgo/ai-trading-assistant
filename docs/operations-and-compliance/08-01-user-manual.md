# User Manual

**Version:** 5.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Status:** Approved
**Category:** Operations & Compliance

---

## 1. Audience & Prerequisites

This manual is for the operator of the Trading Platform — typically a quantitative trader or quant developer running the platform on a local workstation or a cloud VM. Readers are expected to be comfortable with the Rust toolchain (for the engine), Node.js or Bun (for the frontend), and basic Linux shell commands.

Hardware target: any 64-bit Linux/macOS machine capable of running a Rust binary and an Axum HTTP server. No GPU is required. Memory footprint at idle is ~150 MB; under live load it scales with the number of active Market Instances (4 timeframe pipelines each).

Software prerequisites:
- Rust toolchain (stable; `rustup` recommended)
- Node.js ≥ 18 or Bun ≥ 1.0
- SQLite (the engine creates `./telemetry.db` automatically)
- A POSIX shell for `./manage.sh` shortcuts

---

## 2. Installation & First Run

The project is a Cargo workspace with a Svelte 5 frontend in `ui/`. The **order of build matters**: the frontend must be built before the engine binary starts, because the engine serves `ui/dist/` as static assets.

```bash
# From workspace root:
cd ui
npm install          # or: bun install
npm run build        # or: bun run build
cd ../..
cargo run --bin execution-daemon -- --web
```

The engine reads `config.toml` from the workspace root on startup. **If `config.toml` is missing entirely**, the `manage.sh` first-run flow scaffolds a default from the documented schema before the binary launches (one-shot bootstrap — repeat invocations leave an existing `config.toml` untouched). **If `config.toml` exists but is malformed**, the binary panics with a descriptive error and the operator must fix the file before the next start — there is no automatic re-scaffolding on parse failure (a corrupt-but-existing file might contain deliberate operator edits, so re-scaffolding would risk silently overwriting them). Legacy `config.json` is still recognized by `load_config()` as a fallback for existing installations; new deploys should use `config.toml`.

---

## 3. Launch Modes

The project ships a convenience wrapper (`./manage.sh`):

| Command | Mode | Description |
|---------|------|-------------|
| `./manage.sh run` | Web (GUI) | Foreground with live logs; dashboard at `http://127.0.0.1:3000`. |
| `./manage.sh run-silent` | Web (GUI, background) | Daemonized; logs to `engine.log`. |
| `./manage.sh stop` | — | Stop the background engine instance. |
| `./manage.sh status` | — | Print process uptime. |
| `./manage.sh test` | All tests | Run TEST-CORE → TEST-ENGINE → TEST-UI sequentially (~18 s). |
| `./manage.sh test-core` | Pure math / indicators | Run only TEST-CORE (<3 s). |
| `./manage.sh test-engine` | DB, server, failover | Run only TEST-ENGINE (<5 s). |
| `./manage.sh test-ui` | Svelte 5 runes / components | Run only TEST-UI (<10 s). |

Headless cloud operation is supported by running the same binary without `--web` and applying a pre-validated `config.toml` (see [Global Architecture §4](../conceptual-foundations/01-02-global-architecture.md)).

---

## 4. Dashboard Walkthrough

The Svelte 5 dashboard is organized around three levels of navigation:

1. **Sidebar** — Engine selector (Home / Portfolio / Market / Trading / Analysis) + per-pair workspace list with live price, 24 h change, and pause/delete controls.
2. **Tab Header** — Contextual tabs per active engine: Workspace / Overview / Settings for the Market engine; for an open Market Instance the tabs are `Charts / Metrics / Alignment / Opportunities / Risks / Connection Quality / Analysis / Decision / Liquidity` (the **Liquidity** tab carries the Phase 4 LiquidityPanel — see [`07-04-ui-liquidity-panel-spec.md`](../ui-ux/07-04-ui-liquidity-panel-spec.md)). The **Connection Quality** tab is instance-scoped (see [`08-05-connection-quality.md §REST API`](../operations-and-compliance/08-05-connection-quality.md)).
3. **Main Viewport** — Renders the active tab. Each panel is a thin Svelte component with a companion CSS module per the project's CSS conventions.

For architectural details see [UI Overview](../ui-ux/07-01-ui-overview-spec.md) and [Dashboard Layout](../ui-ux/07-02-ui-dashboard-layout.md).

---

## 5. Configuring Engines & Timeframes

The single source of configuration truth is `config.toml` at the workspace root (legacy `config.json` is still recognized as a fallback by `load_config()` for existing installations). It controls:

- `candles.duration_seconds` — base (micro) timeframe
- `fast_timeframe`, `slow_timeframe`, `macro_timeframe` — additional timeframe tiers, each with `enabled` and `duration_seconds`
- `indicators.<name>.<param>` — per-indicator lookback, threshold, smoothing window, etc.
- `risk_per_trade_pct`, `leverage.cross_leverage`, `safety.*` — risk and safety gates
- `symbols` — list of `Exchange:Symbol` instruments to ingest
- `execution_policies` — user-defined rules (see [TAE Execution Policy](../engines/trade-automation-engine/03-03-04-tae-execution-policy-spec.md))

For the 4-tier timeframe model and UTC alignment rules see [Timeframe Model](../conceptual-foundations/01-04-timeframe-model.md).

The full configuration can be inspected via `GET /api/config` (returns the parsed `AppConfig`) and updated via `POST /api/config` (writes back to `config.toml` **explicitly**; the API is the only path that mutates `config.toml` on disk). Routine GUI runtime edits (e.g. changing a risk profile or paper balance) do **not** auto-overwrite `config.toml` — those edits are persisted to the `risk_profiles` and `paper_balances` DB tables per the precedence rules in [06-02-database-schema-spec.md §3.0](../../integration-and-api/06-02-database-schema-spec.md).

---

## 6. Running & Monitoring Trades

**Paper vs Live.** The default mode is paper trading — orders are routed to the internal matching engine described in [Paper Trading Spec](../engines/trade-automation-engine/03-03-05-tae-paper-trading-spec.md). **Live credentials must be entered into the encrypted `exchange_keys` SQLite table, not into `config.toml`.** `config.toml` holds no secret material. The encrypted-key management flow uses `POST /api/keys` (encrypt with `EXCHANGE_SECRET_KEY`) and the master key is loaded from the same-named environment variable at engine start. See [Database Schema §3.5](../integration-and-api/06-02-database-schema-spec.md) for the column schema and encryption contract.

**Reading the Decision Matrix.** The Decision Matrix is delivered per Market Instance on the WebSocket envelope (`/ws`) — there is no per-matrix REST endpoint. Open a Market Instance, switch to the "Decision" tab, and you will see `directional_guidance`, `market_stance`, `trade_readiness`, `confidence_assessment`, and the recommended `entry/exit/protection/target` strategies.

**Reading the Portfolio.** Active positions, margin usage, and the safety veto status are visible in the "Portfolio" sidebar entry. The PME's Ontological Priority Veto overrides the TAE's active stances when systemic thresholds are breached — see [PME Layer 4](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) for the trigger conditions.

**Trade journal.** All closed trades are written to `paper_trades` and `trade_telemetry_history`. Human annotations go into `trade_learning_journal`. Exposes via `GET /api/trade-journal/export/csv` and `/api/trade-journal/export/json`.

---

## 7. Telemetry & Logs

| Channel | Location | Contents |
|---------|----------|---------|
| Live log | `engine.log` | Engine stdout/stderr when running via `./manage.sh run-silent`. |
| Snapshot history | `./telemetry.db` (SQLite) | One row per completed candle; retention 7 days. |
| Equity history | `./telemetry.db` `portfolio_equity_history` | 60-s cadence snapshots; retention 30 days. |
| Trade archive | `./telemetry.db` `paper_trades`, `trade_telemetry_history` | Closed-trade ledger. |
| Connection quality | `./telemetry.db` `connection_quality_samples` | Rolling 1h / 6h / 24h windows; retention 7 days; served by `GET /api/connection-quality`. |
| Decision observability | `GET /api/system/observability` | Recent triggered policies and completed trades. |
| Engine heartbeat | `GET /api/system/status` | Connection state, latency_ms, active_pairs_count. |

---

## 8. Common Failure Modes

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Engine panics on startup with "config not found" | No `config.toml` (and no legacy `config.json` fallback) at workspace root | Run `./manage.sh` once; it scaffolds a default. Or copy `config.example.toml`. |
| WebSocket frames never arrive | `ui/dist/` is missing or empty | Rebuild frontend (`cd ui && npm run build`). |
| All values `null` in dashboard | Initial warm-up not finished | Wait `analysis_limit × duration_seconds` (default 500 × 60 s ≈ 8 h on micro); reduce `analysis_limit` for faster warm-up at the cost of less history. |
| `margin_usage_ratio > 95%` warning | Position size too large for current equity | Reduce `max_position_size_usd` in policy or close a position. |
| Indicator shows but `signals` array is empty | Indicators warmed up but no SignalKind conditions are firing yet | Verify thresholds in `config.toml` `[indicators.*]`; check the indicator rulebook via `GET /api/rules`. |
| Connectivity warning on a specific exchange | Adapter is in backoff after repeated disconnects | Check `/api/system/status`; permanent disable after 5 consecutive failures (supervisor must be restarted). |
| SQLite "database is locked" errors | Long-running query holding a write transaction | Reduce log retention or query frequency; the WAL mode is already enabled. |
| Veto stuck at `AVOID` for a symbol | PME safety trigger fired; threshold must clear | Inspect portfolio equity vs. peak; check `systemic_risk_score`; follow the [PME veto release procedure](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md#43-veto-release). |

---

## 9. Cross-References

- [Global Architecture](../conceptual-foundations/01-02-global-architecture.md) — Engine blueprint.
- [Pre-Trade Risk Controls](08-02-pre-trade-risk-controls.md) — Mandatory gates between policy trigger and order dispatch.
- [API Gateway Contract](../integration-and-api/06-01-api-gateway-contract.md) — REST + WebSocket surface.
- [Database Schema](../integration-and-api/06-02-database-schema-spec.md) — Persistent state.
- [AGENTS.md](../../AGENTS.md) — Build & test commands.
