
# Decision Guidance Layer Specification

## Advisory Matrix Architecture

Version: 1.0

---

# Purpose

The **Decision Guidance Layer** is the fifth analytical layer of the Market Monitoring Platform.

Its responsibility is to transform complete market intelligence and risk assessment into structured human-facing guidance.

This layer represents the final analytical interpretation before the global market overview.

It combines:

- Market understanding
- Multi-timeframe alignment
- Opportunity evaluation
- Risk assessment

to provide a clear summary of what the current market conditions suggest.

The Decision Guidance Layer does not execute trades.

It does not control capital.

It does not replace human decision-making.

Its purpose is to provide an explainable recommendation framework.

The Decision Guidance Layer answers:

> **"Given the current market condition and associated risk, what is the most reasonable interpretation and possible action direction?"**

---

# Position in the Architecture

```text
Market Monitoring Platform


├── Market Telemetry Layer
│
│     └── Metrics Matrix
│
├── Signal Correlation Layer
│
│     └── Alignment Matrix
│
├── Market Intelligence Layer
│
│     └── Analysis Matrix
│
├── Risk Assessment Layer
│
│     └── Risk Matrix
│
├── Decision Guidance Layer
│
│     └── Advisory Matrix
│
└── Market Synthesis Layer
     
      └── Overview Matrix
````

---

# Information Flow

The Decision Guidance Layer consumes all previous analytical layers.

```text
Raw Market Data

        │

        ▼

Market Telemetry Layer

        │

        ▼

Metrics Matrix

        │

        ▼

Signal Correlation Layer

        │

        ▼

Alignment Matrix

        │

        ▼

Market Intelligence Layer

        │

        ▼

Analysis Matrix

        │

        ▼

Risk Assessment Layer

        │

        ▼

Risk Matrix

        │

        ▼

Decision Guidance Layer

        │

        ▼

Advisory Matrix
```

---

# Core Responsibility

The Decision Guidance Layer converts analytical information into structured guidance.

Previous layers answer:

```text
What is happening?

↓

Do timeframes agree?

↓

What does the market condition mean?

↓

How risky is the environment?
```

The Advisory Matrix answers:

```text
Given all available information:

What direction is favored?

What type of opportunity exists?

How confident should we be?

What conditions should be respected?
```

---

# Primary Component

The Decision Guidance Layer contains one primary analytical component:

```text
Advisory Matrix
```

Every monitored asset receives one independent Advisory Matrix.

Example:

```text
BTCUSDT


Analysis Matrix

+

Risk Matrix

↓

Advisory Matrix
```

---

# Advisory Matrix Scope

An Advisory Matrix represents:

```text
One Asset

×

Complete Analytical Context

×

Current Market Snapshot
```

It is the first layer that produces human-facing recommendations.

---

# Important Design Principle

The Advisory Matrix provides guidance.

It does not make autonomous decisions.

The distinction is:

```text
Decision Engine

=

System chooses and executes


Advisory Matrix

=

System explains and recommends
```

The final decision remains external to the platform.

---

# Advisory Matrix Architecture

The Advisory Matrix is composed of:

```text
Advisory Matrix


├── Directional Guidance
│
├── Market Stance
│
├── Opportunity Classification
│
├── Strategy Environment
│
├── Entry Guidance
│
├── Exit Guidance
│
├── Stop Loss Guidance
│
├── Take Profit Guidance
│
├── Confidence Assessment
│
└── Final Recommendation
```

---

# Component 1 — Directional Guidance

## Purpose

Determines the direction that the current market conditions favor.

This is the directional recommendation component.

---

# Possible Values

```text
Strong Long Bias

Long Bias

Neutral

Short Bias

Strong Short Bias

Avoid Directional Exposure
```

---

# Inputs

Uses:

- Market Bias
    
- Trend Assessment
    
- Momentum Assessment
    
- Structure Assessment
    
- Alignment Matrix
    

---

# Example

```text
Directional Guidance:

Long Bias


Reason:

Strong multi-timeframe bullish alignment
```

---

# Important Principle

Directional guidance is not an instruction to open a position.

It represents the direction supported by the available evidence.

---

# Component 2 — Market Stance

## Purpose

Represents the overall recommended attitude toward the current market.

Direction alone is insufficient.

A bullish market can still require caution.

---

# Possible Values

```text
Aggressive

Constructive

Neutral

Cautious

Avoid
```

---

# Examples

Strong trend + low risk:

```text
Market Stance:

Constructive
```

Strong trend + extreme risk:

```text
Market Stance:

Cautious
```

---

# Component 3 — Opportunity Classification

## Purpose

Defines what type of opportunity, if any, currently exists.

---

# Possible Values

```text
Trend Continuation

Breakout

Pullback

Mean Reversion

Reversal

No Clear Opportunity
```

---

# Inputs

Uses:

- Opportunity Analysis
    
- Market Regime
    
- Structure
    
- Momentum
    
- Risk
    

---

# Example

```text
Opportunity:

Trend Continuation


Quality:

High
```

---

# Component 4 — Strategy Environment

## Purpose

Describes what type of trading environment currently exists.

The system does not select a strategy.

It identifies compatible environments.

---

# Possible Values

```text
Trend Following Environment

Breakout Environment

Mean Reversion Environment

High Volatility Environment

Low Activity Environment

Unfavorable Environment
```

---

# Example

```text
Environment:

Trend Following Environment
```

---

# Component 5 — Entry Guidance

## Purpose

Provides contextual information about possible entry conditions.

It does not generate orders.

---

# Possible Values

```text
Immediate Observation

Wait For Confirmation

Wait For Pullback

Wait For Breakout

No Entry Context
```

---

# Example

```text
Entry Guidance:

Wait For Pullback


Reason:

Price extended from support zone
```

---

# Component 6 — Exit Guidance

## Purpose

Provides information about conditions that may invalidate the current interpretation.

---

# Possible Values

```text
Trend Weakening

Momentum Exhaustion

Structure Breakdown

Risk Increasing

No Exit Warning
```

---

# Example

```text
Exit Guidance:

Monitor Momentum Exhaustion
```

---

# Component 7 — Stop Loss Guidance

## Purpose

Provides risk-management context for potential protection levels.

The Advisory Matrix does not execute stops.

It provides methodology guidance.

---

# Possible Methods

```text
Structure Based

Volatility Based

ATR Based

Support/Resistance Based

No Recommendation
```

---

# Example

```text
Stop Guidance:

ATR Based


Reason:

High volatility environment
```

---

# Component 8 — Take Profit Guidance

## Purpose

Provides target methodology guidance.

---

# Possible Methods

```text
Resistance Based

Risk/Reward Based

Volatility Based

Trailing Method

No Recommendation
```

---

# Example

```text
Target Guidance:

Resistance Based


Reason:

Nearby structural resistance
```

---

# Component 9 — Confidence Assessment

## Purpose

Measures confidence in the final advisory output.

---

# Inputs

Uses:

- Alignment Confidence
    
- Market Quality
    
- Risk Level
    
- Signal Confidence
    
- Analysis Confidence
    

---

# Scale

```text
0 - 100%
```

---

# Example

```text
Advisory Confidence:

84%
```

---

# Component 10 — Final Recommendation

## Purpose

Creates the final human-readable summary.

This is the highest abstraction level for one asset.

---

# Example

```text
BTCUSDT


Direction:

Long Bias


Market Stance:

Constructive


Opportunity:

Trend Continuation


Risk:

Moderate


Confidence:

82%


Recommendation:

The asset currently shows a bullish environment supported by multi-timeframe alignment and healthy structure. A continuation scenario is favored, but confirmation should be respected due to elevated volatility.
```

---

# Advisory Matrix Output

The complete Advisory Matrix produces:

```text
Advisory Matrix


Directional Guidance

↓

Market Stance

↓

Opportunity Classification

↓

Strategy Environment

↓

Entry Guidance

↓

Exit Guidance

↓

Stop Loss Guidance

↓

Take Profit Guidance

↓

Confidence Assessment

↓

Final Recommendation
```

---

# Relationship With Previous Layers

The Advisory Matrix is the result of all previous analytical stages.

```text
Metrics Matrix

↓

Alignment Matrix

↓

Analysis Matrix

↓

Risk Matrix

↓

Advisory Matrix
```

Each layer contributes a different perspective.

---

# Example Complete Flow

## Telemetry Layer

```text
Indicators:

Bullish trend


Signals:

Breakout


Derived Metrics:

Strong momentum
```

↓

## Correlation Layer

```text
Alignment:

High across timeframes
```

↓

## Intelligence Layer

```text
Market Bias:

Bullish


Regime:

Trending Bull
```

↓

## Risk Layer

```text
Risk:

Moderate


Main Concern:

Volatility expansion
```

↓

## Advisory Layer

```text
Direction:

Long Bias


Opportunity:

Trend Continuation


Guidance:

Wait for confirmation
```

---

# Design Principles

---

# Human-Centered

The Advisory Matrix exists to support human decisions.

---

# No Autonomous Trading

The Advisory Matrix never:

- Opens positions
    
- Closes positions
    
- Places orders
    
- Manages accounts
    

---

# Explainable Recommendations

Every recommendation must be supported by analytical evidence.

Example:

```text
Long Bias

because:

- Bullish trend
- High alignment
- Positive momentum
- Acceptable risk
```

---

# Risk-Aware Guidance

Recommendations must always incorporate risk context.

A strong opportunity with extreme risk must reflect that uncertainty.

---

# Strategy Agnostic

The Advisory Matrix provides guidance without forcing a specific trading methodology.

---

# Deterministic

Identical analytical inputs must generate identical guidance.

---

# Architectural Responsibility

The Decision Guidance Layer answers:

> **"Considering market conditions and risk, what is the most reasonable interpretation and possible direction?"**

The Advisory Matrix is the final analytical layer for an individual asset.

After this point, the platform moves to the final layer:

the **Market Synthesis Layer**, where all monitored assets are combined into a unified global market representation through the **Overview Matrix**.