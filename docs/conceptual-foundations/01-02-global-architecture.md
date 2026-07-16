# Trading Platform Architecture Specification

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.
**Purpose:** This document defines the high-level, two-dimensional architecture of the complete Trading Platform. It outlines the boundaries, operational responsibilities, layer structures, and interface matrices for the five core engines of the system, providing a structural blueprint for developers, system engineers, and frontend designers.

---

## 1. Architectural Blueprint: The Two-Dimensional Framework

The Trading Platform is designed around a **Two-Dimensional Architectural Framework** that isolates business domains horizontally and analytical transformations vertically. This design guarantees modularity, testability, and deterministic data flow.

|             | Data Infra. (DIE) | Market Monitor (MME) | Trade Auto. (TAE) | Portfolio Mgmt. (PME) | Perf. Analytics (PAE) |
| ----------- | ----------------- | -------------------- | ----------------- | --------------------- | --------------------- |
| **LAYER 1** | Raw Data L.       | Metrics L.           | Policy L.         | Position L.           | Trade Anal.           |
| **LAYER 2** | Market D. L.      | Alignment L.         | Execution L.      | Exposure L.           | Strat. Anal.          |
| **LAYER 3** | Data Qual.L.      | Analysis L.          |                   | Capital L.            | Risk Anal.            |
| **LAYER 4** | Dist. Layer       | Opp. Layer           |                   | Portfolio L.          | Perf. Layer           |
| **LAYER 5** |                   | Risk Layer           |                   |                       |                       |
| **LAYER 6** |                   | Decision L.          |                   |                       |                       |
| **LAYER 7** |                   | Overview L.          |                   |                       |                       |

> **MME parallel branch:** Within the Market Monitoring Engine, Layers 4 (Opportunity) and 5 (Risk) are **orthogonal** and execute **in parallel** directly from Layer 3 (Analysis) — neither reads the other. They converge at Layer 6 (Decision). See §2.2.

### 1.1 Horizontal Axis: Specialized Engines
The horizontal axis comprises five decoupled computational engines. Each engine owns one primary quantitative or transactional domain. They maintain zero shared memory or shared states, communicating only via stable, public APIs and read-only message streams.

### 1.2 Vertical Axis: Sequenced Analytical Layers
Within each engine, the vertical axis dictates the step-by-step transformation of raw, low-level data into highly abstract decision-support vectors. Each step is represented by an isolated **Layer** that consumes the preceding layer's output, applies deterministic calculations, and produces a single, immutable, versioned **Matrix** as its official output contract.

---

## 2. The Five Core Engines and Layer Specifications

---

### 2.1 Data Infrastructure Engine (DIE)

The Data Infrastructure Engine is responsible for the ingest, normalization, validation, and real-time distribution of exchange data feeds.

```
[Exchange APIs] -> (Raw Data Layer) -> (Market Data Layer) -> (Data Quality Layer) -> (Distribution Layer) -> [To MME]
```

#### Layer 1: Raw Data Layer
*   **Purpose:** Establish and maintain low-level socket and REST connections to external venues.
*   **Processing:** Read raw network frames, manage rate-limiting constraints, handle reconnection protocols, and buffer incoming byte streams.
*   **Output (Raw Data Matrix):** Standardized JSON or binary representations of raw exchange events (ticks, trades, order book steps).

#### Layer 2: Market Data Layer
*   **Purpose:** Transform raw event-based feeds into structured, uniform temporal boundaries.
*   **Processing:** Aggregate trade events and book snapshots into standardized OHLCV (Open, High, Low, Close, Volume) bars across target intervals.
*   **Output (Market Data Matrix):** Uniform, multi-timeframe candle data per symbol.
*   **Strict UTC-Alignment Constraint (Zero-Drift Synchronization):** All time-boundary candle aggregations synchronize strictly with the UTC daily clock. The closing instant of any candle aligns to the exact epoch-duration multiple of UTC, computed deterministically as `interval_start = ⌊timestamp_ms / duration_ms⌋ × duration_ms` (so a `micro60` candle closes at `:00.000` of the next minute; a `macro900` candle closes at `:15:00.000`, `:30:00.000`, `:45:00.000`, or `:00:00.000`). Local server system clocks execute continuous NTP polling to maintain local system time drift under $\le 50 \text{ microseconds}$ of UTC. Drift enforcement is implemented in `crates/engine/src/clock_monitor.rs` (spawned as a continuous background task by `main.rs` after engine initialization and before live ingestion, polling NTP every 30 s; configured via the `"clock_monitor"` block of `config.json`). This prevents timezone, socket, or aggregation-time drift, ensuring local indicator values align exactly with exchange historical benchmarks. See [08-06-clock-monitor.md](../operations-and-compliance/08-06-clock-monitor.md) for the full lifecycle and breach handling.

#### Layer 3: Data Quality Layer
*   **Purpose:** Enforce data integrity and detect stream anomalies.
*   **Processing:** Audit historical sequences for missing bars, parse for stale tick values, filter spikes, and flag out-of-order execution sequence IDs.
*   **Output (Data Quality Matrix):** Sanitized, gap-filled market datasets paired with reliability metrics.

#### Layer 4: Data Distribution Layer
*   **Purpose:** Route verified data streams to downstream system consumers.
*   **Processing:** Manage pub/sub event loops, prioritize routing queues, and minimize dispatch latency.
*   **Output (Distribution Matrix):** High-throughput, real-time data output channels.

---

### 2.2 Market Monitoring Engine (MME)

The Market Monitoring Engine transforms standardized market data into multi-timeframe, contextual technical intelligence, isolating trend interpretation, opportunity, and risk.

```
                                                 ┌─> (L4: Opportunity Layer) ─┐
                                                 │                            │
[Market Data] -> (Metrics L.) -> (Alignment L.) -> (Analysis L.) ─┤                            ├─> (L6: Decision L.) -> (L7: Overview L.)
                                                 │                            │
                                                 └─> (L5: Risk Layer) ────────┘
```

Layers 4 and 5 read the Analysis Matrix independently and run in parallel (orthogonal dimensions); they converge only at Layer 6.

#### Layer 1: Metrics Layer
*   **Purpose:** Compute single-timeframe technical metrics and project them across standardized multidimensional context dimensions.
*   **Processing:** Compute technical indicators (EMA, RSI, ATR) and detect single-timeframe technical signals (including price-to-indicator divergences, candlestick patterns, and structural breaks). Project each indicator across its standard Indicator Evaluation Axes (Value, State, Direction, Strength, Market Regime, Confidence, Freshness, Quality) and each signal across its standard Signal Evaluation Axes (Signal Type, Direction, Strength, Confidence, Freshness, Confirmation, Market Regime, Multi-Timeframe Agreement, Risk, Priority).
*   **Output (Metrics Matrix):** Structured `IndicatorEvaluation` and `SignalEvaluation` telemetry objects, along with compiled localized analytical features.

#### Layer 2: Alignment Layer
*   **Purpose:** Evaluate spatial-temporal consensus across multiple time horizons.
*   **Processing:** Calculate alignment indices between micro, fast, slow, and macro timeframes for trend direction, momentum vectors, and key support/resistance blocks.
*   **Output (Alignment Matrix):** Unified multi-timeframe alignment and structural confluence scores.

#### Layer 3: Analysis Layer
*   **Purpose:** Formulate the core structural interpretation of an asset.
*   **Processing:** Diagnose the dominant trend bias, represented qualitatively as a categorical classification and quantitatively as a continuous Market Bias Score normalized between `-1.0` (absolute bearish) and `+1.0` (absolute bullish). Identify the structural regime (trending vs. ranging), assess the current cycle phase, and evaluate overall trend quality.
*   **Output (Analysis Matrix):** Standardized classifications of the asset's technical environment and directional bias indexes.

#### Layer 4: Opportunity Layer
*   **Purpose:** Identify and score positive market configurations.
*   **Processing:** Evaluate structural setups (such as breakout pressure, pullback depth, and continuation vectors) to determine the statistical viability of potential entries.
*   **Output (Opportunity Matrix):** Strategy-agnostic opportunity classifications with associated confidence scores (0-100).

#### Layer 5: Risk Layer
*   **Purpose:** Quantify environmental danger and exposure conditions, independent of direction.
*   **Processing:** Compute localized volatility parameters, measure book liquidity depth, evaluate proximity to major invalidation barriers, and assess signal divergence.
*   **Output (Risk Matrix):** Comprehensive risk indices across **eight unipolar danger sub-dimensions** (`market_risk`, `volatility_risk`, `execution_liquidity_risk`, `structure_risk`, `momentum_risk`, `signal_risk`, `execution_risk`, `cascade_risk`) plus `overall_risk`. *The previously-listed 8th sub-dimension was historically referred to as both "correlation risk" and "reward risk" — both terms are stale references to the same retired concept. The dimension was removed in the institutional redesign; its semantic successor is `entry_danger` (renamed from `environment_favorability`) in the Decision Matrix, and correlated-downside danger is now captured by the cross-symbol `systemic_risk_score` at L7 (see [Overview Matrix §4](../matrices/02-09-overview-matrix.md)).*

#### Layer 6: Decision Layer
*   **Purpose:** Synthesize bias, opportunity, and risk into strategic decision support.
*   **Processing:** Grade overall trade readiness, determine strategy-compatibility scores, and map dynamic protection (stop-loss percentage distance) and target zones.
*   **Output (Decision Matrix):** High-value tactical blueprints containing scenario pathways and structural invalidation parameters.

#### Layer 7: Overview Layer
*   **Purpose:** Synthesize cross-symbol market data into global systemic context.
*   **Processing:** Measure market-wide breadth vectors, compile leaderboards of asset strength/weakness, and aggregate risk variables into a single systemic risk score.
*   **Output (Overview Matrix):** Global market intelligence summaries used for portfolio-level asset allocation.

---

### 2.3 Trade Automation Engine (TAE)

The Trade Automation Engine evaluates user-defined execution rules and coordinates order execution with external venues.

```
[MME Decision] -> (Policy Layer) -> (Execution Layer) -> [Exchange APIs]
```

#### Layer 1: Policy Layer
*   **Purpose:** Evaluate real-time market intelligence against programmable automation rules.
*   **Processing:** Map incoming symbol-specific Decision Matrices to active user configurations, validate trigger boundaries, and check symbol-level stances (Active, Close Only, Avoid).
*   **Output (Policy Matrix):** Active policy directives containing target direction, entry parameters, and protective criteria.

#### Layer 2: Execution Layer
*   **Purpose:** Route transactional orders and manage trade lifecycles on live exchanges or simulated execution environments (such as paper trading engines).
*   **Processing:** Execute the Position Sizing Protocol upon trade entry validation. Query PME Capital Matrix for Available Margin ($E$) and MME Decision Matrix for Stop-Loss Distance as a raw percentage float ($D_{sl}$, e.g. `1.5`). Calculate the exact trade size ($S$) based on the user-configured risk-per-trade fraction ($R$, e.g. $0.01$ = 1% of margin; with the default `risk_per_trade_pct = 1.0`, $R = 0.01$):
    $$S = \frac{E \times R}{D_{sl} / 100}$$
    Construct order packets, apply slippage filters against real-time order books, dispatch execution messages to the target venue, and track order execution states.
*   **Output (Execution Matrix):** Structured database of outstanding, filled, modified, and cancelled orders.

---

### 2.4 Portfolio Management Engine (PME)

The Portfolio Management Engine manages capital safety boundaries, tracks asset exposures, and maintains active ledger accounts.

```
[Fills & Fails] -> (Position Layer) -> (Exposure Layer) -> (Capital Layer) -> (Portfolio Layer) -> [TAE / UI]
```

#### Layer 1: Position Layer
*   **Purpose:** Track active open positions and calculate live valuation states.
*   **Processing:** Update average entry prices, monitor mark-to-market valuations, compute active unrealized PnL, and coordinate dynamic stop adjustments with external fills.
*   **Output (Position Matrix):** Directory of active positions with high-frequency valuation metrics.

#### Layer 2: Exposure Layer
*   **Purpose:** Group and limit active capital allocations across correlated boundaries.
*   **Processing:** Aggregate gross and net allocations across symbols, sectors, and correlated currency assets to prevent concentration risk.
*   **Output (Exposure Matrix):** Consolidated risk allocation and concentration metrics.

#### Layer 3: Capital Layer
*   **Purpose:** Monitor overall account solvency and balance distributions.
*   **Processing:** Manage available balances, track margin usage, evaluate leverage limits, and record fee impacts.
*   **Output (Capital Matrix):** High-frequency balance sheet of active and available capital.

#### Layer 4: Portfolio Layer
*   **Purpose:** Consolidate active position, exposure, and capital matrices into a unified ledger and enforce absolute account-level safety parameters.
*   **Processing:** Synthesize child matrices to track account health. Enforce aggregate portfolio safety rules (e.g., maximum daily drawdown thresholds). If a systemic threshold is breached, execute **Veto Power**: override the Trade Automation Engine's active stances, immediately setting affected symbol stances to `Avoid` or `Close Only` at the execution boundary to reject new entry trigger payloads.
*   **Output (Portfolio Matrix):** The master financial state ledger of the trading account.

---

### 2.5 Performance Analytics Engine (PAE)

The Performance Analytics Engine evaluates historical trading records to isolate strategy efficacy and identify system drag.

```
[Historical Ledgers] -> (Trade Analytics L.) -> (Strategy Analytics L.) -> (Risk Analytics L.) -> (Performance L.)
```

#### Layer 1: Trade Analytics Layer
*   **Purpose:** Reconstruct completed trading events.
*   **Processing:** Extract entry and exit timestamps, calculate exact net holding times, parse execution slippage values, and measure peak trade deviations (MFE, MAE).
*   **Output (Trade Analytics Matrix):** Normalized ledger of reconstructed, closed trades.

#### Layer 2: Strategy Analytics Layer
*   **Purpose:** Group and evaluate closed trade performance segmented by originating Execution Policy, calculating mathematical significance against random market noise.
*   **Processing:** Parse trade performance by policy ID. Calculate standard win rates, gross profit factors, trade expectancy, and average win/loss ratios. Perform Null Hypothesis Significance Testing (NHST) against the baseline hypothesis ($H_0$) that strategy returns are a product of random chance: compute the T-Statistic and P-Value of the strategy's return distribution, and run Monte Carlo sign-randomized baseline samples (each trade's PnL multiplied by a fair-coin ±1) to output the empirical significance probability ($p_{mc}$).
*   **Output (Strategy Analytics Matrix):** Performance metrics and statistical significance parameters segmented by strategy and policy type.

#### Layer 3: Risk Analytics Layer
*   **Purpose:** Evaluate historical drawdown patterns and risk-adjusted metrics.
*   **Processing:** Track equity drawdowns, evaluate recovery timelines, and compute normalized risk-adjusted performance values (Sharpe, Sortino, Ulcer Index).
*   **Output (Risk Analytics Matrix):** Historical risk metrics representing overall system safety.

#### Layer 4: Performance Layer
*   **Purpose:** Correlate performance against historical market conditions.
*   **Processing:** Cross-reference strategy results with recorded market regimes to map compatibility and generate metric-driven optimization inputs.
*   **Output (Performance Matrix):** The complete performance and regime compatibility profile of the trading system.

---

## 3. Communication Model and Decoupling Boundaries

Engines maintain high operational efficiency by communicating through strictly defined, contract-based boundaries.

```
       UNIDIRECTIONAL CASCADE MODEL
       
  +------------------+     [Market Data Matrix]
  |  Data Infra.     |------------------------------+
  |  Engine (DIE)    |                              |
  +------------------+                              v
                                           +------------------+
                                           |  Market Monitor  |
                                           |  Engine (MME)    |
                                           +------------------+
                                                    |
                                                    | [Decision Matrix]
                                                    v
  +------------------+  [Capital Matrix]   +------------------+
  |  Portfolio Mgmt. |-------------------->|  Trade Auto.     |
  |  Engine (PME)    |                     |  Engine (TAE)    |
  +------------------+                     +------------------+
          |                                         |
          | [Closed Trade Logs]                     | [Execution events]
          v                                         v
  +------------------+                              |
  |  Performance     |<-----------------------------+
  |  Analytics (PAE) |
  +------------------+
```

### 3.1 Unidirectional Stream Cascade
The platform prohibits bidirectional dependency chains. Execution details do not influence market interpretation; instead, normalized telemetry cascades forward. If a downstream engine requires information from an upstream engine, it subscribes to that engine's published, read-only matrix stream.

### 3.2 Decoupled API Interfaces
Engines interact using two communication methods:
1.  **Publish/Subscribe (Event Stream):** Used for real-time, continuous streams of data (such as sending ticks from DIE, decision matrices from MME, or position valuations from PME).
2.  **Request/Response (Service Call):** Used for single operations or specific state queries (such as dispatching an order packet or retrieving portfolio parameters).

### 3.3 Zero Shared State
Engines must run in separate memory structures or isolated processes. No engine is permitted to query another engine's private database tables or manipulate its internal runtime variables. All exchange of information is governed strictly by the public matrices defined.

> **Documented exception: TAE–PME in-process sizing query.** The TAE Execution Layer and the PME Capital Layer share state via an in-process `tokio::sync::RwLock` over the in-memory `Capital Matrix` (no IPC, no SQLite round-trip on the sizing hot path). This is a deliberate latency-driven compromise — the Position Sizing Protocol must complete in microseconds, which is incompatible with cross-process RPC or DB round-trips. See [03-03-03-tae-layer2-execution.md §2.0](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) for the synchronization contract. Migration to out-of-process isolation is on the post-v2.2 roadmap.

### 3.4 Documented Exception: MME L5 Multi-Source Input

MME Layer 5 (Risk) consumes the L3 Analysis Matrix **and** the L2.5 LiquidationClusterMatrix (Phase 0-4 Liquidity Intelligence extension). The unidirectional invariant is preserved: L2.5 does not read from L5; L5 → L6 remains unidirectional. This is the only multi-source MME input. See [03-02-11-mme-liquidity-extension.md](../engines/market-monitoring-engine/03-02-11-mme-liquidity-extension.md) for the cascade invariant.

---

## 4. Platform Serialization and Dual-Execution Architecture (GUI vs. CLI)

The platform enforces absolute system portability and reproducibility. Any strategy, configuration profile, or portfolio state established within the graphical interface can be serialized and run headlessly in the cloud.

```
      +--------------------------------------------------+
      |               Graphical UI (GUI)                 |
      |   (Research, Exploration, Strategy Config)       |
      +--------------------------------------------------+
                               |
                   [EXPORT CONFIGURATION JSON]
                               |
                               v
      +--------------------------------------------------+
      |               Headless CLI Engine                |
      |   (Low-Overhead Execution, live/paper trades)    |
      +--------------------------------------------------+
                               |
                   [STREADS METRIC TELEMETRY]
                               v
                     Shared SQL/Time-Series DB
                               ^
                               | [READ TO ANALYZE]
      +------------------------+-------------------------+
      | Graphical UI (Retroactive Analytics & Diagnostics)|
      +--------------------------------------------------+
```

### 4.1 Configuration Portability Principle
To ensure that an execution profile developed on a local setup runs identically in a remote headless environment, all system configurations are exported into a single, unified JSON file. This serialization contract captures:
*   **The Global Configuration:** API thresholds, database targets, network rates, and DIE adapters.
*   **Symbol Instances:** The exact list of target trading pairs (e.g., `BTCUSDT`, `ETHUSDT`) and their mapped timeframes.
*   **The Strategy Profile:** Standardized metrics configs, indicator evaluation criteria, opportunity and risk evaluation targets, stop/target parameters, and active TAE execution policies.

### 4.2 GUI Mode (Exploration and Research)
*   **Purpose:** The main interface for interactive development, validation, and optimization of trading setups.
*   **Operation:** Boots all five engines with full graphical visualization modules. Users load assets, visualize indicator axes, paper-trade live streams to prove a statistical edge (alpha), adjust safety guidelines, and export the finalized environment payload.

### 4.3 CLI Mode (Headless Automated Execution)
*   **Purpose:** High-performance, zero-overhead execution designed for cloud environments.
*   **Operation:** Operates purely headlessly with no visual interface. Upon initialization, it consumes the standardized JSON configuration file, boots the DIE, MME, TAE, and PME internally, constructs the defined pair pipelines, and executes pre-configured live or paper trades automatically.
*   **Boundaries:** The CLI mode is restricted to loading and applying previously validated configuration payloads. It is strictly banned from exploratory research, manual pair configurations, or manual visualization task processing.

### 4.4 Shared Persistence & Retroactive Visualization
Both modes write metrics, signals, orders, and execution events to a shared SQL/Time-Series database.
*   During active CLI headless operation, all transactional and analytical matrices are persisted in real time.
*   At any later point, the operator can boot the GUI application. The GUI reads these persisted records from the database, allowing the **Performance Analytics Engine (PAE)** to run retroactive trade reconstructions, significance tests ($P$-Values, $t$-statistics), and performance evaluations of the cloud-running strategy.

> **Matrix invariance.** The CLI mode emits the **same** matrices as the GUI mode — the WebSocket `MarketSnapshot` envelope carries the full cascade (Metrics → Alignment → Analysis → Opportunity → Risk → Decision), and the Overview Matrix is broadcast on a separate channel. The CLI/GUI split is purely about the rendering / operator surface; the matrix contract is invariant across modes.

> **Operational setup for cloud + local GUI.** When the CLI engine runs headlessly in the cloud (VPS, container, or remote server), the SQLite database (`./telemetry.db`) is local to that instance. The local GUI accesses the engine via **SSH port-forwarding** — e.g. `ssh -L 3000:127.0.0.1:3000 user@cloud-host` so the local browser sees `http://127.0.0.1:3000` as the engine's API. Alternatively, the operator can run a local engine instance with a synced copy of `telemetry.db` (via `rsync`, `scp`, or a cloud-synced volume like `rclone`) and point the local engine at the synced DB. The platform assumes one of these two operational setups when running in cloud-headless mode.

---

## 5. End-to-End Operational Flow

To illustrate the complete pipeline in practice, below is the sequence of events that occurs when a market movement triggers a trade and is subsequently logged by the analytics engine:

1.  **Ingest:** A rapid tick update occurs on the BTCUSDT exchange. The Data Infrastructure Engine (DIE) ingests the event via the *Raw Data Layer*, packages it as standard OHLCV data in the *Market Data Layer*, validates it in the *Data Quality Layer*, and broadcasts the updated **Market Data Matrix**.
2.  **Telemetry:** The Market Monitoring Engine (MME) receives the update. The *Metrics Layer* recalculates indicators and detects signals, immediately projecting them onto their respective multi-dimensional Evaluation Axes (e.g., extracting State, Direction, Strength, and Quality) and updating the **Metrics Matrix**.
3.  **Consensus:** The *Alignment Layer* checks for trend agreement across micro, fast, and slow time horizons, updating the **Alignment Matrix**.
4.  **Diagnosis:** The *Analysis Layer* confirms a transition to a `TRENDING_BULL` regime under a `STRONG_BULLISH` bias (with a `Market Bias Score: +0.82`), updating the **Analysis Matrix**.
5.  **Opportunity:** The *Opportunity Layer* detects a high-probability breakout setup and logs an `Opportunity Score: 85` in the **Opportunity Matrix**.
6.  **Risk:** The *Risk Layer* consumes the Analysis Matrix (L3) and the underlying indicator map — running in parallel with the Opportunity Layer (L4) and independent of the Opportunity Matrix — assesses close proximity to major support, and logs a low `Overall Risk Score: 28` in the **Risk Matrix**. The Risk Matrix reads Analysis Matrix fields such as `market_quality` (L3) but does *not* consume the L4 Opportunity Matrix itself.
7.  **Decision:** The *Decision Layer* synthesizes these matrices, sets *Trade Readiness* to `READY`, and logs a structural invalidation target and calculated `stop_loss_distance_pct: 1.5` in the **Decision Matrix**.
8.  **Trigger:** The Trade Automation Engine (TAE) receives this decision snapshot. The *Policy Layer* identifies that this state satisfies an active long breakout policy and logs an entry command in the **Policy Matrix**.
9.  **Routing:** The *Execution Layer* queries the PME *Capital Matrix* to check available margin, reads the Stop-Loss Distance from the MME *Decision Matrix*, runs the Position Sizing Protocol to calculate a safe, risk-adjusted position size ($S = \frac{E \times R}{D_{sl} / 100}$), routes the buy order to the exchange or paper trading engine, and records the event in the **Execution Matrix**.
10.  **Supervision:** The Portfolio Management Engine (PME) receives the execution confirmation. The *Position Layer* initializes a new open position in the **Position Matrix**. The *Exposure Layer* recalculates directional risk, the *Capital Layer* locks margin, and the *Portfolio Layer* updates the account's master balance vector, running continuous safety checks to verify that the portfolio-wide drawdown ceiling has not been breached, triggering a veto clamp if required.
11. **Analysis:** Upon a subsequent exit trigger, the trade is closed. The Performance Analytics Engine (PAE) imports the closed log into the *Trade Analytics Layer*, calculates performance metrics and runs statistical significance calculations (P-Value, T-Stat, Monte Carlo sign-randomization) in the *Strategy Analytics Layer*, measures drawdown impact in the *Risk Analytics Layer*, and updates the *Performance Matrix* to refine the strategy-to-regime compatibility maps. If the trade was run headlessly via CLI Mode, these entries are persisted to the database; the operator boots the GUI retroactively to display and analyze these metrics.

---

## 6. Hybrid Memory and Math Architecture (DOD vs. OOP)

> **Target Architecture (Not Yet Implemented).** This section specifies a planned execution model. It does **not** reflect the current implementation, which computes most indicators in `rust_decimal::Decimal` over `VecDeque` buffers, carries telemetry in a `HashMap`-keyed `MarketSnapshot` (with `Decimal` OHLCV), distributes it over Tokio `broadcast` channels as cloned structs, and performs position sizing in `f64` (`crates/engine/src/risk_calculator.rs`). The design below is the target to migrate toward; it is recorded here so the intended boundary is unambiguous.

To achieve microsecond-level analytical throughput without sacrificing penny-perfect financial safety, the platform draws a strict boundary between two execution models.

### 6.1 The Hot Path — Data-Oriented Design (`f64` / `f32` primitives)

Scope: **DIE ingestion + MME Layers 1–5.**

- Volatile ticks, order-book deltas, and the 50 indicator arrays are packed into cache-aligned, contiguous memory blocks using a **Structure of Arrays (SoA)** layout.
- Rolling histories live in **pre-allocated arena buffers / object pools**, reclaimed rather than freed, to eliminate heap fragmentation and allocator pauses on the analytical loop.
- Calculations run on native floating-point primitives so the compiler can **auto-vectorize (SIMD, AVX/SSE)** and drive hardware FPUs directly.
- Indicator lookup avoids string hashing: the metrics frame is a flat, enum-indexed array (`[IndicatorEvaluation; 50]`) rather than a `HashMap<String, …>`.

### 6.2 The Cold Path — OOP / Domain-Driven Design (`Decimal` precision)

Scope: **TAE, PME, and the relational database.**

- Order routing, position-ledger balancing, margin allocation, and fee accrual use 128-bit fixed-point decimals (`rust_decimal::Decimal`) to prevent rounding drift and exchange rejections.
- Domain objects (Order, Position, Portfolio) encapsulate invariants and are the single source of truth for account state.

### 6.3 The Type-Boundary Handoff

The transition occurs at **MME Layer 6 (Decision Support)**:

1. Layer 6 receives fast, raw `f64` analytics and resolves them into trade readiness.
2. It emits the **Decision Matrix** carrying the stop-loss distance as a raw `f64` — `stop_loss_distance_pct` (e.g. `1.5`, meaning 1.5%).
3. The **TAE Execution Layer** pulls that `f64`, pulls **available margin** ($E$) from the PME Capital Matrix as a `Decimal`, and safely converts the float to `Decimal` at the entry boundary before executing the Position Sizing Protocol with transactional precision:

   ```rust
   // Type-boundary conversion (target design)
   let d_sl          = Decimal::from_f64_retain(stop_loss_distance_pct / 100.0)?; // 1.5 → 0.015
   let risk_fraction = Decimal::from_f64_retain(risk_per_trade_pct     / 100.0)?; // 1.0 → 0.010
   let size          = (available_margin * risk_fraction) / d_sl;                  // all Decimal
   ```

   Equivalently, $S = \dfrac{E \times R}{D_{sl} / 100}$ with $E$ = available margin (Decimal), $R = \text{risk\_per\_trade\_pct} / 100$ (the **fraction** form: $R \in [0, 1]$; the raw user-facing value `risk_per_trade_pct` is divided by `100` to obtain $R$), and $D_{sl}$ = stop-loss distance as a raw percentage float.

   > **Variable-naming hazard (correction).** A previous snippet used `risk_pct` in the multiplication — this is a 100× over-size hazard if `risk_pct` carries the raw-percent float (`1.0`) instead of the fraction (`0.01`). The canonical variable name for the fraction is `risk_fraction`.