# Systemic Data Flow Specification

**Version:** 2.0  
**Status:** Approved  
**Purpose:** This document details the chronological, systemic data flows across the five core engines of the Trading Platform. It specifies the step-by-step path of telemetry as it transforms from raw exchange events into structured market intelligence, automated order routing, active portfolio tracking, and post-trade performance analytics.

---

## 1. Flow Governance and Unidirectional Design

The platform enforces a strict **Unidirectional Stream Cascade** model. Information must only move forward through the designated pipeline to prevent logical circular dependencies, reduce race conditions under high-frequency conditions, and ensure absolute trace reproducibility.

```
+-------------------+      +-------------------+      +-------------------+
|    Data Ingest    |      | Market Monitoring |      | Trade Automation  |
|   Engine (DIE)    | ===> |   Engine (MME)    | ===> |   Engine (TAE)    |
+-------------------+      +-------------------+      +-------------------+
                                                                |
                                                                v
+-------------------+      +-------------------+      +-------------------+
| Performance Anal. |      | Portfolio Mgmt.   |      |  Execution Venue  |
|   Engine (PAE)    | <=== |   Engine (PME)    | <=== | (Exchange/Paper)  |
+-------------------+      +-------------------+      +-------------------+
```

### 1.1 Core Flow Rules
*   **Rule 1: Forward Cascade Only:** Upstream matrices are entirely blind to downstream states. The Market Monitoring Engine (MME) cannot query the Portfolio Management Engine (PME) to alter its calculation of market bias or risk scores.
*   **Rule 2: Decoupled Inter-Engine State:** State updates are exchanged exclusively via stable, typed read-only matrices over high-speed message buses. There is no shared memory, and direct database queries from one engine's private store to another's are prohibited.
*   **Rule 3: Restricted Read-Only Sizing Feedback:** The only exception to the forward cascade is a controlled, read-only pull request during execution routing, where the Trade Automation Engine (TAE) reads the current capital metrics from the PME to calculate order sizing.

---

## 2. High-Level Stream Cascade

The sequence below illustrates the communication boundaries and matrix exchanges across the system.

```
Exchange       DIE           MME           TAE           PME           PAE
   |            |             |             |             |             |
   |--[Ticks]-->|             |             |             |             |
   |            |--[Candles]->|             |             |             |
   |            |             |--[Decision]>|             |             |
   |            |             |             |--[Capital]? |             |
   |            |             |             |<--[Matrix]--|             |
   |            |<================[Orders]--|             |             |
   |<--[Fills]--|             |             |             |             |
   |--[Events]===========================================>|             |
   |            |             |             |             |--[Logs]---->|
```

---

## 3. Detailed Chronological Sequences

---

### Sequence A: Market Telemetry & Analysis Cascade (The Observation Loop)

This sequence runs continuously on every received tick to build symbol-specific and market-wide intelligence.

```
Exchange                DIE                                    MME
   |                     |                                      |
   |--[Raw WS Tick]----->|                                      |
   |                     |--[Layer 1: Standardize Network]----->|
   |                     |--[Layer 2: Standardize Candle]------>|
   |                     |--[Layer 3: Gap-Fill / Quality Check]->|
   |                     |                                      |
   |                     |=====[Publish: Market Data Matrix]====>|
   |                     |                                      |--[Layer 1: Multi-Axis Projection]
   |                     |                                      |--[Layer 2: Multi-Timeframe Align]
   |                     |                                      |--[Layer 3: Bias & Regime Diag.]
   |                     |                                      |
   |                     |                                      |  BIFURCATION POINT (PARALLEL RUN)
   |                     |                                      ├──[Layer 4: Opportunity Profiling (L3 input)]
   |                     |                                      └──[Layer 5: Unipolar Risk Scoring (L3 input)]
   |                     |                                      |
   |                     |                                      |  CONVERGENCE POINT
   |                     |                                      |--[Layer 6: Decision Synthesis (L4+L5 input)]
   |                     |                                      |--[Layer 7: Systemic Breadth (L6 input)]
   |                     |                                      |
   |                     |                                      |=====[Publish: Decision Matrix]====> [TAE]
```

#### Detailed Operations:
1. **DIE Ingestion:** The exchange socket pushes raw trades and order book updates. The DIE standardizes the network frame at Layer 1 and groups updates into uniform time intervals (OHLCV) at Layer 2.
2. **Quality Verification:** Layer 3 validates sequence integrity, cleans bad ticks, and publishes the immutable **Market Data Matrix**.
3. **MME Multi-Axis Projection:** The MME reads the Market Data Matrix. Layer 1 calculates indicators and signals, projecting them onto their standardized **Evaluation Axes** (e.g., converting RSI to a structured object containing Value, State, Direction, and Strength).
4. **Consensus & Regime Diagnosis:** Layer 2 measures cross-timeframe alignment scores. Layer 3 evaluates these inputs to determine the categorical `market_bias` and computes the continuous numeric `market_bias_score` (between $-1.0$ and $+1.0$).
5. **Opportunity Scoring:** Layer 4 evaluates specific strategy-agnostic opportunities (0-100 score) based on the Analysis Matrix, running in parallel with Layer 5.
6. **Risk Scoring:** Layer 5 consumes the Analysis Matrix (L3) and the underlying indicator map — running **in parallel with Layer 4 and independent of the opportunity score** — to evaluate multidimensional unipolar risk across nine dimensions (market, volatility, liquidity, structure, momentum, signal, execution, reward, and overall risk), compiling the overall risk score.
7. **Guidance and Overview Compilation:** Layer 6 is the convergence boundary: it merges the parallel Opportunity and Risk branches (plus directional bias) into a single symbol's **Decision Matrix** (trade readiness, stop-loss distance, and scenario pathways). Layer 7 aggregates all symbols into the global **Overview Matrix** (breadth ratios and Systemic Risk Score).

---

### Sequence B: Order Sizing & Automated Execution Loop (The Entry Loop)

This sequence is initiated when the Decision Matrix declares an asset is ready for trade entry.

```
 MME                   TAE                                    PME                  Exchange
  |                     |                                      |                      |
  |===[Decision Mat.]==>|   (stop_loss_distance_pct as f64)    |                      |
  |                     |--[Layer 1: Policy Validation]        |                      |
  |                     |                                      |                      |
  |                     |-----[Query: Capital Matrix Balance]->|                      |
  |                     |<----[Return: Available Margin (E)]---|  (Decimal)           |
  |                     |                                      |                      |
  |     ===== TYPE BOUNDARY: f64 → Decimal cast here =====     |                      |
  |                     |--[Layer 2: Position Sizing Protocol] |                      |
  |                     |        S = (E * R) / (D_sl / 100)    |                      |
  |                     |                                      |                      |
  |                     |--[Route Signed API Order Packet]===========================>|
  |                     |                                      |                      |<--[Fill Confirmed]
  |                     |<=[Order State & Execution Matrix]===========================|
```

> **Target Architecture (Not Yet Implemented).** The dashed **type boundary** above is where the fast analytical hot path (`f64`) meets the precise financial cold path (`Decimal`). In the target design the MME Decision Matrix passes `stop_loss_distance_pct` as an `f64`, the TAE pulls available margin from the PME as a `Decimal`, and the TAE casts the float to `Decimal` before sizing so the protocol runs entirely in fixed-point:
> $$S\,(\text{Decimal}) = \frac{E\,(\text{Decimal}) \times R\,(\text{Decimal})}{D_{sl}\,(\text{Decimal}) / 100}$$
> *Current implementation:* the sizing math in `risk_calculator.rs` runs in `f64` end-to-end; the boundary cast is not yet in place.

#### Detailed Operations:
1. **Policy Evaluation:** The TAE Policy Layer consumes the MME **Decision Matrix**. It maps the values to user configurations. If the stance is `Active` and the decision state satisfies entry triggers, a buy signal is dispatched to the Execution Layer.
2. **Dynamic Capital Query:** The TAE Execution Layer issues a synchronous request-response query to the PME Capital Layer to retrieve the current account Available Margin ($E$).
3. **Position Sizing Calculation:** The TAE Execution Layer retrieves the Stop-Loss Distance Percentage ($D_{sl}$, a raw percentage float such as $1.5$) from the MME Decision Matrix and pulls the user-defined Risk-Per-Trade fraction ($R$, e.g., $0.01$ = 1%). At this **type boundary** (target design) it casts the `f64` stop-loss distance to `Decimal` and combines it with the `Decimal` available margin, running the **Position Sizing Protocol** in fixed-point:
   $$S = \frac{E \times R}{D_{sl} / 100}$$
4. **Order Transmission:** The TAE signs the calculated order payload and dispatches it to the live exchange API or simulated paper trading matching engine. It logs transaction state transitions to the **Execution Matrix** until receiving confirmation of execution fill.

---

### Sequence C: Active Risk & Capital Supervision (The Active Loop)

Once an order is filled, the active exposure lifecycle is handed over to the PME for tracking and preservation.

```
Exchange                 PME (Position & Exposure)              MME (Decision Support)
   |                                 |                                    |
   |=====[Order Fill Event]=========>|                                    |
   |                                 |--[Layer 1: Initialize Position]    |
   |                                 |--[Layer 2: Update Net Exposure]    |
   |                                 |                                    |
   | <---[Continuous Mark-Price]-----|                                    |
   |                                 |--[Live UnPnL Valuation updates]    |
   |                                 |                                    |
   |                                 |<======[Update Invalidation Stops]--|
   |                                 |--[Adjust Dynamic Stop orders]      |
```

#### Detailed Operations:
1. **Position Initialization:** PME receives execution events from the transaction venue. Layer 1 initializes the trade metrics (volume-weighted entry price, size, and initial stop limits) in the **Position Matrix**.
2. **Exposure Recalculation:** Layer 2 aggregates net and gross exposure limits across correlated asset pairs and sectors, writing the results to the **Exposure Matrix** to prevent concentration breaches.
3. **Mark-to-Market Tracking:** The DIE continuously feeds current mark-prices to PME. Layer 1 updates active valuation fields, calculating dynamic unrealized PnL and active ROI.
4. **Dynamic Stop Management:** As the MME analyzes market structure updates, it publishes adjusted invalidation levels in the Decision Matrix. The PME Position Layer reads these levels and dynamically updates stop-loss coordinates on the exchange to lock in open equity.

---

### Sequence D: Systemic Safety Veto (The Circuit Breaker Loop)

This safety loop operates continuously in the background. It intercepts and overrides active trading stance authorizations when systemic thresholds are crossed.

```
 PME (Portfolio Layer)            TAE (Policy Layer)          MME (Overview Matrix)
         |                                |                            |
         |                                |<====[Systemic Risk Score]--|
         |--[Compute Drawdown & Margin]   |                            |
         |                                |                            |
         |====[VETO TRIGGERED]===========>|                            |
         |    (Stance forced to AVOID)    |                            |
         |                                |--[Nullify Entry Triggers]  |
         |                                |--[Cancel Pending Orders]   |
```

#### Detailed Operations:
1. **Systemic Health Evaluation:** The PME Portfolio Layer continuously monitors total unrealized losses, aggregate margin usage, and account balance values. Concurrently, it reads the Systemic Risk Score from the MME **Overview Matrix**.
2. **Veto Trigger:** If the portfolio drawdown limit is breached (e.g., net equity drops more than 5% within a defined rolling window) or the Systemic Risk Score exceeds safety tolerances:
   * PME asserts **Ontological Priority (Veto Power)**.
   * It publishes a high-priority state override message to the TAE.
3. **Execution Blockade:** The TAE Policy Layer processes the override. It immediately changes the symbol stance states to `Avoid` or `Close Only`.
4. **Order Cancellation:** The TAE Execution Layer intercepts any pending trigger messages, discards fresh entry attempts at the boundary, and issues batch cancellation orders for any outstanding limit orders on the exchange.

---

### Sequence E: Settlement, Ingestion & Performance Reconstruction (The Analytics Loop)

This sequence runs asynchronously when a position is liquidated and closed on the exchange.

```
Exchange                 PME                            PAE
   |                      |                              |
   |===[Exit Fill Event]=>|                              |
   |                      |--[Layer 1: Close Position]   |
   |                      |                              |
   |                      |=====[Publish Trade Logs]====>|
   |                      |                              |--[Layer 1: Reconstruct Trades]
   |                      |                              |--[Layer 2: Run NHST Significance]
   |                      |                              |--[Layer 3: Drawdowns & Sharpe]
   |                      |                              |--[Layer 4: Regime Performance Mapping]
```

#### Detailed Operations:
1. **Exposure Clearing:** PME receives confirmation of the exit order's fill. The Position Layer closes the active trade, clears locked margin balances, and archives the trade metrics.
2. **Log Dispatch:** PME writes the completed execution records, time-series mark records, and associated transaction logs to the database, signaling the PAE.
3. **Trade Reconstruction:** PAE Layer 1 processes the database logs to reconstruct the complete trade timeline, computing hold durations, MAE, MFE, and actual execution slippage overhead.
4. **Statistical Significance Testing:** PAE Layer 2 runs **Null Hypothesis Significance Testing (NHST)** on the strategy's return distribution, calculating the T-Statistic and P-Value relative to a zero-edge baseline. It executes Monte Carlo sign-randomized baseline runs (each trade's PnL multiplied by a fair-coin ±1) to output the empirical probability ($p_{mc}$).
5. **Regime Mapping:** PAE Layer 3 computes drawdown risk profiles and Sharpe/Sortino performance ratios. Layer 4 maps strategy performance directly to the technical market regimes recorded by MME during the trade, updating the master **Regime Compatibility Matrix** to refine parameter optimization.

---

## 4. Matrix Lifecycle & Performance Targets

To maintain operational integrity across sequences, the platform enforces strict quality and latency SLAs for matrix serialization and propagation.

| Sequence Loop | Initiating Matrix | Terminating Matrix | Max Permissible Latency (SLA) | Serialization Protocol |
| :--- | :--- | :--- | :--- | :--- |
| **Observation Loop** | Raw Data Matrix | Overview Matrix | $< 25 \text{ ms}$ | JSON Schema / Zero-Copy Binary |
| **Execution Entry** | Decision Matrix | Execution Matrix | $< 15 \text{ ms}$ (Excl. Network) | JSON Schema / Strictly Typed |
| **Safety Veto** | Portfolio Matrix | Policy Matrix | $< 2 \text{ ms}$ | High-Priority IPC / Memory Map |
| **Analytics Loop** | Position Matrix | Performance Matrix | Asynchronous (Batch) | JSON Database Persistence |

### 4.1 Immutability Guarantees
Every matrix produced during these sequences is written to the database with a high-resolution timestamp and a sequential version identifier. Once a matrix is committed to the communication bus, it must not be modified. Retrospective adjustments are prohibited; updates are instead represented as a subsequent timestamped matrix version. This guarantees perfect reproducibility of any automated decision or risk evaluation during historical playback.