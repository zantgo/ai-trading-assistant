# Documentation Changelog

> **Purpose.** Single canonical home for version history, every deferred-work item, every audit-issue identifier, and every cross-version migration note. Per `docs/README.md` §Key Conventions, this is the only file in `docs/` that is allowed to carry `MAT-##`, `SIG-##`, `EXE-##`, `UI-##`, `DB-##`, `OPS-##`, `API-##`, `AUDIT-##`, and `Issue NN` references. All normatively cited by other documents.

---

## v4.0 (2026-07-16) — Corpus closure

### What changed

**Reconciliation against the source registry.** The entire corpus is reconciled against `crates/market-analyzer/src/indicators/registry.rs` (read-only verification at 2026-07-16). Outcomes:

1. The `VolatilityCycle` SignalKind rename (introduced in a v2.1 docs-only patch) never propagated to the registry. The registry still emits the variant `CompressionRelease`. v4.0 reverts the docs to **`CompressionRelease`** as the canonical name, with this entry recorded here as the only place the v2.1 rename is acknowledged.
2. Per-SignalKind counts in `01-01 §B.3` and `04-02-00 §Summary` were stale against the registry. v4.0 publishes the registry-verified counts (see **Per-SignalKind counts (registry-truth)** below).
3. The `×N` notation in per-indicator manifest rows (e.g. `PatternForming×2`) was undocumented, leading to misreading as declaration-count. v4.0 publishes a normative paragraph clarifying that **×N is internal event multiplicity**, not declaration count.

**Closure of v2.1 deferred-work items.** All audit-issue identifiers (MAT-, SIG-, EXE-, OPS-, UI-, DB-, API-, AUDIT-) are absorbed into the **Resolved Issues in v4.0** section below. They no longer appear in normative sections.

**Architecture corrections.**
- `cascade_risk` is the **8th** of the eight unipolar danger sub-dimensions in the Risk Matrix (not the 9th).
- Clock-monitor candle boundaries use exact UTC epoch multiples (`:MM:00.000`), never `:MM:59.999`.
- LiquidityPanel `cascade_asymmetry` sign convention: `> 0` ⇒ short squeeze risk, `< 0` ⇒ long squeeze risk.
- Risk Matrix `overall_risk.score` worked example recomputes to **28.3** (was 28.0).
- Decision Matrix `entry_danger.level` for `score = 20.0` is **`LOW`**, not `VERY_LOW` (half-open band boundary).
- Decision Matrix drops `DecisionGuard` confusion: Gates 1 and 7 are sequential, not duplicated. Gate 7 reads the stance set by Gate 1's PME upstream.

**API contract.**
- WebSocket `/ws` payload gets a normative reference to `02-07-metrics-matrix.md §2.1`. The `/* MarketSnapshot */` placeholder is removed.
- HTTP status & error envelope documented (`{ error: { code, message, details, request_id } }`).
- `/api/history?limit=` parameter documented.
- `/api/connection-quality` is instance-scoped (`instance_id`, `timeframe_secs`).
- Pre-dispatch approval resource added: `GET /api/pre-dispatch`, `POST /api/pre-dispatch/:id/approve`, `DELETE /api/pre-dispatch/:id`.
- `local_operator` identity model documented; carries through UI audit, DB, WebSocket control frames.

**Database schema.**
- New `risk_control_events` table for gate-rejection and override audit; `operator_id TEXT NOT NULL DEFAULT 'local_operator'`.
- `order_fills` table activated as live; PAE contract upgraded to "complete per-fill attribution".
- All `id` PKs use `INTEGER PRIMARY KEY AUTOINCREMENT` (canonical SQLite).
- Vocabularies canonicalized: `exit_reason` (5-value enum), order states (Execution Matrix lifecycle), `roi_pct` (deprecate `roi_percentage`).

**UI.**
- LiquidityPanel normative color/sign mapping documented.
- Dashboard's "19 indicator panes" corrected to "18 dedicated indicator panes + PriceChart overlay + shared generic panes".
- Decision Panel lists all canonical Decision Matrix fields including the four that were previously omitted.
- Connection Quality tab is instance-scoped.

### File count

`docs/` at v4.0: **130** markdown files (1 README + 128 numbered + this CHANGELOG). Net delta from v2.x: +1 file (this CHANGELOG).

### Per-SignalKind counts (registry-truth)

| # | SignalKind | Count | Notes |
|---|---|---|---|
| 1 | `Divergence` | **9** | 8 nested on parent (`supports_divergence: true`: `rsi`, `stochastic`, `chandemo`, `macd`, `obv`, `cmf`, `mfi`, `squeeze`) + 1 standalone (`oi_price_divergence`, own registry entry) |
| 2 | `Crossover` | **10** | |
| 3 | `Threshold` | **21** | |
| 4 | `Breakout` | **9** | |
| 5 | `BandTouch` | **4** | |
| 6 | `ZeroLineCross` | **13** | |
| 7 | `CompressionRelease` | **4** | Renamed from `VolatilityCycle` in docs v2.1; never propagated to registry; v4.0 docs revert to this canonical name |
| 8 | `LevelTest` | **14** | |
| 9 | `TrendFlip` | **10** | |
| 10 | `VolumeClimax` | **2** | |
| 11 | `StackChange` | **1** | |
| 12 | `PatternForming` | **3** | `patterns`, `candlestick`, `smc_liquidity` (each contributing one declaration) |
| | **TOTAL** | **100** | Sum-check: 9+10+21+9+4+13+4+14+10+2+1+3 = 100 |

**×N notation norm.** The `×N` suffix on per-indicator manifest rows (e.g. `PatternForming×2`) counts **internal event multiplicity within a single declaration**, not declaration count. For example, `patterns` has one `(patterns, PatternForming)` declaration but emits multiple PatternForming event subtypes — the `×2` reflects that internal multiplicity. **It does not** mean "2 declarations of `(patterns, PatternForming)`". The 100-declaration total is the sum of distinct `(indicator, SignalKind)` pairs across all 50 indicators.

---

## Resolved Issues in v4.0

Every audit issue from v2.x is closed below. New identifiers (`AUDIT-V4-NN`) are the canonical IDs going forward; legacy identifiers (`MAT-NN`, `SIG-NN`, etc.) are preserved for grep-back-compat but the normative reference is the new ID.

### MAT / SIG (SignalKind contract)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| `Issue 2.A` | `AUDIT-V4-001` | `state_confidence` rename (L3) | Stable in v2.1; doc-confirmed in v4.0 |
| `Issue 2.B` | `AUDIT-V4-002` | `opportunity_classification` removed from L6 (canonical is L4 `primary_opportunity`) | Confirmed in v4.0 |
| `Issue 2.C` | `AUDIT-V4-003` | L4 institutional redesign fields (`entry_zone`, `target_zone`, `invalidation_level`, `expected_rr_internal`, `time_horizon`) | Stable in v2.1; v4.0 confirms `invalidation_level` (not `invalid_level`) is the canonical name across L4 and Position Matrix |
| `Issue 2.D` | `AUDIT-V4-004` | `entry_danger` rename (L6, from `environment_favorability`) | Stable in v2.1; doc-confirmed |
| `Issue 2.E` | `AUDIT-V4-005` | `cascade_risk_index` placeholder in Overview Matrix | Stable (still placeholder, deferred to v4.x follow-up) |
| `SIG-02` | `AUDIT-V4-006` | Aroon `Crossover` → `TrendFlip` reclassification | Stable in v2.1 |
| `SIG-03` | `AUDIT-V4-007` | Supertrend `BandTouch` → `LevelTest` reclassification | Stable in v2.1 |
| `MAT-04` | `AUDIT-V4-008` | Risk Matrix banding (half-open intervals) | Confirmed in v4.0 §Decision Matrix §3.8 and Risk Matrix §2.3 cross-reference |
| `MAT-06` | `AUDIT-V4-009` | Opportunity Matrix `SCALP` cadence | Cell updated to "Every completed sub-minute candle" |
| `MAT-10` | `AUDIT-V4-010` | Decision Matrix `NO_RECOMMENDATION` reachability | Added explicit rule path for `NO_RECOMMENDATION` |
| `MAT-16` | `AUDIT-V4-011` | Opportunity Matrix setup-quality bands (half-open) | Stable in v2.1; v4.0 keeps |
| `MAT-17` | `AUDIT-V4-012` | Liquidity fields top-level (not nested in `indicators`) | Confirmed in v4.0 |
| `MAT-18` | `AUDIT-V4-013` | Liquidity-extension test count | 55 unit + 1 integration = 56 |
| `Issue 4.O` | `AUDIT-V4-014` | Veto-release endpoint `/api/instances/:id/safety/release-veto` | Confirmed in v4.0 |
| `Issue 4.N` | `AUDIT-V4-015` | Timeframe-weight divisor = slowest enabled tier | Confirmed in v4.0 §Timeframe Model §4 |
| `Issue 5.A` | `AUDIT-V4-016` | Config file is `config.json`, not `config.toml` | Confirmed in v4.0 |

### EXE (Execution / Order lifecycle)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| `EXE-08` | `AUDIT-V4-017` | Pre-dispatch state has no persistent table | Resolved by `risk_control_events` table (DB) and pre-dispatch approval resource (API) |
| — | `AUDIT-V4-018` | Order states vocab diverged between DB and Execution Matrix | Unified in v4.0 to Execution Matrix lifecycle vocabulary |
| — | `AUDIT-V4-019` | `exit_reason` DB vocabulary diverged from PAE | Unified to 5-value canonical enum (`STOP_LOSS`, `TAKE_PROFIT`, `SIGNAL_EXIT`, `MANUAL`, `VETO`) |
| — | `AUDIT-V4-020` | `order_fills` table deferred | Activated in v4.0; PAE upgraded to per-fill attribution |

### OPS (Operations & Compliance)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-021` | Clock-monitor `:14:59.999` candle-close convention | Corrected to `:15:00.000` |
| — | `AUDIT-V4-022` | Connection-Quality worked example arithmetic (67.5 → 65.5) | Corrected |
| — | `AUDIT-V4-023` | Backoff jitter post-cap (could exceed `max_backoff`) | Jitter applied before cap in v4.0 |
| — | `AUDIT-V4-024` | Linear extrapolation misnamed "LinearInterpolation" | Renamed to `LinearExtrapolation` |
| — | `AUDIT-V4-025` | NTP server order inconsistent between API and config | Standardized on `["pool.ntp.org", "time.aws.com"]` |
| — | `AUDIT-V4-026` | User manual mixed recovery paths for missing config | Unified canonical scaffold flow |
| — | `AUDIT-V4-027` | User manual tab list stale | Updated to include `Connection Quality` and `Liquidity` |
| — | `AUDIT-V4-028` | User manual instructed placing credentials in `config.json` | Replaced with reference to encrypted `exchange_keys` SQLite table |

### UI

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-029` | LiquidityPanel reversed `cascade_asymmetry` sign | Fixed; normative mapping block added |
| — | `AUDIT-V4-030` | LiquidityPanel data path `microTerm` | Replaced with `instance.timeframes.micro.*` |
| — | `AUDIT-V4-031` | LiquidityPanel used shortened signal names | Replaced with canonical `LIQUIDITY_*` prefixed names |
| — | `AUDIT-V4-032` | UI dashboard "19 indicator panes" | Corrected to "18 dedicated indicator panes + PriceChart overlay + shared generics" |
| — | `AUDIT-V4-033` | UI chart map: `volume_profile` / `oi_price_divergence` placement | Moved into PriceChart overlay bucket |
| — | `AUDIT-V4-034` | UI analysis panel "6 assessments" claim | Reconciled with 5 assessment fields + market-quality + market-phase |
| — | `AUDIT-V4-035` | UI overview panel missed `breadth_pct` numeric | Added `breadth_pct: f64 ∈ [-100, 100]` to Overview Matrix |
| — | `AUDIT-V4-036` | UI Decision panel omitted 4 canonical fields | Added `trade_readiness`, `entry_danger`, `expected_reward_risk_ratio`, `stop_loss_distance_pct` |
| — | `AUDIT-V4-037` | Connection Quality tab mounted per-instance but API unscoped | API now requires `instance_id` + `timeframe_secs` |
| — | `AUDIT-V4-038` | LiquidityPanel color semantics conflicting (state vs intensity) | Two-channel color mapping documented |
| — | `AUDIT-V4-039` | CSS Modules pattern underexplained | Normative block added in `07-01` |

### DB

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-040` | Header table inventory listed `individual_indicator_logs` (not in §3) | Removed; canonical telemetry is `market_snapshots.indicators_json` |
| — | `AUDIT-V4-041` | `id | SERIAL PK` PostgreSQL-style types against SQLite contract | Replaced with `INTEGER PRIMARY KEY AUTOINCREMENT` |
| — | `AUDIT-V4-042` | `market_snapshots` partial persistence of canonical MarketSnapshot | Documented as deliberate (not-persisted fields enumerated) |
| — | `AUDIT-V4-043` | `policy_id` labelled as FK without `execution_policies` table | Re-labelled as configuration string key |
| — | `AUDIT-V4-044` | `roi_percentage` legacy alias | Canonical `roi_pct`; `roi_percentage` deprecated at v5.0 |
| — | `AUDIT-V4-045` | `funding_rate_8h` cannot distinguish 0 from unset | Nullable column semantics (NULL = inherit; '0' = disable) |
| — | `AUDIT-V4-046` | Safety state persistence incomplete | Documented reconstruction rule from persisted metrics |

### API

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-047` | `/ws` payload `/* MarketSnapshot */` placeholder | Normative reference to `02-07-metrics-matrix.md §2.1` |
| — | `AUDIT-V4-048` | `liquidity_signals` empty-array omission policy ambiguous | Settled: always serialize `[]` when empty |
| — | `AUDIT-V4-049` | `/api/history` limit parameter undocumented | Documented (default 100, max 1000) |
| — | `AUDIT-V4-050` | Connection-quality API unscoped | Now requires `instance_id`, `timeframe_secs` |
| — | `AUDIT-V4-051` | Pre-dispatch approval flow has no API | New resource: `GET / POST / DELETE /api/pre-dispatch` |
| — | `AUDIT-V4-052` | HTTP status codes and error envelope undocumented | Documented JSON envelope with stable codes |
| — | `AUDIT-V4-053` | SPA fallback can mask `/api/*` typos | Scoped to non-`/api/*` paths |
| — | `AUDIT-V4-054` | Authentication = None conflicts with operator-ID requirement | `local_operator` identity model |
| — | `AUDIT-V4-055` | `roi_percentage` legacy field in journal API | Canonical `roi_pct` |

### Architecture narrative

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-056` | MME overview "seven sequential layers" — but L4 ∥ L5 parallel | Re-phrased: "L1–L3 sequential; L4 ∥ L5 parallel; L6–L7 sequential after convergence" |
| — | `AUDIT-V4-057` | MME L5 §1 input description omitted L1.5/L2.5 | Added explicit input list (Analysis Matrix + indicator map + LiquidityFlow + LiquidationClusterMatrix for `cascade_risk`) |
| — | `AUDIT-V4-058` | MME L6 confidence-attenuation formula LHS was `confidence` | Renamed to `confidence_assessment` |
| — | `AUDIT-V4-059` | `02-04-decision-matrix.md §6` cross-reference `§3.7 weights` invalid | Added `confluence_score` formula in `02-04 §2.3`; corrected §6 reference |
| — | `AUDIT-V4-060` | `02-00-matrix-field-ownership.md §2.6` missing `stop_loss_distance_pct` | Added row |
| — | `AUDIT-V4-061` | `02-00-matrix-field-ownership.md §1` ASCII diagram abbreviation `expected_rr_ratio` | Replaced with full name |

### Cascade / Liquidity

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-062` | `cascade_risk` called "9th dimension" in `03-02-06 §7` | Corrected to "8th sub-dimension" |
| — | `AUDIT-V4-063` | `03-02-11` cascade-invariant diagram omitted L4 edge | Updated to `L1.5 → L2.5 → {L4, L5} → L6` |
| — | `AUDIT-V4-064` | `03-02-11` test-coverage table arithmetic | Aligned with canonical 55 + 1 |
| — | `AUDIT-V4-065` | `cascade_asymmetry` threshold split (0.5 event vs 0.3 continuous) | Documented split |
| — | `AUDIT-V4-066` | `cascade_risk_index` placeholder shown in `01-01 §A.7` with non-canonical `trend` field | `trend` removed |

### Authoring hygiene

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-067` | Inline correction notes (`(MAT-XX — correction)` etc.) in normative sections | Stripped from normative text; preserved here |
| — | `AUDIT-V4-068` | Source-line citations (`crates/...rs::func`) in normative sections | Replaced with cross-doc references |
| — | `AUDIT-V4-069` | Subjective adjectives ("most defensible", "best estimate") in algorithmic specs | Replaced with measurable criteria |
| — | `AUDIT-V4-070` | Revision history tables in every doc | Consolidated into this CHANGELOG; tables deleted from individual docs |

### Counts / Renames (closed)

| Legacy ID | New ID | Description | Resolution |
|---|---|---|---|
| — | `AUDIT-V4-071` | Stale per-SignalKind counts (Threshold 26→21, Crossover 9→10, TrendFlip 8→10, ZeroLineCross 11→13) | Corrected |
| — | `AUDIT-V4-072` | `VolatilityCycle` rename to `CompressionRelease` (v4.0) | Reverted docs to registry-truth name |
| — | `AUDIT-V4-073` | `entry_danger.level = VERY_LOW` at score 20.0 violates banding | Corrected to `LOW` |
| — | `AUDIT-V4-074` | `01-01 §A.5` `overall_risk.score = 28.0` (canonical 28.3) | Corrected to 28.3; downstream formulas re-derived |

---

## Open Items (forwarded to future versions)

These are the items deferred from v4.0. They are tracked here only; downstream docs **must link here**, never restate status.

| ID | Item | Status | Target |
|---|---|---|---|
| `AUDIT-V4-005` | `cascade_risk_index` aggregation into `systemic_risk_score` | Open (placeholder field in canonical schema; aggregation formula deferred) | v4.1 |
| `AUDIT-V4-046` | `safety_state` deterministic reconstruction algorithm | Open (reconstruction rule documented but not yet unit-tested) | v4.1 |
| `AUDIT-V4-076` | `X-Operator-Id` optional header for caller-supplied operator identity | Open (single-user `local_operator` fixed identity in v4.0; caller-supplied identity in v5.0) | v5.0 |
| `AUDIT-V4-077` | Authentication beyond `local_operator` (multi-user / OAuth / mTLS) | Open | v5.0 |
| `AUDIT-V4-044` | `roi_percentage` legacy field removal | Deprecated in v4.0; remove entirely | v5.0 |
| `AUDIT-V4-078` | Per-WASM lightweight connection-quality scoring | Open | v4.1 |
| `AUDIT-V4-079` | PriceChart marker overlay for cluster positions (Phase 4 extension) | Deferred | v4.1 |
| `AUDIT-V4-080` | `liquidation_events` → PAE backtest ingestion | Deferred | v5.0 |

---

## Conventions enforced in v4.0

> Stated here once; referenced from individual docs so they don't drift.

1. **All enum values serialize as `SCREAMING_SNAKE_CASE`** (e.g. `STRONG_BULLISH`, `TRENDING_BULL`, `AVOID`, `CLOSE_ONLY`).
2. **File and directory names are lowercase-kebab-case**, prefixed `NN-MM[-KK]-…` per the section scheme in `docs/README.md`.
3. **Per-matrix field renames** are applied uniformly across every cite. The current canonical names:
   - L3 Analysis: `state_confidence`, `market_quality`, `market_regime`, `market_phase`, `bias`, `*_assessment` (×5)
   - L4 Opportunity: `forecast_confidence`, `primary_opportunity`, `opportunity_score`, `setup_quality`, `entry_zone`, `target_zone`, `invalidation_level`, `expected_rr_internal`, `time_horizon`
   - L5 Risk: 8 sub-dims (`market_risk`, `volatility_risk`, `execution_liquidity_risk`, `structure_risk`, `momentum_risk`, `signal_risk`, `execution_risk`, `cascade_risk`) + `overall_risk`
   - L6 Decision: `confidence_assessment`, `trade_readiness`, `entry_danger`, `expected_reward_risk_ratio`, `stop_loss_distance_pct`, `protection_strategy`, `target_strategy`, `directional_guidance`, `market_stance`, `strategy_environment`, `entry_guidance`, `exit_guidance`, `final_recommendation`
   - L7 Overview: `systemic_risk_score`, `market_breadth`, `breadth_pct`, `regime_distribution`, `opportunity_distribution`, `risk_distribution`, `cascade_risk_index`, `asset_ranking`, `market_synchronization`, `market_health`, `instance_count`, `active_symbols`
4. **Engine communication is unidirectional** — no backward dependencies. Information flows `Data Infrastructure → Market Monitoring → Trade Automation → Portfolio Management → Performance Analytics`.
5. **Every engine layer produces exactly one immutable matrix** as its output contract.
6. **Engine bifurcation** (MME L4 ∥ L5, converging at L6) is preserved everywhere it is referenced.
7. **Sizing formula** `S = (E × R) / (D_sl / 100)` with `E = available_margin` (Decimal from PME Capital Matrix), `R = risk_per_trade_pct / 100`, `D_sl = stop_loss_distance_pct` (raw percent float from Decision Matrix) — cast to Decimal at the type-boundary handoff (`03-03-03-tae-layer2-execution.md §2`).
8. **Two distinct drawdown metrics**: `max_daily_drawdown_pct` (5% early-warning) and `drawdown_limit_pct` (30% hard veto). See `03-04-05-pme-layer4-portfolio.md §3–§4`.
9. **Candle aggregation** uses exact UTC epoch-multiple boundaries: `interval_start = ⌊timestamp_ms / duration_ms⌋ × duration_ms`. Candles close at `interval_start + duration_ms`. The clock-monitor drift budget is `≤ 50µs` of UTC.
10. **Timeframe weighting**: `w_tf = clamp(duration_seconds / divisor, 0.2, 1.0)`, with `divisor = max(duration_seconds for tier in enabled_tiers)`.
11. **Systemic risk score**: `SystemicRisk = 0.6 × high_pct + 0.4 × sync_penalty`.
12. **Operator identity** is `local_operator` (fixed identity for single-user deployments); multi-user identity is on the v5.0 roadmap.
13. **All cross-doc audit-issue identifiers** (`MAT-##`, `SIG-##`, `EXE-##`, `OPS-##`, `UI-##`, `DB-##`, `API-##`, `AUDIT-##`, `Issue NN`) live **only** in this CHANGELOG. They are not in normative text.
