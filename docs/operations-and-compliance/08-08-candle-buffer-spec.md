# Candle Buffer Specification

**Version:** 6.10 (2026-08-15) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Specified — target of record (implementation status: README §Feature Status)
**Engine:** Data Infrastructure Engine (DIE)
**Owner:** network-adapters + portfolio-supervisor + market-analyzer

---

## §1 Purpose

This document is the **single source of truth** for the platform's candle buffer behavior across all exchanges and all four timeframes. Before this document existed the corpus had three competing numbers for the rolling buffer size (the config-model default `analysis_limit = 1000`, the in-memory cap `HIST_BUFFER_MAX = 1000`, the UI selector cap `500`) and per-exchange REST behavior diverged (Hyperliquid sent no `limit` parameter at all; Bitget hardcoded `limit=200` with no pagination; both exchanges silently coerced sub-minute durations to `"1m"` and warmed sub-minute pipelines with 60-second candles).

The platform now has one canonical behavior. Every exchange, every timeframe, every session runs the same algorithm against the same constants; the only inputs that vary are the configured `[candle_buffer] size` and the configured `timeframe_secs` of each pipeline.

## §2 Frozen decisions (CB-01 … CB-12)

| ID | Decision |
|----|----------|
| **CB-01** | The canonical candle-buffer size is configured by **`[candle_buffer] size`** in `config.toml`. The default value is **500**. The previous `analysis_limit` field is **removed**; the historical lookback depth equals the buffer size. |
| **CB-02** | Every per-timeframe in-memory buffer (`NormalizedCandle` history, `MarketSnapshot` history, indicator warm-up buffers) is rolled at exactly `candle_buffer.size` entries. Steady-state memory per pipeline is therefore `2 × size` rows of OHLCV + size snapshots. |
| **CB-03** | The rolling window is **FIFO oldest-evict**. On every completed candle the pipeline pushes the new candle at the back and pops the oldest from the front, keeping the deque at exactly `size`. There is no grow-then-trim mode — the deque never exceeds `size`. |
| **CB-04** | The **sub-minute / ≥ 1 minute** behavior split is binary on `timeframe_secs`: any duration strictly below 60 seconds follows CB-05–CB-07; any duration of 60 seconds or more follows CB-08–CB-10. There is no third branch. |
| **CB-05** | **Sub-minute timeframes (`timeframe_secs < 60`) do not fetch historical candles.** Neither the SQLite `market_snapshots` cache nor the exchange REST endpoint is consulted during bootstrap. The buffer starts at **zero entries** and grows one candle at a time as live trades close their buckets. |
| **CB-05a** | **(AUDIT-V8-003 idle-bucket heartbeat)** When a sub-minute pipeline has **no current candle** (the market is quiet after a close) and wall-clock has advanced past ≥ 1 bucket since the last completed bucket, the stale check synthesizes one doji per elapsed empty bucket at the last known close (`reconstructed: SYNTHETIC`, O=H=L=C=last close, zero volume), feeds it through every stateful indicator, broadcasts it, and pushes it into the in-memory `snapshot_history` (never the DB). The chart therefore shows one closed candle per wall-clock bucket even with zero trades — no gaps, no frontend flat-Doji bridges, no straight-line EMA segments. |
| **CB-06** | While a sub-minute pipeline is warming up, **every one of the 51 indicators is in `IndicatorLifecycleState::Loading`** until its individual `bars_required` is met. The pipeline transitions to `CandlePipelineState::Live` only when the buffer reaches `candle_buffer.size` (the rolling window is full). Reconstructed candles from `[08-04]` count toward `bars_seen` but are not enough on their own to promote `Loading → Live` — at least `bars_required` of **true live** candles must also be present. *(AUDIT-V8-001: `ema_stack` carries `bars_required = 1`; its ribbon lines are gated per-period inside `inject_ema_values` — see [04-02-01](../engines/market-monitoring-engine/indicators/04-02-01-ema-stack.md) §Sub-minute warm-up.)* |
| **CB-07** | Sub-minute pipelines therefore need `size × timeframe_secs` of wall-clock time to reach `Live` from a cold start (500 × 15 s = 125 minutes for the UI-default 15-second micro TF). This is **expected behavior** and is the user's locked decision; the UI surfaces the loading progress via `tf.pipeline_state = LOADING` and per-indicator badges. |
| **CB-08** | **≥ 1 minute timeframes (`timeframe_secs ≥ 60`) always start with exactly `candle_buffer.size` historical candles.** The platform paginates the exchange REST endpoint until either `size` candles are returned or the exchange's earliest available history is reached, then merges with the SQLite `market_snapshots` cache (newest DB takes precedence on overlap), then caps at `size`. |
| **CB-09** | A ≥ 1 minute cold start always completes with **all 51 indicators in `IndicatorLifecycleState::Live`** (every indicator's `bars_required ≤ size = 500`). The user sees a chart with full history on first paint; the only visible loading state is the brief exchange-REST round-trip. |
| **CB-10** | Per-exchange REST pagination is the responsibility of the `HistoricalFetchPolicy` trait ([03-01-07](../engines/data-infrastructure-engine/03-01-07-die-historical-fetch-policy.md)). Bitget must paginate against its 200-row limit until `size` rows are returned; Hyperliquid must paginate against its implicit return-size cap using backward `startTime` cursors. Both adapters **must converge to exactly `size`** rows from a cold start whenever the exchange has sufficient history. |
| **CB-11** | A **timeframe change** (the operator alters `micro.duration_seconds`, `fast.duration_seconds`, `slow.duration_seconds`, or `macro.duration_seconds`) triggers a **single-TF reload** of only the affected pipeline via the `reload_timeframe` API ([03-04 PMPE](../engines/portfolio-management-engine) / `portfolio-supervisor`). The other three pipelines continue uninterrupted. The reload tears down the affected `TimeframePipeline`, re-runs bootstrap against the new `timeframe_secs`, and re-emits `INITIALIZING → LOADING → LIVE` on the new pipeline. |
| **CB-12** | SQLite retains its existing **7-day** retention policy unchanged. The `market_snapshots` table is the long-term log; the in-memory `candle_buffer.size` rolling window is the only thing bounded by `size`. On eviction the candle leaves memory; the corresponding SQLite row remains queryable until the 7-day cleanup deletes it. |

## §3 Configuration schema

```toml
# config.toml — single source of truth for candle buffer behavior
[candle_buffer]
size = 500                                # canonical rolling window length (CB-01)
stale_threshold_secs = 300                # CB-06/§4 stale-loading escalation
sub_minute_skip_historical = true         # CB-05 (default true; set false only for legacy compatibility)
```

Per-instance `analysis_limit` blocks and the `analysis_limit` field on `TimeframeConfig` are **removed**. Any leftover value in `config.toml` after the migration is logged as a warning and ignored.

## §4 Behavior matrix

| `timeframe_secs` | Cold-start candles in buffer | State on cold start | Earliest `Live` | Historical source |
|-----------------:|-----------------------------:|---------------------|-----------------|-------------------|
| `< 60` (sub-min) | 0                            | `LOADING`           | After `size × timeframe_secs` of wall-clock | None (CB-05); quiet buckets filled by the idle-bucket heartbeat (CB-05a) |
| `≥ 60`           | exactly `size`               | `LIVE`              | Immediately after REST/DB merge | Exchange REST paginated to `size` + SQLite merge (CB-08) |
| `≥ 60` (already-warm DB) | ≤ `size`, oldest-first | `LIVE`              | Immediately | SQLite (newest), forward REST gap fill if any |

| Event | Effect |
|-------|--------|
| Live completed candle | Push back; if buffer length would exceed `size`, pop front (CB-03) |
| Reconstructed candle (gap fill) | Push back; pop front; increment indicator `bars_seen` but does **not** promote `Loading → Live` on its own (CB-06) |
| Operator TF change | Tear down + rebuild only that TF via `reload_timeframe` (CB-11) |
| Operator full restart | All 4 TFs rebuild from cold via CB-05/CB-08 rules |
| Connection loss + reconnect | Reconstruction path per `[08-04]`; pipeline remains in its current state |
| Indicator `bars_seen ≥ bars_required` and parent TF = `LIVE` | Indicator transitions `Loading → Live` ([03-02-15](../engines/market-monitoring-engine/03-02-15-mme-indicator-lifecycle-states.md)) |
| `(now - last_updated_at) > stale_threshold_secs` and `bars_seen < bars_required` | Indicator transitions `Loading → Failed`; TF follows (CB-06, see [03-01-06 §4](../engines/data-infrastructure-engine/03-01-06-die-candle-pipeline-states.md)) |

## §5 Reload triggers

The `reload_timeframe` API is the canonical entry point for all in-place changes that affect a single TF pipeline. Triggers:

| Trigger | Reload scope | Pipeline state after reload |
|---------|--------------|-----------------------------|
| Operator edits `micro.duration_seconds` (or fast/slow/macro) in the dashboard | micro only (or fast/slow/macro only) | `INITIALIZING → LOADING → LIVE` (CB-11) |
| Operator invokes `/api/instances/:id/reload?slot=micro` | micro only | same |
| Operator invokes `/api/instances/:id/reload?slot=all` | all four TFs | each follows the above |
| Boot-time instance spawn | all four TFs | `INITIALIZING → LOADING → LIVE` per CB-05/CB-08 |
| Recharge (existing API) | all four TFs | same |

Other reload triggers (clock-drift panic, exchange key rotation) continue to follow their existing per-trigger paths.

## §6 Interaction with existing systems

| System | Interaction |
|--------|-------------|
| `ConnectionStatus` (`03-01-01 §4.2`) | Independent. A `Failed` `ConnectionStatus` over the supervisor-level threshold escalates the parent `TimeframePipeline` to `Failed` via [03-01-06 §3](../engines/data-infrastructure-engine/03-01-06-die-candle-pipeline-states.md). |
| `ReconnectState` (`08-03`) | Independent. Reconnects do not reload the candle buffer; they trigger reconstruction (CB-12 + `[08-04]`). |
| `LifecycleState` (`03-03-06`) | Independent. A `STOPPED` instance halts the event loop but keeps the buffer readable; a `RUNNING → STOPPED` transition does not flush history. |
| SQLite `market_snapshots` (`06-02`) | The 7-day retention policy is unchanged. Bootstrap reads SQLite newest-first; live candles append; cleanup deletes rows older than 7 days. The in-memory `size` cap is unrelated. |
| `MarketContext` ([03-02-02 §6](../engines/market-monitoring-engine/03-02-02-mme-layer1-metrics.md)) | Continues to be computed once the pipeline is `Live`; while `Loading` it emits neutral values explicitly tagged `INSUFFICIENT_DATA` per the indicator-lifecycle spec. |
| UI `analysisLimit` selector | Removed. The UI selector now exposes only `[candle_buffer] size` from the config API. |

## §7 Implementation work items

Tracked in `docs/CHANGELOG.md §Open Items` with `AUDIT-V7-NN` identifiers.

- `AUDIT-V7-300` — `config-models`: introduce `CandleBufferConfig` struct + `[candle_buffer]` block; remove `analysis_limit` from `TimeframeConfig`; add migration log line for legacy `analysis_limit` keys.
- `AUDIT-V7-301` — `core-domain`: introduce `CandlePipelineState`, `IndicatorLifecycleState`, `IndicatorLifecycleStatus` (see [03-01-06](../engines/data-infrastructure-engine/03-01-06-die-candle-pipeline-states.md) §2 and [03-02-15](../engines/market-monitoring-engine/03-02-15-mme-indicator-lifecycle-states.md) §2).
- `AUDIT-V7-302` — `network-adapters`: introduce `HistoricalFetchPolicy` trait; implement `HyperliquidHistoricalFetch` (paginated backward cursor); implement `BitgetHistoricalFetch` (paginated forward cursor with `limit=200` per page).
- `AUDIT-V7-303` — `market-analyzer`: replace `HIST_BUFFER_MAX = 1000` with `candle_buffer.size`; ensure deque never exceeds `size`; populate `IndicatorLifecycleStatus` for all 50 registry entries; publish `tf.pipeline_state`.
- `AUDIT-V7-304` — `portfolio-supervisor`: rewrite `collect_candles` to use `HistoricalFetchPolicy`; sub-minute returns empty Vec; ≥ 1 minute paginates until `size` then merges DB; expose `reload_timeframe(instance_id, slot, new_config)` API.
- `AUDIT-V7-305` — `api-gateway`: add `POST /api/instances/:instance_id/reload?slot=`; extend `/api/history` clamp to `candle_buffer.size`; add `pipeline_state` + `indicator_lifecycle` to the `/api/history` response.
- `AUDIT-V7-306` — `execution-daemon`: fix `--web` boot so `init_session` does not deactivate before auto-spawning configured instances (currently `main.rs:261` sets `active = false` immediately).
- `AUDIT-V7-307` — `ui`: introduce `IndicatorStatusBadge.svelte`; honor `tf.pipeline_state` in chart headers; stop merging old values when a snapshot arrives with `pipeline_state = LOADING`; remove the `analysisLimit` selector (replace with read-only display of `candle_buffer.size`).

## §8 Cross-References

- [01-04 Timeframe Model](../conceptual-foundations/01-04-timeframe-model.md) — sub-minute support, pipeline architecture.
- [01-08 Candle Buffer & Indicator Lifecycle](../conceptual-foundations/01-08-candle-buffer-and-indicator-lifecycle.md) — conceptual overview tying this doc to its companions.
- [03-01-06 DIE Candle Pipeline States](../engines/data-infrastructure-engine/03-01-06-die-candle-pipeline-states.md) — per-TF pipeline state machine.
- [03-01-07 DIE Historical Fetch Policy](../engines/data-infrastructure-engine/03-01-07-die-historical-fetch-policy.md) — exchange-independent fetch contract.
- [03-02-15 MME Indicator Lifecycle States](../engines/market-monitoring-engine/03-02-15-mme-indicator-lifecycle-states.md) — per-indicator operational lifecycle.
- [08-04 Candle Reconstruction](08-04-candle-reconstruction.md) — gap-fill source (CB-12).
- [06-02 Database Schema](../integration-and-api/06-02-database-schema-spec.md) — `market_snapshots` retention policy.
- [08-05 Connection Quality](08-05-connection-quality.md) — `reconstructed_candles` counter.