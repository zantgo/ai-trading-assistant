# Trading Platform Ontology

---

## Chapter 1 — Introduction

### 1.1 Document Purpose
This document establishes the definitive, formal ontology for the Trading Platform (v2). It serves as the single source of truth for all terminology, conceptual models, system boundaries, and architectural definitions across the entire software suite. 

By defining a precise, unambiguous vocabulary, this ontology ensures conceptual alignment across all development phases, architectural specifications, database schemas, API contracts, and user interface designs.

### 1.2 Scope
The scope of this ontology encompasses the entire lifecycle of quantitative and automated trading activities, structured across five specialized, independent business domains:
1. **Data Acquisition and Standardization** (Data Infrastructure Engine)
2. **Market Observation and Intelligence** (Market Monitoring Engine)
3. **Execution Policy and Order Management** (Trade Automation Engine)
4. **Active Exposure and Capital Supervision** (Portfolio Management Engine)
5. **Historical Performance and Strategy Evaluation** (Performance Analytics Engine)

Here is the complete, unabridged content for the missing chapters—**Chapter 2 (Design Philosophy)**, **Chapter 3 (Core Concepts)**, and **Chapter 4 (Information Flow)**—specifically tailored to match the structural architecture, separate Opportunity and Risk matrices, and formal definitions of the **Trading Platform Ontology v2**.

---

## Chapter 2 — Design Philosophy

The Trading Platform is founded on a set of architectural and conceptual principles that define how the system is organized, how information is transformed, and how responsibilities are distributed. These principles are independent of any programming language, implementation, or trading strategy. They represent the permanent design philosophy of the platform and guide every architectural decision.

### 2.1 Purpose
The purpose of the design philosophy is to establish a consistent framework for building, extending, and maintaining the Trading Platform. Rather than solving isolated technical problems, every component of the platform must follow the same conceptual principles. This ensures that the platform remains:
*   Consistent
*   Modular
*   Explainable
*   Predictable
*   Extensible
*   Maintainable

As the platform evolves, new functionality must integrate naturally into the existing conceptual model rather than introducing competing design patterns.

### 2.2 Single Responsibility
Every conceptual component of the platform exists for exactly one purpose. Responsibilities are never duplicated across multiple components. Each responsibility belongs to a single architectural location. 
*   The Data Infrastructure Engine acquires and prepares data.
*   The Market Monitoring Engine understands and interprets the market.
*   The Trade Automation Engine executes configured policies.
*   The Portfolio Management Engine supervises active positions and capital.
*   The Performance Analytics Engine evaluates historical results.

The same principle applies within every engine: each layer performs one analytical transformation, and each matrix represents one analytical output. This separation minimizes coupling and maximizes clarity.

### 2.3 Progressive Abstraction
The platform transforms information through successive levels of abstraction. Each stage adds business meaning and analytical value without discarding the integrity of preceding information. Raw market data gradually becomes structured intelligence. 

Conceptually, the transformation follows this progression:
$$\text{Raw Data} \longrightarrow \text{Observation} \longrightarrow \text{Correlation} \longrightarrow \text{Interpretation} \longrightarrow \text{Opportunity} \longrightarrow \text{Risk} \longrightarrow \text{Decision Support} \longrightarrow \text{Execution Action} \longrightarrow \text{Portfolio State} \longrightarrow \text{Performance Intelligence}$$

Every stage consumes structured information from the previous stage and produces a higher level of understanding. No stage should bypass another.

### 2.4 Hierarchical Organization
The Trading Platform follows a hierarchical conceptual organization. Higher-level concepts coordinate lower-level concepts without replacing their responsibilities. The hierarchy is:
$$\text{Trading Platform} \longrightarrow \text{Engine} \longrightarrow \text{Layer} \longrightarrow \text{Matrix} \longrightarrow \text{Entity} \longrightarrow \text{Feature}$$

*   The **Trading Platform** defines the complete system.
*   An **Engine** owns a business domain.
*   A **Layer** owns an analytical stage.
*   A **Matrix** represents the structured output of that stage.

This hierarchy is applied consistently throughout all technical layers of the platform.

### 2.5 Separation of Concerns
Different business domains remain independent. Market analysis must not execute trades. Trade execution must not calculate indicators or drawdowns. Portfolio management must not interpret market trend bias. Performance analytics must not influence active portfolio risk. Each engine specializes in one domain and communicates only through well-defined, published outputs. This separation reduces complexity while allowing each domain to evolve independently.

### 2.6 Explainability
Every conclusion produced by the platform must be explainable and traceable. The platform must never produce a recommendation, execution command, or risk assessment that cannot be traced back to its supporting evidence. Every high-level decision possesses a complete chain of supporting data:
$$\text{Market Data} \longrightarrow \text{Indicators} \longrightarrow \text{Signals} \longrightarrow \text{Features} \longrightarrow \text{Alignment} \longrightarrow \text{Analysis} \longrightarrow \text{Opportunity/Risk} \longrightarrow \text{Decision Support} \longrightarrow \text{Policy Trigger} \longrightarrow \text{Execution}$$

Explainability is a core design goal of the platform, enabling systematic debugging and historical auditability.

### 2.7 Deterministic Behavior
The platform is designed to behave deterministically. Given identical inputs, the platform must produce identical outputs. Analytical components must avoid hidden states, undefined behavior, or non-reproducible processes. Deterministic behavior provides predictability, testability, and reliability, which are essential for quantitative trading systems.

### 2.8 Composability
Complex behavior emerges from the composition of simple, specialized components. Rather than building large, monolithic processing systems, the platform combines small analytical stages into complete pipelines. Each matrix represents a reusable analytical component; each layer composes multiple concepts into a higher-level representation; and each engine composes multiple layers into a complete business capability.

### 2.9 Encapsulation
Every engine encapsulates its internal implementation. External systems interact only through stable API boundaries. Consumers understand what an engine provides, what information it consumes, and what information it produces, but they do not depend on internal algorithms, localized data structures, or specific library implementations. This allows engines to be upgraded internally without affecting the rest of the platform.

### 2.10 Contract-Based Communication
Information flows between components through structured contracts. Within an engine, each layer communicates through its Matrix. Between engines, communication occurs through stable API contracts:
$$\text{Layer} \longrightarrow \text{Matrix} \longrightarrow \text{Engine API} \longrightarrow \text{Next Engine}$$

This ensures that every analytical transformation has a clearly defined input and output, establishing a shared language across independent components.

### 2.11 Strategy-Agnostic Design
The platform is designed to describe and interpret markets objectively rather than to enforce a single trading methodology. No engine assumes trend-following, mean-reversion, breakout, scalping, or arbitrage logic natively within its telemetry or interpretation layers. Instead, the platform produces structured analytical data that can support many different trading strategies. Trading strategies are external consumers of market intelligence rather than part of the conceptual model itself.

### 2.12 Human-Centered Intelligence
The primary objective of the platform is to produce high-quality, interpretable market intelligence. While the system supports full automation, its analytical outputs are structured to be easily understood by human operators. Every decision vector, risk evaluation, and position state must remain interpretable, ensuring that automation enhances decision-making rather than obscuring it.

### 2.13 Configurable Automation
Automation is intentionally decoupled from market intelligence. The Market Monitoring Engine identifies and evaluates market states, opportunities, and risks. The Trade Automation Engine decides whether those environmental parameters satisfy user-defined execution policies. This separation allows identical market intelligence to support completely different execution behaviors depending on user preferences.

### 2.14 Scalability
The conceptual model is designed for continuous expansion. Future additions—such as new indicators, asset classes, execution venues, portfolio optimization algorithms, or performance metrics—must integrate naturally without requiring structural revisions to the platform's ontology.

### 2.15 Consistency
Concepts must retain the exact same meaning throughout the platform. An Engine always represents a business domain; a Layer always represents an analytical stage; a Matrix always represents a structured analytical output. Terminology is never modified depending on context, reducing ambiguity across documentation, implementation, and user interfaces.

### 2.16 Long-Term Evolution
Architectural decisions prioritize long-term maintainability, reliability, and logical consistency over short-term implementation convenience. The conceptual model remains stable while implementations improve over time.

---

## Chapter 3 — Core Concepts

The Trading Platform is built upon a small set of fundamental concepts that define every component of the system. These concepts form the common language used throughout the architecture, ontology, implementation, documentation, and user interface.

```
       +---------------------------------------------+
       |             Trading Platform                |
       +---------------------------------------------+
                              |
                     +--------v--------+
                     |     Engine      |
                     +--------v--------+
                              |
                     +--------v--------+
                     |     Layer       |
                     +--------v--------+
                              |
                     +--------v--------+
                     |     Matrix      |
                     +-----------------+
```

### 3.1 Trading Platform
The Trading Platform is the complete, integrated software system. It unifies every engine into a single quantitative trading environment. The platform is responsible for the complete lifecycle of trading activities, from raw data acquisition to long-term performance analytics.

### 3.2 Engine
An Engine is the largest independent functional unit within the platform. Each engine owns one business domain and is completely responsible for solving one category of problems. Engines expose their capabilities through stable APIs and communicate through immutable matrix contracts.

### 3.3 Layer
A Layer is a sequential analytical stage within an engine. Layers divide a complex business process into smaller, manageable steps. A layer consumes structured input, performs one transformation, and produces one structured output (its Matrix). Layers are executed hierarchically, with each layer increasing the level of abstraction.

### 3.4 Matrix
A Matrix is the formal, immutable output produced by a layer. It represents the data contract between analytical stages. A Matrix defines required inputs, processing boundaries, produced outputs, and quality guarantees, describing mathematical knowledge rather than programmatic implementation.

### 3.5 Entity
An Entity represents the object being analyzed. Within the Trading Platform, the primary entity is a financial instrument (e.g., `BTCUSDT`, `ETHUSDT`, `EURUSD`, `AAPL`). An entity provides the baseline context for every analytical process.

### 3.6 Market Instance
A Market Instance represents one monitored entity at one specific timeframe.
$$\text{Market Instance} = \text{Symbol} \times \text{Timeframe}$$

Examples: `(BTCUSDT × 1 Minute)`, `(ETHUSDT × 1 Hour)`. Each Market Instance owns its own independent analytical pipeline, representing the smallest operational unit of the Market Monitoring Engine.

### 3.7 Timeframe
A Timeframe defines the temporal resolution used to analyze an entity. Timeframes are structured sequentially (e.g., micro, fast, slow, macro) to produce multi-timeframe intelligence.

### 3.8 Indicator
An Indicator is a continuous quantitative measurement derived from market data. Rather than returning a single numeric value, an indicator is represented as a structured telemetry object projected across multiple **Indicator Evaluation Axes** to provide immediate mathematical and behavioral context.

> **Target Architecture (Not Yet Implemented).** On the planned Data-Oriented hot path (DIE ingestion + MME Layers 1–5), indicators are computed with fast floating-point primitives (`f64`/`f32`) packed contiguously in memory (Structure of Arrays), enabling SIMD auto-vectorization, and are converted to exact decimals (`rust_decimal::Decimal`) only when crossing the transactional execution boundary (MME Layer 6 → TAE). *Current implementation:* most indicator calculators compute in `rust_decimal::Decimal` over `VecDeque` rolling windows (`crates/shared/src/indicators/*.rs`), with `f64` used at the normalization/synthesis stage.

### 3.9 Signal
A Signal is a discrete technical event detected from market telemetry. Rather than returning a binary state, a signal is represented as a structured telemetry object projected across multiple **Signal Evaluation Axes** to supply structural, risk-based, and transactional context.

> **Target Architecture (Not Yet Implemented).** Like indicators, signals on the target hot path are detected over contiguous `f64` primitive arrays and only lifted to `Decimal` at the execution boundary. *Current implementation:* signals are emitted as nested `IndicatorSignal` objects within the `Decimal`/`HashMap`-based `MarketSnapshot`.

### 3.10 Evaluation Axis
An Evaluation Axis is a standardized analytical dimension used to contextualize an indicator or signal. Projecting flat telemetry onto multiple axes transforms raw measurements into high-fidelity, multidimensional features, separating the raw value from its strength, direction, environmental context, and reliability.

#### 3.10.1  Evaluation Axes for Indicators
*   **Value:** The raw, quantitative numerical output of the mathematical formula (e.g., RSI = 68.4, ATR = 125).
*   **State:** The qualitative interpretation of the raw value (e.g., Bullish, Bearish, Neutral).
*   **Direction:** The immediate directional trajectory of the indicator's value vector (e.g., Rising, Falling, Flat).
*   **Strength:** The intensity or magnitude of the current reading relative to historical boundaries (e.g., Weak, Moderate, Strong, Extreme).
*   **Market Regime:** The environmental context under which the indicator is evaluated (e.g., Trending, Ranging, Expansion, Compression). Indicators are interpreted differently depending on this active state.
*   **Confidence:** The estimated mathematical reliability of the current reading, expressed as a probability percentile (0-100%).
*   **Freshness:** The temporal decay state of the evaluation, measuring how recently the current reading was established (e.g., New, Recent, Old, Expired).
*   **Quality:** An audit of the indicator's signal-to-noise ratio, assessing whether the telemetry is clean or structurally distorted (e.g., Healthy, Noisy, Weak, Exceptional).

#### 3.10.2 Evaluation Axes for Signals
*   **Signal Type:** The specific technical event or pattern classification detected (e.g., Bullish Divergence, EMA Crossover).
*   **Direction:** The directional bias implied by the triggered event (e.g., Bullish, Bearish).
*   **Strength:** The qualitative weight or intensity of the signal trigger (e.g., Weak, Medium, Strong).
*   **Confidence:** The historical or statistical probability score associated with the trigger's reliability (0-100%).
*   **Freshness:** The chronological distance (measured in elapsed intervals or candles) since the signal triggered (e.g., Just Triggered, 3 candles ago, Expired).
*   **Confirmation:** The validation state of the event, indicating whether supporting conditions have validated the trigger (e.g., Confirmed, Waiting, Rejected).
*   **Market Regime:** The macro regime context in which the signal occurred, determining its localized baseline reliability (e.g., `TRENDING_BULL`, `TRENDING_BEAR`, `RANGE`).
*   **Multi-Timeframe Agreement:** A boolean matrix mapping horizontal consensus across neighboring time horizons (micro, fast, slow, macro).
*   **Risk:** The localized threat classification associated with entering a trade on this specific trigger (e.g., Low, Medium, High).
*   **Priority:** The structural execution urgency assigned to the event (e.g., Critical, High, Medium, Low).

### 3.11 Local Confluence
Local Confluence measures the degree of mathematical agreement among indicators, signals, and analytical features within a single timeframe. It represents single-timeframe consensus.

### 3.12 Alignment
Alignment measures the degree of agreement regarding market direction or structure among multiple timeframes of the same entity. Alignment answers the question: *Do multiple timeframes describe the same market condition?*

### 3.13 Analysis
Analysis represents the structural and behavioral diagnosis of observed market behavior. It synthesizes alignment and timeframe metrics to determine the dominant directional bias—represented qualitatively as a categorical classification and quantitatively as a continuous **Market Bias Score** normalized between `-1.0` (absolute bearish) and `+1.0` (absolute bullish)—as well as the market regime, active cycle phase, and trend quality.

### 3.14 Opportunity
Opportunity represents the positive market potential identified within the current market conditions. It evaluates whether favorable trading setups exist (e.g., Trend Continuation, Breakout, Pullback) and scores them from 0 to 100, independent of actual execution parameters.

### 3.15 Risk
Risk represents the structural, technical, and environmental dangers present in the current market, independent of directional bias. It scores threats (volatility risk, liquidity risk, structural distance to invalidation) from 0 to 100, providing an objective metric for exposure limits.

### 3.16 Decision Support
Decision Support transforms market intelligence into actionable tactical guidance. It combines the Analysis Matrix (context), Opportunity Matrix (value), and Risk Matrix (vulnerability) to provide structured recommendations (Trade Readiness, dynamic stop-loss methods, and target take-profit environments) without making autonomous execution choices.

### 3.17 Market Overview
Market Overview represents the aggregated state of the entire monitored market universe. Rather than describing one asset, it summarizes cross-market dynamics, such as market breadth, risk distributions, and systemic risk indices.

### 3.18 Execution Policy
An Execution Policy is a user-defined, conditional rule managed by the Trade Automation Engine. Execution policies evaluate incoming decision-support parameters against programmatic constraints to determine whether to trigger an automated order (e.g., `IF Bias == Bullish AND Opportunity > 75 AND Risk < 30 AND Stance == Active THEN Trigger Long`).

### 3.19 Trade Execution
Trade Execution represents the physical interaction with an exchange interface or simulated execution environments. It manages order types, routes, transactional states, slippage controls, and exchange acknowledgments.

> **Target Architecture (Not Yet Implemented).** Trade Execution belongs to the OOP/Domain-Driven **cold path**: order routing, sizing, and transactional state are governed by 128-bit arbitrary-precision decimals (`rust_decimal::Decimal`) to prevent rounding errors and exchange rejections. *Current implementation:* the position-sizing / risk calculator (`crates/engine/src/risk_calculator.rs`) computes in `f64`; `Decimal` is used for stored order/position fields and quote data.

### 3.20 Position
A Position represents an active market exposure within the portfolio. It contains all real-time tracking parameters, such as entry price, current price, size, unrealized PnL, leverage, and active stop/target coordinates.

### 3.21 Portfolio
A Portfolio represents the complete collection of active positions, balances, margin requirements, and capital allocations. The portfolio provides the global financial and safety boundaries for the trading account.

> **Target Architecture (Not Yet Implemented).** The Portfolio ledger is strictly a cold-path domain: all balances, margin, capital allocations, and fees are maintained with `rust_decimal::Decimal` for penny-perfect cross-asset accounting. The base capital supplied to the sizing protocol is **available margin** (`available_margin`), not raw equity.

### 3.22 Performance
Performance represents the historical evaluation of trading results. It analyzes profitability, realized risk-to-reward ratios, risk-adjusted returns (Sharpe, Sortino), strategy expectancy, and strategy-to-regime compatibility vectors over time.

### 3.23 API Contract
An API Contract is the formalized, typed interface schema exposed by each engine's matrix. It guarantees that external systems can consume engine outputs reliably without dependencies on internal implementation details.

---

## Chapter 4 — Information Flow

The Trading Platform is fundamentally an information transformation system. Its purpose is to progressively convert raw physical observations into increasingly higher levels of intelligence until they become measurable business outcomes. Information always flows forward. Each stage consumes structured information, enriches it, and produces a new representation that is consumed by the next stage.

```
+--------------------+      +--------------------+      +--------------------+
| Data Infra. Engine | ---> | Market Mon. Engine | ---> | Trade Auto. Engine |
| (Acquires & Clean) |      | (Interprets State) |      | (Evaluates Policy) |
+--------------------+      +--------------------+      +--------------------+
                                                                  |
                                                                  v
+--------------------+      +--------------------+      +--------------------+
| Perf. Anal. Engine | <--- | Portfolio M. Engine| <--- |   Active Exchange  |
| (Evaluates History)|      | (Supervises State) |      |  (Fills & Orders)  |
+--------------------+      +--------------------+      +--------------------+
```

### 4.1 High-Level Information Flow
The complete Trading Platform transforms information through five major business domains in a controlled, unidirectional cascade:
$$\text{Exchange Data} \longrightarrow \text{Data Infrastructure} \longrightarrow \text{Market Monitoring} \longrightarrow \text{Trade Automation} \longrightarrow \text{Portfolio Management} \longrightarrow \text{Performance Analytics}$$

Each engine increases the abstraction and business value of the information it receives. Reverse dependencies are prohibited; for example, Portfolio Management must not calculate technical indicators, and Market Monitoring must not execute orders.

### 4.2 Information Transformation Sequence
The platform progressively abstracts raw data into business intelligence:
1.  **Raw Data:** Unstructured network events (WebSocket packets, REST responses).
2.  **Observations:** Structured, synchronized temporal OHLCV candles (Market Data).
3.  **Local Confluence:** Telemetry metrics, indicators, and localized signals (Metrics Matrix).
4.  **Relationships:** Multi-timeframe agreement and structural correlation (Alignment Matrix).
5.  **Interpretation:** Categorized market bias, regime diagnostics, and cycle phase (Analysis Matrix).
6.  **Value Assessment:** Quantified potential of structural setups (Opportunity Matrix).
7.  **Vulnerability Assessment:** Quantified environmental threat vectors (Risk Matrix).
8.  **Decision Support:** Actionable tactical guidance and stop/target zones (Decision Matrix).
9.  **Execution Decisions:** Triggered execution policies (Policy Matrix) and order fills (Execution Matrix).
10. **Portfolio State:** Active positions, aggregated exposures, and margin tracking (Portfolio Matrix).
11. **Performance Intelligence:** Strategy win rates, drawdowns, and strategy-regime maps (Performance Matrix).

### 4.3 Engine Communication Model
Communication between engines always occurs through stable API contracts exchanging read-only, immutable matrix payloads. Engines never access each other's internal variables, databases, or runtime states directly.

### 4.4 Data Infrastructure Engine (DIE) Flow
The DIE transforms raw exchange packets into standardized market data:
$$\text{Exchange APIs} \longrightarrow \text{Raw Data Layer} \longrightarrow \text{Market Data Layer} \longrightarrow \text{Data Quality Layer} \longrightarrow \text{Data Distribution Layer (Distribution Matrix)}$$

The DIE ensures data timeliness and validity before publishing the **Market Data Matrix** to the rest of the system.

### 4.5 Market Monitoring Engine (MME) Flow
The MME transforms standardized market data into multi-timeframe, explainable market intelligence:
$$\text{Market Data Matrix} \longrightarrow \text{Metrics Matrix} \longrightarrow \text{Alignment Matrix} \longrightarrow \text{Analysis Matrix} \longrightarrow \text{Opportunity Matrix} \longrightarrow \text{Risk Matrix} \longrightarrow \text{Decision Matrix} \longrightarrow \text{Overview Matrix}$$

This vertical pipeline processes the raw data step-by-step, producing symbol-specific tactical blueprints and global market breadth indicators.

### 4.6 Trade Automation Engine (TAE) Flow
The TAE transforms passive market intelligence into active exchange orders based on policy parameters:
$$\text{Decision Matrix} \longrightarrow \text{Policy Layer (Policy Matrix)} \longrightarrow \text{Execution Layer (Execution Matrix)} \longrightarrow \text{Exchange Orders}$$

The TAE never recalculates technical parameters or alters the MME's risk and opportunity ratings; it evaluates configured execution rules and manages order lifecycles.

### 4.7 Portfolio Management Engine (PME) Flow
The PME transforms transactional exchange events and fill confirmations into capital and exposure management states:
$$\text{Exchange Fills} \longrightarrow \text{Position Matrix} \longrightarrow \text{Exposure Matrix} \longrightarrow \text{Capital Matrix} \longrightarrow \text{Portfolio Matrix}$$

This pipeline manages position updates, portfolio exposure limits, and margin safety metrics.

### 4.8 Performance Analytics Engine (PAE) Flow
The PAE transforms historical transaction records and recorded market states into long-term system optimization intelligence:
$$\text{Portfolio History} \longrightarrow \text{Trade Analytics Matrix} \longrightarrow \text{Strategy Analytics Matrix} \longrightarrow \text{Risk Analytics Matrix} \longrightarrow \text{Performance Matrix}$$

The PAE correlates historical trade results with the market regimes diagnosed during those trades, providing a clean feedback loop for strategy refinement.

### 4.9 Information Granularity and Scope
As information moves through the platform, its **granularity** decreases (becoming more abstract and stable), while its **scope** expands:

| Platform Level | Granularity | Scope |
| :--- | :--- | :--- |
| **Data Infrastructure** | High-frequency, raw physical data | Individual network packets / ticker events |
| **Telemetry & Metrics** | Standardized temporal intervals | Single-timeframe, single-symbol parameters |
| **Alignment & Analysis** | Contextual technical states | Multi-timeframe, single-symbol state |
| **Opportunity & Risk** | Evaluative score vectors | Strategy-agnostic symbol potential and vulnerability |
| **Decision Support** | Actionable tactical guidance | Complete tactical blueprint for a single symbol |
| **Automation & Execution** | Conditional, policy-triggered actions | Specific orders and execution logs |
| **Portfolio Management** | High-frequency financial ledger | Combined positions, capital, and global account safety |
| **Performance Analytics**| Statistically stable, long-term ratios | Multi-regime, historical system-wide evaluation |

### 4.10 Direction of Dependencies
To maintain system stability, dependencies must always point forward (upstream to downstream). Downstream engines depend on the outputs of upstream engines. Bidirectional dependencies or backward loops are strictly prohibited:
*   The MME must never depend on active position sizes or execution states to calculate indicator alignment.
*   The TAE must never query the exchange directly to recalculate market regimes.
*   The PME must never compute indicators to manage active stops.

This strict separation of concerns protects the platform from cascading transaction errors and circular logic loops.

### 4.11 Information Integrity and Traceability
At no point in the pipeline should analytical transformations destroy the trace evidence of preceding stages. Every high-level decision or state change must remain fully traceable back to its root indicators and raw exchange inputs, ensuring that the system's actions can always be explained and audited.

### 4.12 Information Flow Design Rules
1.  **Rule 1:** Information must only move forward through the designated pipeline.
2.  **Rule 2:** Every engine owns one distinct business domain.
3.  **Rule 3:** Every layer owns one distinct analytical stage.
4.  **Rule 4:** Every layer must produce exactly one immutable Matrix as its output contract.
5.  **Rule 5:** Every Matrix represents a stable, published API contract.
6.  **Rule 6:** Higher-level intelligence must always be explainable by tracing back through the preceding matrices.
7.  **Rule 7:** No engine or layer may bypass preceding structural stages.
8.  **Rule 8:** No component may assume, recalculate, or modify responsibilities owned by another component.

---

## Chapter 5 — Trading Platform Ontology

The Trading Platform is conceptually organized as a decentralized ecosystem of five autonomous engines. Each engine owns a distinct business domain, manages its own internal lifecycle, and communicates exclusively through stable, contract-based APIs.

```
                  +-----------------------------------+
                  |         Trading Platform          |
                  +-----------------------------------+
                                    |
     +------------------+-----------+-----------+------------------+
     |                  |                       |                  |
+---------+        +---------+             +---------+        +---------+
|  Data   |        | Market  |             |  Trade  |        |Portfolio|
| Infra.  |------->| Monitor |------------>| Automat.|------->| Manage. |
| Engine  |        | Engine  |             | Engine  |        | Engine  |
+---------+        +---------+             +---------+        +---------+
     |                  |                       |                  |
     +------------------+-----------+-----------+------------------+
                                    |
                                    v
                           +-----------------+
                           |   Performance   |
                           |    Analytics    |
                           +-----------------+
```

### 5.1 Data Infrastructure Engine (DIE)
*   **Domain:** Data Ingest, Normalization, Quality Assurance, and High-Throughput Distribution.
*   **Primary Responsibility:** Transform raw, heterogeneous data streams from multiple external exchanges into standardized, reliable, and real-time market data.
*   **Core Question:** *What market data is available, and is it valid?*
*   **Input Boundary:** Raw WebSockets, REST API endpoints, and historical CSV/database repositories from external execution venues.
*   **Output Boundary:** The **Distribution Matrix** (real-time normalized trade/depth updates) and the **Market Data Matrix** (standardized OHLCV candle streams across monitored symbols and timeframes).

### 5.2 Market Monitoring Engine (MME)
*   **Domain:** Multi-Timeframe Technical Analysis, Trend and Structure Diagnostics, Opportunity Mapping, Environmental Risk Assessment, and Tactical Decision Support.
*   **Primary Responsibility:** Progressively transform standardized market data into multi-timeframe, explainable market intelligence on a per-symbol and global-market level.
*   **Core Question:** *What is happening in the market, what does it mean, and what are the associated opportunities and risks?*
*   **Input Boundary:** Consumes the **Market Data Matrix** produced by the Data Infrastructure Engine.
*   **Output Boundary:** The **Decision Matrix** (for individual symbols) and the **Overview Matrix** (for the global market state), exposed as the primary analytical inputs for trade automation or manual monitoring.

### 5.3 Trade Automation Engine (TAE)
*   **Domain:** Execution Policy Evaluation, Order Routing, Transaction Lifecycle Management, and Slippage Mitigation.
*   **Primary Responsibility:** Evaluate user-configured execution policies against real-time market intelligence and dispatch structured orders to target exchanges.
*   **Core Question:** *Do current market intelligence and portfolio conditions satisfy the configured parameters to execute a transactional action?*
*   **Input Boundary:** Consumes the **Decision Matrix** (MME) and the current **Portfolio Matrix** (PME).
*   **Output Boundary:** The **Execution Matrix**, containing active order status updates, transactional events, and execution feedback logs.

### 5.4 Portfolio Management Engine (PME)
*   **Domain:** Position Supervision, Leverage and Margin Management, Cross-Symbol Exposure Aggregation, and Account Balance Tracking.
*   **Primary Responsibility:** Maintain the real-time financial state of the trading account, supervising active market exposures and enforcing systemic capital safety rules.
*   **Core Question:** *What is the current financial and exposure state of the trading account, and are capital limits being respected?*
*   **Input Boundary:** Consumes exchange execution events and order confirmations from the Trade Automation Engine (TAE).
*   **Output Boundary:** The **Portfolio Matrix**, detailing active positions, gross/net exposures, margin metrics, and available capital reserves.

### 5.5 Performance Analytics Engine (PAE)
*   **Domain:** Historical Trade Reconstruction, Strategy Performance Diagnostics, Risk-Adjusted Return Attribution, and Behavioral Evaluation.
*   **Primary Responsibility:** Analyze historical trade logs and portfolio states to generate objective, mathematical insights regarding system performance and strategy efficacy.
*   **Core Question:** *How effectively did the trading platform perform historically, which strategies succeeded under which regimes, and what is the system's risk profile?*
*   **Input Boundary:** Consumes historical transaction archives from the Portfolio Management Engine (PME) and historical regime logs from the Market Monitoring Engine (MME).
*   **Output Boundary:** The **Performance Matrix**, containing standardized metric reporting (e.g., Sharpe, Sortino, Profit Factor, Drawdown durations, and strategy compatibility matrices).

---

## Chapter 6 — Market Monitoring Ontology

The Market Monitoring Engine is structured as a pipeline of 7 analytical layers. Layers 1, 2, and 3 are **strictly sequential**. After Layer 3 the pipeline **bifurcates (splits) in parallel**: Layer 4 (Opportunity) and Layer 5 (Risk) calculate independent, orthogonal dimensions of the market directly from the Analysis Matrix — evaluating risk must never depend on the opportunity score, and scoring an opportunity must never be limited by risk. These two independent states then **converge at Layer 6 (Decision Support)** for tactical synthesis, after which Layer 7 aggregates across symbols.

```
+------------------------------------------------------------------+
|                  Market Monitoring Engine (MME)                  |
+------------------------------------------------------------------+
|                                                                  |
|   [Layer 1] Metrics Layer (Metrics Matrix)                       |
|                    |                                             |
|                    v                                             |
|   [Layer 2] Alignment Layer (Alignment Matrix)                   |
|                    |                                             |
|                    v                                             |
|   [Layer 3] Analysis Layer (Analysis Matrix)                     |
|                    |                                             |
|          BIFURCATION (parallel, orthogonal)                      |
|             +------+------+                                      |
|             |             |                                      |
|             v             v                                      |
|   [Layer 4] Opportunity   [Layer 5] Risk Layer                   |
|   Layer (Opp. Matrix)     (Risk Matrix)                          |
|             |             |                                      |
|             +------+------+                                      |
|                    |  CONVERGENCE                                |
|                    v                                             |
|   [Layer 6] Decision Layer (Decision Matrix)                     |
|                    |                                             |
|                    v                                             |
|   [Layer 7] Overview Layer (Overview Matrix)                     |
|                                                                  |
+------------------------------------------------------------------+
```

### 6.1 Metrics Layer (Layer 1)
*   **Concept:** Multidimensional observation.
*   **Responsibility:** Compute mathematical indicators and extract technical signals, projecting each asset metrics stream onto its respective **Evaluation Axes** to generate structured `IndicatorEvaluation` and `SignalEvaluation` objects. It also compiles localized timeframe attributes and standardized analytical features.
*   **Output (Metrics Matrix):** Contains structured multidimensional indicator and signal evaluation objects, alongside localized timeframe attributes.
*   **Primary Structural Objects:**
    *   *IndicatorEvaluation:* Evaluates an indicator across the axes of **Value**, **State**, **Direction**, **Strength**, **Market Regime**, **Confidence**, **Freshness**, and **Quality**.
    *   *SignalEvaluation:* Evaluates a signal across the axes of **Signal Type**, **Direction**, **Strength**, **Confidence**, **Freshness**, **Confirmation**, **Market Regime**, **Multi-Timeframe Agreement**, **Risk**, and **Priority**.

### 6.2 Alignment Layer (Layer 2)
*   **Concept:** Horizontal correlation.
*   **Responsibility:** Analyze relationships and measure agreement among multiple timeframes for a single symbol.
*   **Output (Alignment Matrix):** Measures multi-timeframe directional and structural consensus.
*   **Key Metrics:**
    *   *Trend Alignment Score (0-100%):* Degree of agreement in trend direction across micro, fast, slow, and macro timeframes.
    *   *Momentum Alignment Score (0-100%):* Concordance of momentum vectors.
    *   *Confluence Matrix:* Grid mapping structural overlap and localized signal intersection points across timeframes.

### 6.3 Analysis Layer (Layer 3)
*   **Concept:** Market Interpretation.
*   **Responsibility:** Synthesize alignment and timeframe metrics into a cohesive technical diagnosis of the asset's overall state.
*   **Output (Analysis Matrix):** Establishes the dominant directional bias, the structural regime, the phase of the market cycle, and the reliability of the trend.
*   **Key Classifications:**
    *   *Market Bias:* `STRONG_BULLISH`, `BULLISH`, `NEUTRAL`, `BEARISH`, `STRONG_BEARISH`.
    *   *Market Regime:* `TRENDING_BULL`, `TRENDING_BEAR`, `RANGE_BOUND`, `ACCUMULATION`, `DISTRIBUTION`, `VOLATILITY_EXPANSION`, `VOLATILITY_CONTRACTION`, `TRANSITION`.
    *   *Market Phase:* `ACCUMULATION`, `MARKUP`, `DISTRIBUTION`, `MARKDOWN`.
    *   *Market Quality (0-100):* Measure of trend clarity and structural predictability.

### 6.4 Opportunity Layer (Layer 4)
*   **Concept:** Favorable environment tracking.
*   **Responsibility:** Evaluate the structural and momentum conditions to determine if they support high-probability trading configurations. This layer is strictly non-execution-based; it identifies potential, not action. It reads directly from the **Analysis Matrix (Layer 3)**, running in parallel with the Risk Layer (Layer 5).
*   **Output (Opportunity Matrix):** Identifies and scores specific tactical opportunity vectors. Fed directly to Layer 6 (Decision Support).

### 6.5 Risk Layer (Layer 5)
*   **Concept:** Environmental danger tracking.
*   **Responsibility:** Quantify structural, technical, and execution risks present in the market environment, independent of directional bias. It reads directly from the **Analysis Matrix (Layer 3)**, executing independently from, and in parallel with, the Opportunity Layer (Layer 4).
*   **Output (Risk Matrix):** Compiles and scores multi-dimensional risk factors on a unipolar scale. Fed directly to Layer 6 (Decision Support).

### 6.6 Decision Layer (Layer 6)
*   **Concept:** Decision support and actionable guidance.
*   **Responsibility:** The **convergence (synthesis) boundary** where the two parallel branches merge. Synthesize the Analysis Matrix (Regime/Bias), the Opportunity Matrix (Layer 4), and the Risk Matrix (Layer 5) into structured, strategy-agnostic Decision Matrix profiles. It determines market compatibility and protection parameters.
*   **Output (Decision Matrix):** Unified tactical blueprint for a single symbol.
*   **Key Elements:**
    *   *Trade Readiness:* `READY`, `FORMING`, `WATCH`, `STAND_ASIDE` (canonical vocabulary; see [Decision Matrix §4](../matrices/02-04-decision-matrix.md)).
    *   *Strategy Compatibility Matrix:* Score mapping current conditions to strategies (e.g., Trend Following: `HIGH`, Mean Reversion: `LOW`).
    *   *Stop Loss/Protection Environment:* Recommended invalidation methodology (`STRUCTURE_BASED`, `VOLATILITY_BASED`, `ATR_BASED`).
    *   *Take Profit Environment:* Target-rich environments (`RESISTANCE_BASED`, `RR_BASED`, `VOLATILITY_BASED`).
    *   *Scenario Analysis:*
        *   `Primary Scenario:` Most probable path (e.g., `BULLISH_CONTINUATION`).
        *   `Alternative Scenario:` Most probable failure path (e.g., `BREAKDOWN_AND_LIQUIDITY_SWEEP`).
        *   `Invalidation Trigger:` Concrete market parameters that nullify the primary scenario.

### 6.7 Overview Layer (Layer 7)
*   **Concept:** Global market breadth.
*   **Responsibility:** Aggregate individual asset Decision Matrices into a unified state representation of the entire monitored market universe.
*   **Output (Overview Matrix):** Summarizes market-wide dynamics for portfolio allocation and systemic risk control.
*   **Key Elements:**
    *   *Market Breadth Indices:* Percentage of symbols exhibiting Bullish vs. Bearish biases.
    *   *Regime Distribution:* Mapping of how many symbols are in expansion vs. contraction regimes.
    *   *Asset Rankings:* Leaderboard of strongest and weakest symbols based on Quality and Opportunity scores.
    *   *Systemic Risk Score (0-100):* Cross-market aggregated risk metric representing market-wide danger levels.

---

## Chapter 7 — Trade Automation Ontology

The Trade Automation Engine bridges the gap between passive intelligence (MME) and execution. It evaluates user-defined risk parameters and programmatic constraints to execute transaction rules.

### 7.1 Policy Layer (Layer 1)
*   **Concept:** Conditional validation rules.
*   **Responsibility:** Evaluate active Execution Policies against incoming symbol-specific Decision Matrices and active global Overview Matrices.
*   **Output (Policy Matrix):** List of triggered policies, containing exact entry/exit/management directives.
*   **Key Components:**
    *   *Execution Policy:* A deterministic programmatic rule.
        *   *Example Structure:* `IF (Decision.Bias == "BULLISH") AND (Decision.OpportunityScore > 75) AND (Decision.RiskScore < 40) THEN Trigger LONG`.
    *   *Stance Definition:* Automated execution states per symbol:
        *   `Active:` Full automated trading permitted.
        *   `Close Only:` Only exit or protection-tightening operations allowed.
        *   `Avoid:` Strictly block all execution triggers.

### 7.2 Execution Layer (Layer 2)
*   **Concept:** Transaction implementation.
*   **Responsibility:** Translate approved policy actions into physical order messages, select routing methods, and manage order execution state. It executes the **Position Sizing Protocol**, dynamically calculating entry position size ($S$) using the formula:
$$S = \frac{E \times R}{D_{sl} / 100}$$
	where $E$ represents available margin (from PME Capital Matrix), $R$ represents the risk-per-trade decimal fraction (`risk_per_trade_pct / 100`, e.g. $0.01$ = 1%), and $D_{sl}$ represents the stop-loss distance as a raw percentage float (from MME Decision Matrix, e.g. `1.5` = 1.5%, divided by 100 in the formula).
*   **Output (Execution Matrix):** Structured database of outstanding, filled, modified, and cancelled orders.
*   **Key Components:**
    *   *Order Routing Strategy:* Logic determining whether to deploy `Limit Orders`, `Market Orders`, `Stop Orders`, or `TWAP/VWAP Execution Schemes`.
    *   *Slippage Control:* Algorithms adjusting limit offsets based on immediate order book depth.
    *   *Transaction State:* Lifecycles tracking orders from `Pending Ingest` -> `Sent` -> `Acknowledged` -> `Partially Filled` -> `Filled` / `Rejected` / `Expired`.

---

## Chapter 8 — Portfolio Management Ontology

The Portfolio Management Engine manages capital safety. It supervises active risk exposures, maintains account balances, and monitors margin usage in real-time.

### 8.1 Position Layer (Layer 1)
*   **Concept:** Active exposure tracking.
*   **Responsibility:** Monitor individual, active asset positions, updating market-driven valuation metrics.
*   **Output (Position Matrix):** Directory of active positions with live metrics.
*   **Key Fields:**
    *   `Asset Pair:` (e.g., BTCUSDT)
    *   `Direction:` `Long` or `Short`
    *   `Entry Price:` Volume-Weighted Average Entry Price.
    *   `Position Size:` Base asset size.
    *   `Current Price:` Immediate index or mark price.
    *   `Unrealized PnL / ROI:` Live dollar and percentage return.
    *   `Stop Loss Trigger:` Operational coordinate of active protection.
    *   `Take Profit Coordinates:` Programmatic target execution points.

### 8.2 Exposure Layer (Layer 2)
*   **Concept:** Risk aggregation.
*   **Responsibility:** Group positions across correlated sectors, asset types, or asset baskets to prevent overexposure to specific market vectors.
*   **Output (Exposure Matrix):** Risk distribution maps across predefined boundaries.
*   **Key Metrics:**
    *   *Gross Portfolio Exposure:* Total combined nominal value of all active positions.
    *   *Net Portfolio Exposure:* Combined directional bias (Long nominal value minus Short nominal value).
    *   *Sector/Asset Concentration:* Percentage of total capital allocated to a single asset or closely correlated group of assets.

### 8.3 Capital Layer (Layer 3)
*   **Concept:** Account solvency monitoring.
*   **Responsibility:** Track available margin, leverage ratios, equity curves, and funding impacts.
*   **Output (Capital Matrix):** High-frequency balance sheet of the trading account.
*   **Key Fields:**
    *   `Initial Balance:` Starting capital.
    *   `Current Equity:` Balance + Unrealized PnL.
    *   `Available Margin:` Liquid capital available for new position initiation.
    *   `Margin Usage Ratio:` Percentage of total equity committed to maintenance/initial margin requirements.
    *   `Effective Leverage:` Ratio of gross exposure to total account equity.

### 8.4 Portfolio Layer (Layer 4)
*   **Concept:** Unified portfolio state.
*   **Responsibility:** Synthesize Position, Exposure, and Capital matrices into a single, high-level status vector representing the absolute financial health of the trading system. It possesses **Ontological Priority (Veto Power)**; if safety thresholds or drawdown limits are breached, it overrides active TAE stances, setting them to `Avoid` or `Close Only` to automatically block execution at the transaction boundary.
*   **Output (Portfolio Matrix):** Unified ledger used for high-level automated safety checks (e.g., portfolio-wide drawdown stops, maximum margin warnings).

***

## Chapter 9 — Performance Analytics Ontology

The Performance Analytics Engine operates on historical transaction logs, portfolio equity metrics, and recorded market regimes. Its role is to reconstruct and analyze completed trading activities, transforming raw trade histories into structured business intelligence.

```
+--------------------------------------------------------+
|          Performance Analytics Engine (PAE)            |
+--------------------------------------------------------+
|                                                        |
|   [Layer 1] Trade Analytics Layer                      |
|             (Trade Analytics Matrix)                   |
|                    |                                   |
|                    v                                   |
|   [Layer 2] Strategy Analytics Layer                   |
|             (Strategy Analytics Matrix)                |
|                    |                                   |
|                    v                                   |
|   [Layer 3] Risk Analytics Layer                       |
|             (Risk Analytics Matrix)                    |
|                    |                                   |
|                    v                                   |
|   [Layer 4] Performance Layer                          |
|             (Performance Matrix)                       |
|                                                        |
+--------------------------------------------------------+
```

### 9.1 Trade Analytics Layer (Layer 1)
*   **Concept:** Single-trade reconstruction.
*   **Responsibility:** Parse raw execution logs to reconstruct individual, completed trades from initial entry order to final exit order, capturing execution efficiency metrics.
*   **Output (Trade Analytics Matrix):** A comprehensive ledger of closed trades, with detailed execution and timing attributes.
*   **Key Fields:**
    *   `Trade ID:` Unique system-wide transaction identifier.
    *   `Symbol:` The financial instrument traded.
    *   `Direction:` `Long` or `Short`.
    *   `Hold Time:` Exact duration between first entry fill and final exit fill.
    *   `Gross PnL:` Closed financial result before fees.
    *   `Net PnL:` Realized profit or loss after trading fees, funding costs, and slippage.
    *   `Execution Slippage:` The mathematical difference between target policy price and actual exchange fill price.
    *   `MFE (Maximum Favorable Excursion):` The peak unrealized profit reached during the trade's lifespan.
    *   `MAE (Maximum Adverse Excursion):` The peak unrealized loss reached during the trade's lifespan.

### 9.2 Strategy Analytics Layer (Layer 2)

- **Concept:** Strategy-level aggregation.
- **Responsibility:** Group completed trades by their originating Execution Policy to analyze the performance and statistical significance of individual trading methodologies against random noise.
- **Output (Strategy Analytics Matrix):** Performance metrics and statistical significance parameters segmented by strategy type or specific execution rule.
- **Key Metrics:**
    - Win Rate: Percentage of profitable trades relative to total trades executed by the strategy.
    - Profit Factor: Gross profits divided by gross losses.
    - Average Win/Loss Ratio: Mean net profit of winning trades divided by the mean net loss of losing trades.
    - Expectancy: The expected net return per trade executed under the strategy.
    - Slippage Overhead: Combined drag of slippage and exchange fees on the strategy's net return.
    - T-Statistic: Measures how many standard errors the strategy's average return deviates from zero to validate edge against the null hypothesis (`H0H0​`).
    - P-Value: The probability of obtaining the strategy's observed performance metrics if the returns were generated by random chance.
    - Monte Carlo Empirical Probability: The fraction of sign-randomized baseline samples (each trade's PnL multiplied by a fair-coin ±1) whose performance metric equals or exceeds the strategy's actual performance.

### 9.3 Risk Analytics Layer (Layer 3)
*   **Concept:** Drawdown and capital safety analytics.
*   **Responsibility:** Analyze historical portfolio equity curves to quantify systemic risk, capital degradation speed, and portfolio volatility.
*   **Output (Risk Analytics Matrix):** Volatility and drawdown metrics representing the safety profile of the capital.
*   **Key Metrics:**
    *   *Maximum Drawdown (Peak-to-Trough):* The largest historical percentage drop in equity.
    *   *Drawdown Duration:* The length of time spent in a drawdown state before reclaiming previous equity peaks.
    *   *Sharpe Ratio:* Risk-adjusted return based on the standard deviation of daily portfolio returns.
    *   *Sortino Ratio:* Risk-adjusted return focusing exclusively on downside volatility.
    *   *Ulcer Index:* Measure of the depth and duration of drawdowns.

### 9.4 Performance Layer (Layer 4)
*   **Concept:** Unified performance intelligence.
*   **Responsibility:** Synthesize trade, strategy, and risk matrices to map strategy performance against historical market regimes. It identifies structural compatibility, helping developers and optimization systems determine which configurations excel in specific environments.
*   **Output (Performance Matrix):** The definitive performance profile of the trading platform.
*   **Key Components:**
    *   *Regime Compatibility Matrix:* Grid mapping execution policies to the market regimes (from the Analysis Matrix) in which they were active, isolating where alpha was generated versus where capital was degraded.
    *   *System Optimization Guidance:* Metric-driven feedback indicating necessary threshold adjustments for active execution policies.

---

## Chapter 10 — Trading Lifecycle Ontology

The Trading Platform views a trade not as a single action, but as a sequential process transitioning through seven distinct operational phases.

```
  +-------------------------------------------------------------+
  |                     The Trading Lifecycle                   |
  +-------------------------------------------------------------+
  |                                                             |
  |  [Phase 1] Inception / Discovery (MME Analysis)             |
  |                      |                                      |
  |                      v                                      |
  |  [Phase 2] Evaluation / Tactical Support (MME Decision)      |
  |                      |                                      |
  |                      v                                      |
  |  [Phase 3] Trigger Validation (TAE Policy)                  |
  |                      |                                      |
  |                      v                                      |
  |  [Phase 4] Execution Routing (TAE Order dispatch)           |
  |                      |                                      |
  |                      v                                      |
  |  [Phase 5] Supervision / Active Management (PME Monitoring) |
  |                      |                                      |
  |                      v                                      |
  |  [Phase 6] Liquidation / Exit (TAE -> PME Close)            |
  |                      |                                      |
  |                      v                                      |
  |  [Phase 7] Reconstruction / Analytics (PAE Ingest)          |
  |                                                             |
  +-------------------------------------------------------------+
```

### 10.1 Phase 1: Inception / Discovery
*   **Description:** The market is continuously monitored. Technical indicators, signals, and multi-timeframe alignments are computed for all active symbols. 
*   **Milestone:** The Market Monitoring Engine generates a fresh **Analysis Matrix**, establishing a clear directional bias and identifying an active regime (e.g., `TRENDING_BULL` during a `MARKUP` cycle phase).

### 10.2 Phase 2: Evaluation / Tactical Support
*   **Description:** The technical setup is evaluated to determine positive potential and environmental danger.
*   **Milestone:** The **Opportunity Matrix** and **Risk Matrix** are generated. These are processed by the Decision Layer to produce a **Decision Matrix**, declaring the asset's *Trade Readiness* as `READY` or `FORMING` and establishing concrete structural invalidation zones.

### 10.3 Phase 3: Trigger Validation
*   **Description:** The Trade Automation Engine's Policy Layer processes the **Decision Matrix**. It checks whether the symbol's readiness, opportunity scores, and risk classifications satisfy any active, user-configured execution policies.
*   **Milestone:** A programmatic policy condition is fully satisfied, generating a trigger command in the **Policy Matrix**.

### 10.4 Phase 4: Execution Routing
*   **Description:** The Policy trigger is translated into actionable exchange commands. Capital parameters (from the PME Capital Matrix) are referenced to calculate exact position sizing based on risk-per-trade guidelines.
*   **Milestone:** Order messages are dispatched to the target exchange, and the **Execution Matrix** tracks the order states until they are confirmed as `Filled`.

### 10.5 Phase 5: Supervision / Active Management
*   **Description:** The filled order becomes an active position. The Portfolio Management Engine takes ownership of tracking live valuation changes, margin commitments, and total portfolio exposure.
*   **Milestone:** The active position is continuously updated in the **Position Matrix**. Stop loss levels and target take profit zones are managed dynamically based on ongoing Market Monitoring updates.

### 10.6 Phase 6: Liquidation / Exit
*   **Description:** The exit criteria are met. This can occur via a hit stop loss, reached profit target, a structural invalidation signal from the Market Monitoring Engine, or a portfolio-wide drawdown emergency trigger.
*   **Milestone:** Exit orders are executed, the market exposure is cleared, and the final transactional data is written to the database. The position status is updated to `Closed` in the **Position Matrix**.

### 10.7 Phase 7: Reconstruction / Analytics
*   **Description:** The completed transaction is handed over to the Performance Analytics Engine. 
*   **Milestone:** The execution fills, entry/exit coordinates, and environmental market states are analyzed to update the **Trade Analytics Matrix** and refine the **Regime Compatibility Matrix**.

---

## Chapter 11 — Relationship Between Engines and Lifecycle

Engines do not own the trading lifecycle individually; instead, they collaborate sequentially, passing structured matrices to coordinate transition states. The matrix below defines which engine leads, supports, or remains inactive during each phase of a trade's lifecycle.

| Lifecycle Phase | Data Infrastructure Engine (DIE) | Market Monitoring Engine (MME) | Trade Automation Engine (TAE) | Portfolio Management Engine (PME) | Performance Analytics Engine (PAE) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **1. Inception** | **Lead** (Provides raw and normalized candle streams) | **Lead** (Computes features, alignment, and regime) | Inactive | Inactive | Inactive |
| **2. Evaluation** | Support (Provides real-time book depth / spread) | **Lead** (Quantifies Opportunity, Risk, and Decision support) | Inactive | Inactive | Inactive |
| **3. Trigger** | Inactive | Support (Updates Decision Matrix parameters) | **Lead** (Validates active programmatic policy rules) | Support (Provides capital balance metrics for sizing) | Inactive |
| **4. Routing** | Inactive | Inactive | **Lead** (Determines order routes and manages fills) | Support (Blocks execution if margin limits are breached) | Inactive |
| **5. Supervision**| Support (Updates current market mark prices) | Support (Identifies dynamic structure changes) | Support (Modifies orders if dynamic exit rules trigger) | **Lead** (Tracks active position metrics and net exposure) | Inactive |
| **6. Liquidation**| Inactive | Support (Signals structural invalidation if reached) | **Lead** (Executes liquidation and exit order sequences) | Support (Reclaims margin and updates balance ledgers) | Inactive |
| **7. Analytics** | Inactive | Inactive | Inactive | Support (Exposes historical trade logs and equity values) | **Lead** (Reconstructs trades, groups metrics, maps regimes) |

---

## Chapter 12 — Information Hierarchy

Information in the Trading Platform is organized hierarchically. Data begins at a high-frequency, granular level and is progressively aggregated, filtered, and contextualized into increasingly stable and abstract representations.

```
       +---------------------------------------------+
       |             Information Hierarchy           |
       +---------------------------------------------+
       |                                             |
       |  [Level 5] Historical / Performance Level   |
       |            (Regime Maps, Ratios)            |
       |                      ^                      |
       |                      |                      |
       |  [Level 4] Global / Systemic Level          |
       |            (Overview Matrix, Portfolio)     |
       |                      ^                      |
       |                      |                      |
       |  [Level 3] Asset / Symbol Level             |
       |            (Analysis, Opp, Risk, Decision)  |
       |                      ^                      |
       |                      |                      |
       |  [Level 2] Timeframe / Contextual Level     |
       |            (Metrics, Alignment Matrices)    |
       |                      ^                      |
       |                      |                      |
       |  [Level 1] Granular / Transactional Level   |
       |            (Raw Ticks, Order Book Depth)    |
       |                                             |
       +---------------------------------------------+
```

### 12.1 Level 1: Granular / Transactional Level
*   **Data Types:** Raw execution fills, individual order book updates, WebSocket price ticks, network latency measurements.
*   **Characteristics:** High-frequency, volatile, high-volume, lacking analytical context.
*   **Primary Owner:** Data Infrastructure Engine (DIE).

### 12.2 Level 2: Timeframe / Contextual Level
*   **Data Types:** Standardized candles, mathematical indicators (RSI, ATR), directional signals, multi-timeframe consensus alignments.
*   **Characteristics:** Structured temporal intervals, standardized representations, local confluence metrics.
*   **Primary Owner:** Market Monitoring Engine (MME) - Layers 1 and 2.

### 12.3 Level 3: Asset / Symbol Level
*   **Data Types:** Structural regimes, directional biases, opportunity classifications, risk profiles, tactical decision guides.
*   **Characteristics:** Unified, multi-timeframe analytical profiles of a single financial instrument.
*   **Primary Owner:** Market Monitoring Engine (MME) - Layers 3, 4, 5, and 6.

### 12.4 Level 4: Global / Systemic Level
*   **Data Types:** Market breadth, systemic risk index, active portfolio positions, total net currency exposures, margin limit warnings, automated policy triggers.
*   **Characteristics:** Broad, cross-symbol aggregations and complete account state definitions.
*   **Primary Owner:** Market Monitoring Engine (MME Layer 7), Trade Automation Engine (TAE), and Portfolio Management Engine (PME).

### 12.5 Level 5: Historical / Performance Level
*   **Data Types:** Strategy win rates, maximum drawdown metrics, Sharpe/Sortino ratios, strategy-regime compatibility maps, slippage overhead profiles.
*   **Characteristics:** Highly abstract, statistically stable, long-term evaluative intelligence.
*   **Primary Owner:** Performance Analytics Engine (PAE).

***

## Chapter 13 — Levels of Abstraction

Abstraction is the primary mechanism by which raw physical market states are simplified into operational meaning. By defining strict boundaries between levels of abstraction, the system prevents technical implementation details from leaking into higher-level business decisions.

```
+-----------------------------------------------------------------------------+
|                               Levels of Abstraction                          |
+-----------------------------------------------------------------------------+
|                                                                             |
| [Level 8: Performance Meta-Intelligence] -> Strategy/Regime mapping         |
| [Level 7: Systemic Context]               -> Market Breadth, Portfolio State |
| [Level 6: Tactical Guidance]              -> Stop/Target areas, Readiness    |
| [Level 5: Opportunity & Risk Evaluation]  -> Opp. & Risk Scoring Matrices   |
| [Level 4: Market Interpretation]          -> Trend Biases and Regimes        |
| [Level 3: Inter-Temporal Correlation]     -> Multi-Timeframe Alignment       |
| [Level 2: Structured Observations]        -> Indicator metrics, Candlesticks |
| [Level 1: Granular Physical State]        -> Raw network packets, WS frames  |
|                                                                             |
+-----------------------------------------------------------------------------+
```

### 13.1 Level 1: Granular Physical State
*   **Definition:** Raw data streams directly from execution venues.
*   **Abstraction Goal:** Standardize the medium of transport.
*   **Operations:** Packet parsing, WebSocket connection maintenance, rate limit compliance, feed reconnection, and byte decoding.
*   **Cognitive Load:** High volume, low business value.

### 13.2 Level 2: Structured Observations
*   **Definition:** Uniform temporal slices and derived localized measurements.
*   **Abstraction Goal:** Standardize the unit of time and price.
*   **Operations:** OHLCV candle construction, calculation of basic technical indicators, signal extraction (e.g., crossovers), and localized confluence scoring.
*   **Cognitive Load:** Moderately high volume, basic descriptive value.

### 13.3 Level 3: Inter-Temporal Correlation
*   **Definition:** Multi-timeframe trend, momentum, and structural relationships.
*   **Abstraction Goal:** Identify spatial-temporal agreement.
*   **Operations:** Mathematical comparisons of vectors across different timeframes to produce alignment indices.
*   **Cognitive Load:** Medium volume, high structural correlation value.

### 13.4 Level 4: Market Interpretation
*   **Definition:** The qualitative categorization of market state.
*   **Abstraction Goal:** Determine the structural context of the asset.
*   **Operations:** Categorizing the dominant trend bias, classifying the current market regime, identifying the active cycle phase, and calculating overall trend quality.
*   **Cognitive Load:** Low volume, high explanatory value.

### 13.5 Level 5: Opportunity & Risk Evaluation
*   **Definition:** Independent measurements of positive trade potential and negative environmental uncertainty.
*   **Abstraction Goal:** Isolate value from vulnerability.
*   **Operations:** Calculating independent scoring vectors for various strategy-agnostic opportunities (breakout, pullback, trend following) and risk dimensions (volatility, liquidity, structural proximity).
*   **Cognitive Load:** Low volume, high evaluative value.

### 13.6 Level 6: Tactical Guidance
*   **Definition:** Strategy-agnostic decision-support blueprints.
*   **Abstraction Goal:** Define parameters of engagement without enforcing execution.
*   **Operations:** Establishing overall trade readiness, assessing strategic compatibility, formulating dynamic invalidation parameters, and designing support/resistance target ranges.
*   **Cognitive Load:** Minimal volume, high actionable value.

### 13.7 Level 7: Systemic Context
*   **Definition:** Global market breadth and active portfolio status.
*   **Abstraction Goal:** Align individual symbol opportunities with global system capacity.
*   **Operations:** Computing market-wide breadth indices, tracking total net exposure, evaluating margin levels, and monitoring portfolio-wide risk metrics.
*   **Cognitive Load:** Minimal volume, high coordination value.

### 13.8 Level 8: Performance Meta-Intelligence
*   **Definition:** Historical evaluation of strategy efficacy across market regimes.
*   **Abstraction Goal:** Refine operational assumptions over time.
*   **Operations:** Historical trade reconstruction, calculating risk-adjusted metrics, mapping returns to MME regimes, and identifying structural slippage drag.
*   **Cognitive Load:** Minimal volume, high optimizing value.

---

## Chapter 14 — Separation of Responsibilities

To maintain modularity and prevent coupling, the platform enforces strict boundaries of responsibility between its core engines. No engine may assume, calculate, or manipulate data belonging to another engine's business domain.

```
                    +-----------------------------+
                    |    Architectural Boundaries |
                    +-----------------------------+
                                   |
         +-------------------------+-------------------------+
         |                                                   |
+------------------------------+            +------------------------------+
| Market Monitoring Engine     |            | Trade Automation Engine      |
|                              |            |                              |
| - Observes technical metrics |            | - Evaluates policy rules     |
| - Identifies market regimes  |            | - Determines active stances  |
| - Calculates risk scores     |            | - Manages order lifecycles   |
| - Suggests dynamic targets   |            | - Controls routing logic     |
|                              |            |                              |
| NO order execution knowledge |            | NO indicator computation     |
| NO portfolio balance access  |            | NO raw price determination   |
+------------------------------+            +------------------------------+
                                   |
         +-------------------------+-------------------------+
         |                                                   |
+------------------------------+            +------------------------------+
| Portfolio Management Engine  |            | Performance Analytics Engine |
|                              |            |                              |
| - Monitors balance & margin  |            | - Reconstructs closed trades |
| - Controls asset exposure    |            | - Evaluates risk metrics     |
| - Tracks active positions    |            | - Compares regimes to alpha  |
| - Enforces drawdown stops    |            | - Suggests parameter tuning  |
|                              |            |                              |
| NO execution mechanics       |            | NO active trade management   |
| NO indicator recalculations  |            | NO live market data access   |
+------------------------------+            +------------------------------+
```

### 14.1 Market Monitoring Engine (MME) Boundaries
*   **Permitted Actions:**
    *   Analyze normalized OHLCV streams.
    *   Compute indicators, signals, alignments, biases, and regimes.
    *   Establish opportunity and risk scores.
    *   Formulate decision-support parameters and target areas.
    *   Aggregate individual metrics to evaluate global market breadth.
*   **Prohibited Actions:**
    *   Access API keys, trade credentials, or order execution interfaces.
    *   Read account balances, margin details, or active position sizes.
    *   Calculate or apply currency-specific position-sizing logic.
    *   Retain state concerning active or historical orders.

### 14.2 Trade Automation Engine (TAE) Boundaries
*   **Permitted Actions:**
    *   Receive decision-support matrices.
    *   Evaluate user-defined, conditional execution policies.
    *   Query PME for available margin to calculate trade size.
    *   Build, dispatch, and track active orders on external exchanges.
    *   Manage order status, transaction timing, and routing strategies.
*   **Prohibited Actions:**
    *   Perform raw mathematical indicator calculations.
    *   Deduce trend bias or market regimes from candles.
    *   Alter, overwrite, or filter the raw risk or opportunity scores sent by the MME.
    *   Enforce portfolio-wide drawdown or risk limits directly.

### 14.3 Portfolio Management Engine (PME) Boundaries
*   **Permitted Actions:**
    *   Maintain the primary ledger of active positions, capital reserves, and margin usage.
    *   Enforce aggregate exposure boundaries across sectors and assets.
    *   Enforce account-level safety features (e.g., total drawdown stop-outs).
    *   Update active position exit parameters (stops, take-profits) based on received market-structure updates.
*   **Prohibited Actions:**
    *   Decide whether to execute a new trade based on market conditions.
    *   Interact with exchange execution routing mechanics directly.
    *   Recalculate indicators or trend alignments.
    *   Assess strategy compatibility.

### 14.4 Performance Analytics Engine (PAE) Boundaries
*   **Permitted Actions:**
    *   Retrieve completed transaction records from historical databases.
    *   Reconstruct discrete, closed trades.
    *   Calculate mathematical metrics evaluating win rates, drawdowns, and ratios (Sharpe, Sortino).
    *   Correlate historical strategy execution logs with recorded MME regimes.
*   **Prohibited Actions:**
    *   Interact with any active execution pipeline or real-time order.
    *   Monitor live positions or manage current capital limits.
    *   Query exchange feeds or parse real-time market data directly.

---

## Chapter 15 — Communication Model

The platform uses a contract-based, decoupled communication architecture. Engines operate as isolated computational processes, exchanging immutable snapshots of their state via standardized message interfaces.

### 15.1 Flow Directionality
*   **Unidirectional Information Flow:** Information always cascades forward from raw observations to meta-intelligence.
    $$\text{Data Infrastructure} \longrightarrow \text{Market Monitoring} \longrightarrow \text{Trade Automation} \longrightarrow \text{Portfolio Management} \longrightarrow \text{Performance Analytics}$$
*   **Feedback Loops:** Back-propagation of information is highly restricted and occurs only through specific state contracts:
    *   *Sizing Feedback:* PME provides Capital Matrix to TAE to guide size calculation during execution.
    *   *Analytical Feedback:* PAE provides historical performance analysis to configuration databases for off-line policy optimization.

### 15.2 Communication Paradigms
1.  **Publish / Subscribe (Pub/Sub):**
    *   *Usage:* Real-time, continuous data dissemination.
    *   *Examples:* Normalized exchange ticks (Distribution Matrix), standardized candle updates (Market Data Matrix), and real-time analytical updates (Decision Matrix).
    *   *Guarantee:* High-throughput, low-latency, non-blocking delivery.
2.  **Request / Response:**
    *   *Usage:* On-demand state queries or transaction execution commands.
    *   *Examples:* Querying active margin limits from PME, sending an execution order to TAE, or requesting historical performance metrics from PAE.
    *   *Guarantee:* Synchronous or asynchronous state confirmation with explicit error handling.

### 15.3 State Preservation and Serialization
*   **Immutability:** Every matrix generated by an analytical layer is treated as an immutable snapshot. Once published, a matrix cannot be altered. Changes over time are represented by a chronological sequence of versioned matrices.
*   **Payload Structuring:** Communication payloads must adhere to strict schemas (JSON Schema) to enforce boundaries and prevent serialization drift. No language-specific object serialization is permitted between engines.

---

## Chapter 16 — Complete Conceptual Model

The conceptual model integrates all five engines, their respective layers, and the resulting matrices into a single, cohesive quantitative lifecycle.

```
[Exchange WS/REST]
        |
        v
+-----------------------------------------------------------------------------------------+
| DATA INFRASTRUCTURE ENGINE (DIE)                                                        |
|   - Ingests ticks -> standardizes frames -> distributes streams via Distribution Matrix |
+-----------------------------------------------------------------------------------------+
        | (Market Data Matrix)
        v
+-----------------------------------------------------------------------------------------+
| MARKET MONITORING ENGINE (MME)                                                          |
|   1. Metrics Layer     -> Computes indicators & signals per interval -> Metrics Matrix  |
|   2. Alignment Layer   -> Correlates trends/momentum across times    -> Alignment Matrix|
|   3. Analysis Layer    -> Classifies bias, regimes, and phases       -> Analysis Matrix |
|   4. Opportunity Layer -> Isolates positive trade potential          -> Opportunity M.  |
|   5. Risk Layer        -> Scores structural and environmental danger -> Risk Matrix     |
|   6. Decision Layer    -> Maps strategy compatibility & stop/targets -> Decision Matrix |
|   7. Overview Layer    -> Aggregates global breadth & systemic risk  -> Overview Matrix |
+-----------------------------------------------------------------------------------------+
        | (Decision Matrix & Overview Matrix)
        v
+-----------------------------------------------------------------------------------------+
| TRADE AUTOMATION ENGINE (TAE)                                                           |
|   1. Policy Layer      -> Validates incoming signals against rules   -> Policy Matrix   |
|   2. Execution Layer   -> Pulls balance data, routes limit/market orders -> Execution M.|
+-----------------------------------------------------------------------------------------+
        | (Execution and Fill Events)
        v
+-----------------------------------------------------------------------------------------+
| PORTFOLIO MANAGEMENT ENGINE (PME)                                                       |
|   1. Position Layer    -> Directs and tracks active target sizes     -> Position Matrix |
|   2. Exposure Layer    -> Groups exposure boundaries (correlations)  -> Exposure Matrix |
|   3. Capital Layer     -> Tracks margins, available balances, equity -> Capital Matrix  |
|   4. Portfolio Layer   -> Consolidates overall financial state       -> Portfolio Matrix|
+-----------------------------------------------------------------------------------------+
        | (Portfolio/Regime Logs and Closed Trade Archives)
        v
+-----------------------------------------------------------------------------------------+
| PERFORMANCE ANALYTICS ENGINE (PAE)                                                      |
|   1. Trade Analytics   -> Reconstructs single transactions & slippage-> Trade Anal. M.  |
|   2. Strategy Anal.    -> Evaluates win/loss, factor, expectancy     -> Strategy Anal. M|
|   3. Risk Analytics    -> Measures drawdowns and Sharpe/Sortino      -> Risk Anal. M.   |
|   4. Performance Layer -> Evaluates performance against MME regimes  -> Performance M.  |
+-----------------------------------------------------------------------------------------+
```

---

## Appendix A — Formal Matrix Definitions

This appendix defines the authoritative physical schemas for all matrices produced by the Market Monitoring Engine. These schemas correspond to the dedicated specification documents under `docs/matrices/` and represent the canonical JSON serialization contracts. All enum values serialize as `SCREAMING_SNAKE_CASE`.

---

### A.1 Metrics Matrix Schema (MME — Layer 1)

Produced by the Metrics Layer for a single **Market Instance (Symbol × Timeframe)**. This is the foundational observation object: one `MarketSnapshot` per candle, containing the full `indicators` map of `IndicatorEvaluation` entries, each with nested `signals`, plus the attached higher-order matrices (Alignment, Analysis, Risk, Decision).

Full specification: [Metrics Matrix](../matrices/02-07-metrics-matrix.md).

```json
{
  "exchange": "Hyperliquid",
  "symbol": "BTC-USDT",
  "timeframe_secs": 180,
  "timestamp": 1752192000,
  "is_completed": true,
  "mid_price": "64012.5",
  "bid_price": "64012.0",
  "ask_price": "64013.0",
  "bid_size": "1.5",
  "ask_size": "0.8",
  "funding_rate": "0.0001",
  "open": "63890.0",
  "high": "64120.0",
  "low": "63850.0",
  "close": "64012.5",
  "volume": "182.4",
  "average_volume": "150.1",
  "open_interest": "1250000.0",
  "oi_delta_1h": "5000.0",
  "prev_day_px": "63500.0",
  "indicators": {
    "rsi": {
      "raw_value": 68.4,
      "normalized": -0.42,
      "state_label": "BULLISH_MOMENTUM",
      "confidence": 0.42,
      "signals": [
        {
          "kind": "Threshold",
          "direction": "Bearish",
          "status": "Active",
          "label": "OVERBOUGHT_DISTRIBUTION",
          "strength": 0.6,
          "age_bars": 2
        }
      ]
    },
    "macd": {
      "raw_value": 12.3,
      "normalized": 0.55,
      "state_label": "BULLISH_CROSSOVER",
      "values": { "line": 12.3, "signal": 9.8, "histogram": 2.5 },
      "confidence": 0.7,
      "signals": [
        {
          "kind": "Crossover",
          "direction": "Bullish",
          "status": "Confirmed",
          "label": "MACD_BULLISH_CROSSOVER",
          "strength": 0.8,
          "age_bars": 0
        }
      ]
    }
  },
  "context": {
    "trend": { "score": 0.62, "confidence": 0.71, "label": "STRONG_BULL" },
    "momentum": { "score": 0.40, "confidence": 0.55, "label": "BULL" },
    "volatility": { "score": 0.30, "confidence": 0.60, "label": "NORMAL" },
    "volume": { "score": 0.50, "confidence": 0.65, "label": "STRONG" },
    "liquidity": { "score": 0.45, "confidence": 0.55, "label": "ADEQUATE" },
    "regime": "TRENDING",
    "overall_score": 54,
    "overall_label": "WEAK_BULL"
  },
  "alignment": {
    "symbol": "BTC-USDT",
    "timeframes_present": 4,
    "dimensions": [
      { "score": 78.0, "state": "BULLISH", "confidence": 78.0 },
      { "score": 65.0, "state": "BULLISH", "confidence": 65.0 }
    ],
    "mtf_trend_alignment": 0.56,
    "mtf_momentum_alignment": 0.30,
    "mtf_overall_score": 41.0,
    "mtf_overall_label": "WEAK_BULL_MTF",
    "trend_agreement_pct": 75.0,
    "signal_cross_tf_count": 3
  },
  "analysis": { },
  "risk": { },
  "advisory": { },
  "decision_context": { },
  "statistical_context": { },
  "risk_profile": null
}
```

**Key structural rules:**
- All `Decimal` price/size fields serialize as **strings** for precision.
- `Option::None` fields are omitted via `skip_serializing_if`.
- Signals are **nested inside each indicator** under the `signals: [IndicatorSignal]` array — there is no top-level `signals` map.
- Divergence signals use `kind: "Divergence"` and are pushed onto the parent indicator's `signals` array (e.g., a bullish RSI divergence appears under `rsi.signals`).

---

### A.2 Alignment Matrix Schema (MME — Layer 2)

Produced by the Alignment Layer. Consumes multiple per-timeframe Metrics Matrices for one symbol and computes 10-dimensional cross-timeframe agreement.

Full specification: [Alignment Matrix](../matrices/02-01-alignment-matrix.md).

```json
{
  "symbol": "BTC-USDT",
  "timeframes_present": 4,
  "dimensions": [
    { "score": 78.0, "state": "BULLISH", "confidence": 78.0 },
    { "score": 65.0, "state": "BULLISH", "confidence": 65.0 },
    { "score": 55.0, "state": "NEUTRAL", "confidence": 55.0 },
    { "score": 60.0, "state": "BULLISH", "confidence": 60.0 },
    { "score": 75.0, "state": "BULLISH", "confidence": 75.0 },
    { "score": 33.3, "state": "BEARISH", "confidence": 33.3 },
    { "score": 100.0, "state": "BULLISH", "confidence": 100.0 },
    { "score": 88.0, "state": "BULLISH", "confidence": 88.0 },
    { "score": 70.0, "state": "BULLISH", "confidence": 70.0 },
    { "score": 100.0, "state": "BULLISH", "confidence": 100.0 }
  ],
  "mtf_trend_alignment": 0.56,
  "mtf_momentum_alignment": 0.30,
  "mtf_volume_alignment": 0.10,
  "mtf_volatility_alignment": 0.20,
  "mtf_overall_score": 41.0,
  "mtf_overall_label": "WEAK_BULL_MTF",
  "timeframe_alignments": [
    {
      "timeframe": "micro60",
      "timeframe_secs": 60,
      "trend_score": 0.5,
      "momentum_score": 0.3,
      "overall_score": 42,
      "regime": "TRENDING",
      "active_signals": 3,
      "price": 64012.5
    }
  ],
  "signal_cross_tf_count": 3,
  "trend_agreement_pct": 75.0
}
```

**The 10 Alignment Dimensions (ordered):**
| # | Dimension | Measures |
|---|-----------|----------|
| 0 | Trend | Directional trend agreement |
| 1 | Momentum | Momentum-vector agreement |
| 2 | Volume | Participation agreement |
| 3 | Volatility | Volatility-regime agreement |
| 4 | Structure | S/R role agreement |
| 5 | Signal | Cross-TF signal confluence |
| 6 | Regime | Regime-classification agreement |
| 7 | Confidence | Confidence consistency |
| 8 | Liquidity | RVOL consistency |
| 9 | Opportunity | Tradability agreement |

---

### A.3 Analysis Matrix Schema (MME — Layer 3)

Produced by the Analysis Layer. Consumes the Alignment Matrix and produces a structured diagnosis of market bias, regime, and quality.

Full specification: [Analysis Matrix](../matrices/02-02-analysis-matrix.md).

```json
{
  "symbol": "BTC-USDT",
  "bias": "BULLISH",
  "confidence": 0.82,
  "market_regime": "TRENDING_BULL",
  "trend_assessment": "HEALTHY",
  "momentum_assessment": "STABLE",
  "structure_assessment": "HEALTHY",
  "volatility_assessment": "NORMAL",
  "volume_assessment": "STRONG",
  "opportunity_analysis": "TREND_CONTINUATION",
  "market_quality": "GOOD",
  "market_interpretation": "Bullish trending market with healthy trend, stable momentum, healthy structure, normal volatility, and strong volume participation. Favors trend continuation.",
  "rationale": "MTF overall score 41/100 → BULLISH. Majority of 4 timeframes agree (75%). 3 signals across multiple timeframes.",
  "supporting_signals": ["fast180 (bullish): score +42, TRENDING regime, 3 signals"],
  "contradicting_signals": [],
  "timeframes_considered": 4
}
```

**Classification vocabularies:**
- **MarketBias:** `STRONG_BULLISH` (>40), `BULLISH` (20–40), `NEUTRAL` (−20 to 20), `BEARISH` (−40 to −20), `STRONG_BEARISH` (<−40)
- **MarketRegime:** `TRENDING_BULL`, `TRENDING_BEAR`, `RANGE`, `ACCUMULATION`, `DISTRIBUTION`, `EXPANSION`, `CONTRACTION`, `TRANSITION`
- **QualityLevel:** `POOR`, `WEAK`, `AVERAGE`, `GOOD`, `EXCELLENT`

The continuous **market_bias_score ∈ [−1, +1]** is the Alignment Matrix's `mtf_overall_score` divided by 100.

---

### A.4 Opportunity Matrix Schema (MME — Layer 4)

Produced by the Opportunity Layer. Consumes the Analysis Matrix and underlying Metrics Matrix signals to profile strategy-agnostic setup viability. The contract is **direction-neutral**: it does not emit a directional bias. Direction is the responsibility of the Decision Matrix and TAE.

Full specification: [Opportunity Matrix](../matrices/02-08-opportunity-matrix.md).

```json
{
  "symbol": "BTC-USDT",
  "primary_opportunity": "BREAKOUT",
  "opportunity_score": 85.0,
  "setup_quality": "PRIME",
  "confidence": 0.81,
  "profiles": [
    {
      "opportunity_type": "BREAKOUT",
      "score": 85.0,
      "preconditions_met": 3,
      "preconditions_total": 3,
      "notes": "Volatility expanding, structure healthy, compression released."
    },
    {
      "opportunity_type": "TREND_CONTINUATION",
      "score": 62.0,
      "preconditions_met": 2,
      "preconditions_total": 3,
      "notes": "Trend healthy but momentum stabilizing."
    }
  ],
  "contributing_signals": ["squeeze:COMPRESSION_RELEASE", "donchian:BREAKOUT_UP"],
  "invalidation_note": "Close back inside the prior Donchian channel invalidates the breakout."
}
```

**Setup Quality bands:** `Prime` (≥85), `Strong` (70–85), `Moderate` (50–70), `Marginal` (30–50), `None` (<30).

**Scoring model:** `score = 0.35·Q_ctx + 0.30·S_sig + 0.20·A_mtf + 0.15·F_fresh`

---

### A.5 Risk Matrix Schema (MME — Layer 5)

Produced by the Risk Layer. Consumes the Analysis Matrix and Opportunity Matrix, quantifying environmental danger across 9 unipolar dimensions on a `[0, 100]` scale, **independent of directional bias**.

Full specification: [Risk Matrix](../matrices/02-11-risk-matrix.md).

```json
{
  "symbol": "BTC-USDT",
  "market_risk": { "score": 35.0, "level": "LOW", "state": "STABLE", "confidence": 50.0, "evidence": ["High confidence"] },
  "volatility_risk": { "score": 45.0, "level": "MODERATE", "state": "STABLE", "confidence": 50.0, "evidence": ["BBWP elevated"] },
  "liquidity_risk": { "score": 15.0, "level": "VERY_LOW", "state": "STABLE", "confidence": 50.0, "evidence": ["Strong participation"] },
  "structure_risk": { "score": 25.0, "level": "LOW", "state": "STABLE", "confidence": 50.0 },
  "momentum_risk": { "score": 20.0, "level": "LOW", "state": "STABLE", "confidence": 50.0 },
  "signal_risk": { "score": 30.0, "level": "LOW", "state": "STABLE", "confidence": 50.0 },
  "execution_risk": { "score": 25.0, "level": "LOW", "state": "STABLE", "confidence": 50.0 },
  "reward_risk": { "score": 30.0, "level": "LOW", "state": "STABLE", "confidence": 50.0 },
  "overall_risk": { "score": 28.0, "level": "LOW", "state": "STABLE", "confidence": 50.0 }
}
```

**9 Risk Dimensions:**
| Dimension | Threat Vector |
|-----------|--------------|
| `market_risk` | General uncertainty from conflicting signals / weak structure |
| `volatility_risk` | Danger from abnormal price movement |
| `liquidity_risk` | Poor market participation / thin volume |
| `structure_risk` | Weak or damaged price structure |
| `momentum_risk` | Exhausted / diverging momentum |
| `signal_risk` | Conflicting or unreliable signals |
| `execution_risk` | Practical difficulty (spread, slippage, thin book) |
| `reward_risk` | Opportunity quality vs environmental uncertainty |
| `overall_risk` | Weighted aggregate: `0.15M + 0.15V + 0.15L + 0.10S + 0.15Mo + 0.10Sig + 0.10E + 0.10R` |

**RiskLevel bands:** `score ≥ 80 → Extreme`, `≥ 60 → High`, `≥ 40 → Moderate`, `≥ 20 → Low`, else `VeryLow`.

---

### A.6 Decision Matrix Schema (MME — Layer 6)

Produced by the Decision Layer. Synthesizes the Analysis, Opportunity, and Risk matrices into structured tactical guidance: trade readiness, directional guidance, protection/target strategies, and scenario pathways. The Decision Matrix is the terminal output the Trade Automation Engine consumes.

Full specification: [Decision Matrix](../matrices/02-04-decision-matrix.md).

```json
{
  "advisory": {
    "symbol": "BTC-USDT",
    "directional_guidance": "STRONG_LONG",
    "market_stance": "CONSTRUCTIVE",
    "opportunity_classification": "BREAKOUT",
    "strategy_environment": "TREND_FOLLOWING",
    "entry_guidance": "IMMEDIATE",
    "exit_guidance": "NO_WARNING",
    "protection_strategy": "ATR_BASED",
    "target_strategy": "RESISTANCE_BASED",
    "confidence_assessment": 72.0,
    "final_recommendation": "Strong long bias: BULLISH bias with 72% confidence, constructive stance in a trend-following environment. Breakout opportunity. Entry: immediate. Stop: ATR-based."
  },
  "decision_context": {
    "score": 54.0,
    "bias": "BULLISH",
    "confidence": 0.54,
    "contributing_indicators": ["ema_stack", "macd", "adx", "squeeze"]
  }
}
```

**Trade Readiness** (`READY` | `FORMING` | `WATCH` | `STAND_ASIDE`) is derived from directional guidance × risk-discounted confidence × market stance. Confidence is always risk-attenuated:

$$\text{confidence\_assessment} = \text{clamp}\Big(\text{analysis.confidence} \times \big(1 - \tfrac{\text{overall\_risk}}{100}\big) \times 100,\ 0,\ 100\Big)$$

---

### A.7 Overview Matrix Schema (MME — Layer 7)

Produced by the Overview Layer. Aggregates every symbol's Decision Matrix into cross-market breadth indices, asset rankings, and a systemic risk score consumed by the PME safety veto.

Full specification: [Overview Matrix](../matrices/02-09-overview-matrix.md).

```json
{
  "global_market_bias": "BULLISH",
  "market_breadth": "POSITIVE",
  "regime_distribution": { "TRENDING_BULL": 0.6, "RANGE": 0.4 },
  "opportunity_distribution": { "BREAKOUT": 2, "TREND_CONTINUATION": 1 },
  "risk_distribution": {
    "low_pct": 60.0,
    "moderate_pct": 40.0,
    "high_pct": 0.0,
    "risk_environment": "LOW_RISK"
  },
  "asset_ranking": [
    { "symbol": "BTC-USDT", "score": 87.0, "bias": "Long", "confidence": 75.0, "regime": "TrendFollowing", "risk_level": "MODERATE" }
  ],
  "market_synchronization": "SYNCHRONIZED",
  "market_health": "HEALTHY",
  "global_summary": "3 active instances across 3 symbols. Global bias: BULLISH with positive market breadth.",
  "instance_count": 3,
  "active_symbols": ["BTC-USDT", "ETH-USDT", "SOL-USDT"]
}
```

**Systemic Risk Score** (consumed by PME veto loop):

$$\text{SystemicRisk} = 0.6 \cdot \text{high\_pct} + 0.4 \cdot \text{sync\_penalty}$$

---

## Appendix B — Complete Indicator, Signal, and SignalKind Manifest

This appendix provides the definitive registry-verified manifest of all 50 indicators, 100 signal-kind declarations, and 12 SignalKind types in the platform. Counts are authoritative — drawn from `crates/shared/src/indicators/registry.rs`.

---

### B.1 Complete Indicator Registry (50 entries, 8 groups)

#### TREND (10)

| Key | Display Name | Class | Dir | SignalKinds |
|-----|-------------|-------|-----|-------------|
| `ema_stack` | EMA Ribbon | Lagging | Y | StackChange, Crossover×4 |
| `supertrend` | Supertrend | Lagging | Y | TrendFlip, Crossover×2, BandTouch |
| `donchian` | Donchian | Lagging | Y | Breakout×2, BandTouch×2, LevelTest×2 |
| `keltner` | Keltner | Lagging | Y | Breakout×2, BandTouch×2, LevelTest×2 |
| `adx` | ADX | Lagging | N (Gate) | TrendFlip, Threshold |
| `vwap` | VWAP | Lagging | Y | LevelTest |
| `anchored_vwap` | Anchored VWAP | Lagging | Y | LevelTest×2, Crossover×2 |
| `ichimoku` | Ichimoku Cloud | Hybrid | Y | Crossover×2, Breakout×2, LevelTest×3, TrendFlip×2 |
| `hull_ma` | Hull MA | Lagging | Y | Crossover×2 |
| `psar` | Parabolic SAR | Lagging | Y | TrendFlip×2, Crossover×3 |

#### MOMENTUM (7)

| Key | Display Name | Class | Dir | Div | SignalKinds |
|-----|-------------|-------|-----|-----|-------------|
| `rsi` | RSI | Leading | Y | Y | ZeroLineCross, Divergence, Threshold×5 |
| `stochastic` | Stochastic | Leading | Y | Y | Crossover×2, ZeroLineCross×2, Divergence, Threshold×4 |
| `chandemo` | Chande MO | Leading | Y | Y | ZeroLineCross, Divergence, Threshold×4 |
| `williams_r` | Williams %R | Leading | Y | — | Threshold, ZeroLineCross |
| `awesome_oscillator` | Awesome Oscillator | Leading | Y | — | ZeroLineCross×2, Threshold×2 |
| `cci` | CCI | Leading | Y | — | Threshold×4, ZeroLineCross |
| `macd` | MACD | Lagging | Y | Y | Crossover×2, ZeroLineCross, Divergence, Threshold |

#### VOLUME (7)

| Key | Display Name | Class | Dir | Div | SignalKinds |
|-----|-------------|-------|-----|-----|-------------|
| `force_index` | Force Index | Hybrid | Y | — | ZeroLineCross, Threshold |
| `volume` | Volume | Hybrid | N (Gate) | — | VolumeClimax |
| `rvol` | RVOL | Hybrid | N (Gate) | — | VolumeClimax |
| `volume_profile` | Volume Profile | Hybrid | Y | — | Breakout×2, LevelTest×2 |
| `obv` | OBV | Lagging | Y | Y | TrendFlip×2, Divergence×2, Threshold×3 |
| `cmf` | Chaikin MF | Hybrid | Y | Y | ZeroLineCross×2, Divergence×2, Threshold×4 |
| `mfi` | Money Flow Idx | Hybrid | Y | Y | Threshold×4, Divergence×2 |

#### VOLATILITY (6)

| Key | Display Name | Class | Dir | Div | SignalKinds |
|-----|-------------|-------|-----|-----|-------------|
| `stddev_channel` | StdDev Channel | Hybrid | Y | — | Breakout×2, BandTouch×2, LevelTest |
| `atr` | ATR | Lagging | N (Gate) | — | Threshold, CompressionRelease |
| `bollinger` | Bollinger | Hybrid | Y | — | Breakout×2, BandTouch×2, LevelTest×3 |
| `bbwp` | BBWP | Leading | N (Gate) | — | CompressionRelease, Threshold |
| `squeeze` | TTM Squeeze | Hybrid | Y | Y | CompressionRelease×3, Divergence, Threshold |
| `hv` | Historical Volatility | Lagging | N (Gate) | — | Threshold |

#### STRUCTURE (5)

| Key | Display Name | Class | Dir | SignalKinds |
|-----|-------------|-------|-----|-------------|
| `fibonacci` | Fibonacci | Leading | Y | LevelTest |
| `support_resistance` | Support/Resistance | Leading | Y | LevelTest×2, Breakout×2 |
| `pivot_points` | Pivot Points | Leading | Y | LevelTest×3, Breakout×2, Crossover×2 |
| `patterns` | Patterns | Leading | Y | PatternForming×3 |
| `candlestick` | Candlestick | Leading | Y | PatternForming×2 |

#### REGIME (4)

| Key | Display Name | Class | Dir | SignalKinds |
|-----|-------------|-------|-----|-------------|
| `aroon` | Aroon | Hybrid | Y | Crossover×2, TrendFlip×2, Threshold×2 |
| `choppiness` | Choppiness | Hybrid | N (Gate) | Threshold×2, CompressionRelease |
| `linreg_slope` | LinReg Slope | Lagging | Y | ZeroLineCross, Threshold×2 |
| `zscore` | Z-Score | Leading | Y | Threshold, ZeroLineCross |

#### INSTITUTIONAL (4)

| Key | Display Name | Class | Dir | SignalKinds |
|-----|-------------|-------|-----|-------------|
| `smc_structure` | SMC Structure | Leading | Y | Breakout, TrendFlip |
| `smc_liquidity` | SMC Liquidity | Leading | Y | PatternForming |
| `smc_fvg` | SMC Fair Value Gap | Leading | Y | LevelTest |
| `smc_order_blocks` | SMC Order Blocks | Leading | Y | LevelTest×2, TrendFlip×2 |

#### DERIVATIVES DATA (7)

| Key | Display Name | Class | Dir | SignalKinds |
|-----|-------------|-------|-----|-------------|
| `open_interest` | Open Interest | Leading | N (Gate) | Threshold |
| `oi_delta` | OI Delta | Leading | Y | Threshold, ZeroLineCross |
| `funding_rate` | Funding Rate | Leading | N (Gate) | Threshold |
| `oi_price_divergence` | OI-Price Divergence | Leading | Y | Divergence |
| `order_flow_imbalance` | Order Flow Imbalance | Leading | Y | Threshold |
| `spread` | Spread | Leading | N (Gate) | Threshold |
| `depth_bias` | Depth Bias | Leading | Y | Threshold |

---

### B.2 Summary Counts

| Metric | Count |
|--------|-------|
| Total Registry Entries | **50** (10 Trend + 7 Momentum + 7 Volume + 6 Volatility + 5 Structure + 4 Regime + 4 Institutional + 7 Derivatives) |
| Directional (scoring contributors) | **40** |
| Non-Directional Gates | **10** (`adx`, `volume`, `rvol`, `atr`, `bbwp`, `hv`, `choppiness`, `funding_rate`, `spread`, `open_interest`) |
| Indicators Supporting Divergence | **8** (`rsi`, `stochastic`, `chandemo`, `macd`, `obv`, `cmf`, `mfi`, `squeeze`) |
| Total Signal-Kind × Indicator Declarations | **102** (one declaration per `(indicator, SignalKind)` pair; `×N` in the index counts multiplicity *within* a single declaration, e.g. 5 RSI threshold zones) |
| SignalKind Types | **12** |

**Important:** Divergence companions (e.g., `rsi_divergence`, `macd_divergence`) are **NOT** separate registry entries and produce **NO** separate JSON keys. A divergence is an `IndicatorSignal { kind: Divergence, ... }` emitted on the parent indicator's `signals` array. Eight parent indicators are annotated with `supports_divergence: true` in the registry.

---

### B.3 SignalKind Frequency Table

Canonical counts: one row per `(indicator, SignalKind)` pair. Cross-checked against the [Indicator Index](../engines/market-monitoring-engine/indicators/04-02-00-indicator-index.md).

| # | SignalKind | Declarations | Description |
|---|-----------|-------------|-------------|
| 1 | `Divergence` | **9** | 8 nested on parent (`supports_divergence: true`: `rsi`, `stochastic`, `chandemo`, `macd`, `obv`, `cmf`, `mfi`, `squeeze`) + 1 standalone (`oi_price_divergence`, own registry entry). Price/indicator directional disagreement. |
| 2 | `Crossover` | **10** | Two series cross (e.g., MACD line × signal). |
| 3 | `Threshold` | **26** | Value enters a named zone (e.g., RSI ≥ 70). |
| 4 | `Breakout` | **9** | Price breaks a structural boundary. |
| 5 | `BandTouch` | **5** | Price contacts a channel/band edge. |
| 6 | `ZeroLineCross` | **12** | Oscillator crosses its zero/mid line. |
| 7 | `CompressionRelease` | **4** | Volatility squeeze fires. |
| 8 | `LevelTest` | **13** | Price tests a horizontal level (S/R, fib, pivot). |
| 9 | `TrendFlip` | **8** | Directional regime reverses (Supertrend, PSAR). |
| 10 | `VolumeClimax` | **2** | Abnormal volume surge. |
| 11 | `StackChange` | **1** | EMA ribbon reorders. |
| 12 | `PatternForming` | **3** | Chart/candlestick pattern detected. |
| | **TOTAL** | **102** | |

---

### B.4 How Divergence Signals Work

Divergence is handled as a **signal on the parent indicator**, not as a separate entry:

1. The parent indicator (e.g., `rsi`) has `supports_divergence: true` and includes `SignalKind::Divergence` in its `signal_types`.
2. When divergence is detected via `DivergenceState`, an `IndicatorSignal { kind: Divergence, direction: Bullish|Bearish, status: Confirmed|Potential, ... }` is pushed into the parent's `signals` array.
3. The parent's `normalized` value and `state_label` are also modulated by the divergence state (e.g., confirmed bullish divergence forces RSI `normalized = 1.0`, `state_label = "OVERSOLD_ACCUMULATION"`).
4. The JSON output nests divergence signals under the parent key — there is never a separate `"rsi_divergence"` key in the indicators map.

```json
{
  "rsi": {
    "raw_value": 28.5,
    "normalized": 1.0,
    "state_label": "OVERSOLD_ACCUMULATION",
    "signals": [
      { "kind": "Divergence", "direction": "Bullish", "status": "Confirmed",
        "label": "CONFIRMED_BULLISH_DIVERGENCE", "strength": 1.0, "age_bars": 0 },
      { "kind": "Threshold", "direction": "Bullish", "status": "Active",
        "label": "OVERSOLD", "strength": 0.0, "age_bars": 0 }
    ]
  }
}
```

---

### B.5 Cross-References

- [Indicator Index](../engines/market-monitoring-engine/indicators/04-02-00-indicator-index.md) — Per-indicator documentation index.
- [MME Signals Guide](../engines/market-monitoring-engine/03-02-10-mme-signals-guide.md) — Signal detection rulebook.
- [Metrics Matrix](../matrices/02-07-metrics-matrix.md) — IndicatorEvaluation and IndicatorSignal schemas.
- [MME Layer 1 — Metrics](../engines/market-monitoring-engine/03-02-02-mme-layer1-metrics.md) — Producing-layer specification.

---

## Appendix C — Formal Architectural Vocabulary

*   **Alignment:** The degree of agreement regarding market direction or structure among multiple independent timeframes of a single symbol.
*   **Analytical Feature:** A standardized, quantitative metric processed from raw indicators and signals designed to serve as a reusable analytical block.
*   **Bias:** The dominant directional interpretation of an asset's price vector (e.g., Bullish, Neutral, Bearish).
*   **Confluence:** The intersection and agreement of multiple independent technical indicators or signals within a single timeframe.
*   **Drawdown:** The peak-to-trough decline in capital reserves for a specific trading account or strategy, expressed as a percentage or nominal dollar value.
*   **Engine:** The largest independent functional block within the trading platform, representing an autonomous business domain.
*   **Execution Policy:** A deterministic, user-configured trigger rule evaluated by the Trade Automation Engine to govern order dispatch.
*   **Layer:** An isolated sequential step within an engine's processing pipeline that transforms data to a higher level of abstraction.
*   **Market Instance:** The pairing of a unique financial symbol and a defined temporal timeframe (e.g., ETHUSDT × 5 Minutes).
*   **Market Phase:** The active stage of an asset within its broader market cycle (`ACCUMULATION`, `MARKUP`, `DISTRIBUTION`, `MARKDOWN`).
*   **Market Regime:** The underlying environmental behavior of an asset, defining the structural context (e.g., `EXPANSION`, `RANGE`).
*   **Matrix:** The structured, immutable output produced by an analytical layer, serving as the interface contract between stages.
*   **Opportunity Score:** A numeric representation from 0 to 100 expressing the density of high-probability entry criteria present in the market.
*   **Risk Score:** A numeric representation from 0 to 100 expressing the structural, technical, and execution dangers inherent in the current market environment, independent of directional bias.
*   **Slippage:** The difference between the targeted execution price of an automation policy and the actual filled price on an exchange.
*   **Trade Readiness:** A classification status indicating whether technical and structural conditions have sufficiently matured to support an entry attempt.