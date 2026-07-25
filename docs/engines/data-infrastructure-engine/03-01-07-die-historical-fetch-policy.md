# DIE Historical Fetch Policy

**Version:** 6.5 (2026-07-24) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Specified — target of record (implementation status: README §Feature Status)
**Engine:** Data Infrastructure Engine (DIE)
**Owner:** network-adapters + portfolio-supervisor

---

## §1 Purpose

Replaces the current `collect_candles` ad-hoc bootstrap (which diverged per exchange — Hyperliquid omitting the `limit` parameter, Bitget hardcoding `limit=200` with no pagination, both coercing sub-minute intervals to `"1m"`) with a **single exchange-independent contract**. The trait plus its two adapter implementations let the `portfolio-supervisor` bootstrap path call `await policy.fetch(...)` and receive exactly `candle_buffer.size` candles whenever the exchange has enough history to give them, regardless of which exchange is wired up.

## §2 Frozen decisions (HFP-01 … HFP-10)

| ID | Decision |
|----|----------|
| **HFP-01** | New trait **`HistoricalFetchPolicy`** lives in `crates/network-adapters/src/adapters/historical_fetch.rs`. Its single method is `async fn fetch(&self, req: HistoricalFetchRequest) -> Result<Vec<NormalizedCandle>, HistoricalFetchError>`. |
| **HFP-02** | `HistoricalFetchRequest` carries: `exchange_symbol` (`"BTC"`, `"BTCUSDT"`, etc.), `timeframe_secs`, `target_count` (= `candle_buffer.size`), `end_ts` (epoch ms, default `now_ms`), `product_type` (`"USDT-FUTURES"` / `"USDC-FUTURES"` for Bitget; `None` for Hyperliquid). |
| **HFP-03** | **Sub-minute timeframes** (`timeframe_secs < 60`) **return `Ok(vec![])` immediately** without touching the network or the SQLite cache. The buffer will be filled by live trades. This is the per-TF behavior split from [08-08 §4](../../operations-and-compliance/08-08-candle-buffer-spec.md) (CB-05). |
| **HFP-04** | **≥ 1 minute timeframes** must paginate the REST endpoint until **either** `target_count` candles have been returned **or** the exchange reports it has no more history. Each adapter is responsible for its own cursor semantics (HFP-05, HFP-06). |
| **HFP-05** | **Hyperliquid** paginates with backward `startTime` cursors: request `[end_ts - N × duration_ms, end_ts]`; on response, if `response.len() == N` (or whatever implicit cap Hyperliquid enforces), issue another request with `end_ts = response[0].start_time_ms - 1`; stop when `result.len() ≥ target_count` or response is empty. Cap per request at 1000 candles; loop until exhausted or target reached. |
| **HFP-06** | **Bitget** paginates with forward `startTime` cursors: request `[start_ts, end_ts]` with `limit=200`; on response, if `response.len() == 200`, issue another request with `start_ts = response.last().start_time_ms + duration_ms`; stop when `result.len() ≥ target_count` or response has fewer than 200 rows. The hardcoded `limit=200` is per-page, not total. |
| **HFP-07** | Both adapters **filter out the currently open / incomplete candle** from the REST response. A `NormalizedCandle` whose `start_time_ms + duration_ms > now_ms` is dropped from the bootstrap result — it is by definition not yet closed and will be emitted by the live aggregator when it closes. |
| **HFP-08** | Each returned candle is tagged `reconstructed: Some(ReconstructionMethod::ExchangeHistorical)` so the UI can render a "synthesized" badge if needed (consistent with [08-04 §Serialization](../../operations-and-compliance/08-04-candle-reconstruction.md)). REST-sourced candles are technically live at fetch time, but the `ExchangeHistorical` tag distinguishes them from candles produced by the live aggregator in the same session. |
| **HFP-09** | REST responses are **always merged newest-first**, deduped by `start_time_ms`, capped at `target_count`. If the SQLite cache returns rows newer than the REST response's first row, those newer DB rows take precedence on overlap (DB wins for already-persisted data; this preserves the invariant that persisted rows are never overwritten by an out-of-order REST response). |
| **HFP-10** | REST pagination must complete within `fetch_timeout_ms` (default 30 000 ms total across all pages, configurable via `[candle_buffer] fetch_timeout_ms`). If the timeout is hit before `target_count` is reached, the adapter returns whatever it has; the bootstrap then accepts the partial result and continues — the pipeline enters `LOADING` and accumulates live candles to reach `size`. A warning is logged with the partial count. |

## §3 Trait definition

```rust
// crates/network-adapters/src/adapters/historical_fetch.rs

use async_trait::async_trait;
use crate::core_domain::NormalizedCandle;

#[derive(Clone, Debug)]
pub struct HistoricalFetchRequest {
    pub exchange_symbol: String,
    pub timeframe_secs: u64,
    pub target_count: usize,
    pub end_ts: u64,               // epoch ms; default = now_ms
    pub product_type: Option<String>, // Bitget only
}

#[derive(Debug, thiserror::Error)]
pub enum HistoricalFetchError {
    #[error("sub-minute timeframe {0}s bypasses historical fetch (HFP-03)")]
    SubMinuteBypassed(u64),
    #[error("HTTP {status} after {attempts} attempt(s): {body}")]
    Http { status: u16, attempts: u32, body: String },
    #[error("decode failure: {0}")]
    Decode(String),
    #[error("fetch timeout after {0}ms (HFP-10)")]
    Timeout(u64),
}

#[async_trait]
pub trait HistoricalFetchPolicy: Send + Sync {
    fn exchange(&self) -> Exchange;
    async fn fetch(
        &self,
        req: HistoricalFetchRequest,
    ) -> Result<Vec<NormalizedCandle>, HistoricalFetchError>;
}
```

The two implementations:

- `HyperliquidHistoricalFetch` — `crates/network-adapters/src/adapters/hyperliquid_historical_fetch.rs`
- `BitgetHistoricalFetch` — `crates/network-adapters/src/adapters/bitget_historical_fetch.rs`

Each is constructed with the adapter's REST config (base URL, retry budget, fetch timeout) and exposes the same trait surface.

## §4 Per-timeframe decision matrix

| `timeframe_secs` | REST touched? | DB touched? | Returned Vec length | Pipeline entry state |
|-----------------:|:-------------:|:-----------:|--------------------:|----------------------|
| `< 60`           | No (HFP-03)   | No (HFP-03) | `0`                 | `LOADING` with 0-candle buffer; fills from live trades |
| `≥ 60`           | Yes (HFP-04–HFP-06) | Yes (HFP-09) | exactly `target_count` if exchange has ≥ `target_count` history, else "all available" | `LIVE` immediately if merged Vec has `target_count`; `LOADING` otherwise |
| `≥ 60`, exchange has 0 history | Yes (returns 0) | Yes | `0` (DB may also be empty) | `LOADING` with 0-candle buffer; fills from live trades (rare: only on first-ever listing) |

## §5 Merge order (HFP-09)

```
db_rows   = newest-first query from market_snapshots
                  (symbol = ?, timeframe_secs = ?, timestamp <= end_ts)
                  LIMIT target_count;
rest_rows = await policy.fetch(req);

merged    = dedup_by_start_time_ms(rest_rows ++ db_rows)   // newer DB wins on overlap
          = sort_by_start_time_ms_desc(merged)
          = truncate_to(target_count)
```

The DB query uses `(symbol, timeframe_secs)` as the key today (existing schema in `06-02`); once the `timeframe_slot` column migration from [03-01-06 §5](03-01-06-die-candle-pipeline-states.md) ships, the query joins on `(symbol, timeframe_slot, timeframe_secs)` to disambiguate same-duration slots across instances.

## §6 Configuration schema

```toml
# config.toml — historical fetch behavior
[candle_buffer]
size = 500
fetch_timeout_ms = 30000                # HFP-10
sub_minute_skip_historical = true       # HFP-03
```

Per-adapter REST settings continue to live in `[adapters.<exchange>]` blocks (existing schema); this document does not redefine them.

## §7 Implementation work items

Tracked in `docs/CHANGELOG.md §Open Items` with `AUDIT-V7-NN` identifiers.

- `AUDIT-V7-320` — `network-adapters`: introduce `HistoricalFetchPolicy` trait + request/error types in `adapters/historical_fetch.rs`.
- `AUDIT-V7-321` — `network-adapters`: implement `HyperliquidHistoricalFetch` with backward `startTime` cursor pagination (HFP-05).
- `AUDIT-V7-322` — `network-adapters`: implement `BitgetHistoricalFetch` with forward `startTime` cursor pagination + `limit=200` per page (HFP-06).
- `AUDIT-V7-323` — `portfolio-supervisor`: replace `collect_candles` with `HistoricalFetchPolicy` caller; HFP-03 sub-minute short-circuit; HFP-09 merge; HFP-10 timeout handling.
- `AUDIT-V7-324` — `tests`: add 5 tests minimum — (a) sub-minute returns empty, (b) Hyperliquid paginates to `size`, (c) Bitget paginates `limit=200` to `size`, (d) DB-precedence on overlap, (e) timeout returns partial + warning.

## §8 Cross-References

- [08-08 Candle Buffer Spec](../../operations-and-compliance/08-08-candle-buffer-spec.md) — the master contract this doc implements (CB-05, CB-08, CB-10).
- [03-01-06 DIE Candle Pipeline States](03-01-06-die-candle-pipeline-states.md) — pipeline state transitions triggered by HFP return values.
- [08-04 Candle Reconstruction](../../operations-and-compliance/08-04-candle-reconstruction.md) — runtime gap-fill path (reconstructed candles are tagged identically to historical candles).
- [06-02 Database Schema](../../integration-and-api/06-02-database-schema-spec.md) — `market_snapshots` read path (HFP-09).