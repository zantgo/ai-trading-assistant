# IASL — Institutional AI Synthesis Layer

> **Layer 8 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: COMPLETE** — Two-agent pipeline (Analyst → Trader) with heuristic fallback and Journal Agent.

---

## Purpose

The Institutional AI Synthesis Layer (IASL) answers the execution question:

> **Given all deterministic analysis from layers 1-7, what is the complete market picture and the optimal trading action?**

IASL implements a **Two-Agent Pipeline**: the Analyst Agent receives ALL deterministic data (51 indicators, DecisionContext, MarketContext, SIL, IRML, S/R levels, price history) and produces a comprehensive institutional market document. The Trader Agent receives this document plus the current position and IRML risk profile, and produces the final trading decision.

IASL is the bridge between pure mathematics (layers 1-7) and human-readable, actionable intelligence. The Analyst prepares; the Trader decides. Neither performs redundant calculations — all math is done in lower layers.

**Code location:** `crates/engine/src/llm/agents.rs` (two-agent pipeline), `crates/engine/src/llm/prompts.rs` (system prompts), `crates/engine/src/llm/types.rs` (AnalystDocument, TraderDecision schemas), `crates/engine/src/services/analyzer.rs` (pipeline orchestration).

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| 51 indicator DTOs | ITIL indicator map | JSON array of NormalizedIndicatorValue |
| DecisionContext | IDCL (17 metrics) | JSON: bullish_probability, consensus, risk_level, trade_quality, trade_readiness, etc. |
| MarketContext | IRCL + MarketContext synthesis | JSON: trend, momentum, volatility, volume, liquidity dimensions + regime |
| Support levels | ISML S/R engine | JSON string array |
| Resistance levels | ISML S/R engine | JSON string array |
| Price history | L0 candle buffer | JSON float array (last 100 closes) |
| Current position | User input / automation state | String: "None" / "Long" / "Short" |
| Entry price | User input / paper trading state | String / null |
| IRML risk profile | IRML | JSON: overall_risk, 6 categories, drawdown_state, permission, opportunity_score, reward_risk |
| Symbol | Config / user selection | String |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| AnalystDocument | JSON: 8-section institutional analysis | Trader Agent, frontend display |
| TraderDecision | JSON: action, confidence(0-100), rationale, risk_notes | Frontend, IPEL (trade journaling), IEPL (execution trigger) |
| Heuristic fallback decision | Same TraderDecision format | Used when no LLM API key configured |

---

## Sub-Components

---

### A. Two-Agent Pipeline Architecture

The IASL pipeline is **sequential, not parallel**. The Analyst Agent produces a document; the Trader Agent reads only that document. This ensures a clean separation of concerns — analysis and decision are distinct cognitive steps.

```
Deterministic Data (L1-L7)
        │
        ▼
┌─────────────────────────┐
│   ANALYST AGENT          │  ← Receives ALL data
│   Information Prep       │     Temperature: 0.3
│   Produces:              │     Max tokens: 2048
│   AnalystDocument (8 §)  │     Output: strict JSON
└───────────┬─────────────┘
            │ AnalystDocument (JSON)
            ▼
┌─────────────────────────┐
│   TRADER AGENT           │  ← Receives ONLY document
│   Decision Execution     │     + position + IRML
│   Produces:              │     Temperature: 0.2
│   TraderDecision         │     Max tokens: 1024
│   {action, conf,         │     Output: strict JSON
│    rationale, risk_notes}│
└─────────────────────────┘
```

**Why two agents instead of one?**
- The Analyst Agent can focus entirely on accurate, thorough data interpretation without the cognitive load of making a trading decision
- The Trader Agent evaluates the document with fresh eyes, applying strict decision rules and risk governance
- Each agent has its own optimized temperature (0.3 for analysis, 0.2 for decisions) and token budget
- If the Analyst produces a poor document, the Trader can still make a sound decision because the decision logic is in the Trader's prompt, not the Analyst's

**Why not 6 parallel domain agents + orchestrator?**
- The previous multi-agent design (Trend/Volatility/Structure/Risk/Position/Institutional agents → Master Orchestrator) was replaced in v3.0 for efficiency
- A single Analyst reading all 51 indicators can see cross-domain patterns that isolated agents would miss
- The 2-agent pipeline reduces API costs (2 calls instead of 7 per cycle) and eliminates agent-to-agent communication complexity
- Token savings: ~57K per cycle (2 agents) vs ~150K+ (7 agents)

---

### B. Analyst Agent — Information Preparation

The Analyst Agent is a single LLM call that receives ALL deterministic data and produces an 8-section institutional market analysis document.

**Input payload (JSON):**
```json
{
  "symbol": "BTC",
  "current_price": 48750.0,
  "indicators": [/* 51 indicator DTOs with normalized values, state_labels, values sub-maps */],
  "decision_context": {/* 17 IDCL fields */},
  "market_context": {/* 6 dimensions + regime */},
  "support_levels": ["48500", "48200"],
  "resistance_levels": ["49200", "49800"],
  "price_history": [/* last 100 closes */]
}
```

**Output: AnalystDocument (8 sections):**
```json
{
  "market_summary": "2-3 sentence summary of overall market condition: regime, directional bias, weighted confluence score. Are indicators aligned or fragmented?",
  "trend_indicators": "EMA stack configuration. ADX strength and regime. Supertrend direction. Ichimoku cloud position. Key trend signal summary.",
  "momentum_indicators": "RSI level and zone. MACD line/signal relationship and histogram trend. Stochastic position. CCI reading. Any momentum divergences (potential or confirmed).",
  "volatility_indicators": "Bollinger Band width and price position within bands. BBWP percentile. ATR regime (expanding/contracting/stable). Squeeze status and momentum direction.",
  "volume_indicators": "RVOL relative to thresholds (institutional/climax). OBV/CMF accumulation/distribution. MFI reading. VWAP bias and proximity.",
  "structure_indicators": "Key support and resistance levels. Price proximity to nearest S/R. Fibonacci golden pocket status. Active chart patterns. Pivot point levels.",
  "active_signals": "List all confirmed signals: RSI/MACD divergences (confirmed only), squeeze releases, EMA/DI crossovers, breakout events. Note signal age if relevant.",
  "confluence_summary": "Weighted confluence score, directional consensus percentage, regime confidence, statistical context (predictability, anomaly score). Overall market cleanliness assessment."
}
```

**Analyst Agent Rules (from system prompt):**
- Be thorough but concise. Each section 2-3 descriptive sentences.
- Use normalized [-1,+1] magnitude conventions: |v| > 0.7 = strong, |v| = 0.3-0.7 = moderate, |v| < 0.3 = weak/neutral.
- Reference actual numerical values from indicator DTOs to support observations.
- **Do NOT suggest any trading action.** Do not say "consider buying" or "suggest waiting for entry."
- Classify divergences as POTENTIAL (unconfirmed, no S/R break) vs CONFIRMED (S/R boundary broken).
- Output strictly JSON. No markdown, no commentary outside the JSON object.

**Temperature:** 0.3 — moderate creativity for descriptive prose while maintaining factual accuracy.
**Max tokens:** 2048 — sufficient for 8 sections of professional analysis.

---

### C. Analyst Document Schema

The AnalystDocument struct (Rust, `llm/types.rs`):

| Field | Type | Description |
|-------|------|-------------|
| `market_summary` | String | Overall regime, directional bias, confluence score summary. 2-3 sentences. |
| `trend_indicators` | String | EMA stack state, ADX strength/regime, Ichimoku, Supertrend, PSAR. 2-3 sentences. |
| `momentum_indicators` | String | RSI level/zone, MACD line/signal/histogram, Stochastic, CCI, Williams %R, AO, Force Index. Any divergences (potential/confirmed). 2-3 sentences. |
| `volatility_indicators` | String | BB width/position, BBWP percentile, ATR regime, Squeeze status/momentum, HV level. 2-3 sentences. |
| `volume_indicators` | String | RVOL vs thresholds, OBV/CMF/MFI accumulation/distribution, VWAP bias/proximity. 2-3 sentences. |
| `structure_indicators` | String | Key S/R levels, Fibonacci GP, active patterns, pivot points, SMC zones. 2-3 sentences. |
| `active_signals` | String | All confirmed signals: divergences, squeeze releases, crossovers, breakouts. Signal age if relevant. 1-2 sentences. |
| `confluence_summary` | String | Weighted confluence score, consensus %, regime confidence, statistical context. 2-3 sentences. |

**Magnitude conventions (applied consistently):**
| \|normalized\| | Adjective |
|---------------|-----------|
| > 0.70 | Strong, decisive |
| 0.30 – 0.70 | Moderate, developing |
| < 0.30 | Weak, neutral, uncertain |

---

### D. Trader Agent — Decision Execution

The Trader Agent reads the Analyst Document and makes a strict, rule-bound trading decision. It does NOT receive raw indicator data — it trusts the Analyst's observations.

**Input payload (JSON):**
```json
{
  "analyst_document": {/* full 8-section AnalystDocument JSON */},
  "position": "None",
  "entry_price": "None (no open position)",
  "symbol": "BTC",
  "risk_profile": {/* full IRML JSON or null */}
}
```

**Output: TraderDecision:**
```json
{
  "action": "Wait",
  "confidence": 72,
  "rationale": "Confluence score of +45 indicates moderate bullish alignment. However, market_summary notes fragmented momentum with RVOL below institutional threshold (1.3). Price near resistance at $49,200 without volume confirmation. Waiting for clearer entry signal.",
  "risk_notes": "Volatility risk elevated (71st percentile). RR recommended ratio 1:1.85 may not be achievable from current levels to nearest resistance. Permission: Allowed, Exposure: Normal."
}
```

**Trader Agent Rules (from system prompt):**
1. **Position-gated actions:**
   - Position = "Long" or "Short" → ONLY "Hold" or "Close"
   - Position = "None" → ONLY "Wait", "Open Long", or "Open Short"
2. **Confluence-driven decisions:** High-confidence signals (multiple indicator groups aligned) → action. Conflicting signals → "Wait".
3. **Divergence weighting:** Confirmed divergences carry more weight than unconfirmed.
4. **Squeeze release:** Squeeze release with confirming momentum is a strong signal.
5. **Level proximity:** Price near S/R levels with indicator confirmation warrants attention.
6. **Confidence score:** 0-100 integer reflecting strength of confluence for the decision.

**Temperature:** 0.2 — low creativity, highly deterministic decision-making.
**Max tokens:** 1024 — sufficient for action + confidence + rationale + risk notes.

---

### E. Decision Categories & Rules

The Trader Agent must select exactly one action from the permitted set.

**When FLAT (position = None):**

| Action | Conditions |
|--------|-----------|
| **Open Long** | Strong bullish confluence (score > +40). Multiple groups aligned (trend + momentum + volume). Confirmed bullish divergence or squeeze release. IRML permission ≥ Restricted. |
| **Open Short** | Strong bearish confluence (score < −40). Multiple groups aligned. Confirmed bearish divergence or squeeze release. IRML permission ≥ Restricted. |
| **Wait** | Confluence neutral (−15 to +15). Conflicting signals across groups. IRML permission Suspended or Emergency Stop. RVOL < 1.0. Transitional regime. |

**When POSITIONED (position = Long or Short):**

| Action | Conditions |
|--------|-----------|
| **Hold** | Original trade thesis remains valid. No confirmed opposing divergences. No structural breakdown (CHoCH absent). IRML risk stable or improving. |
| **Close** | Opposite confluence score > 60% (ICSL trigger). Confirmed opposing divergence with S/R break. Structural trend change (CHoCH + volume). IRML risk elevated to Critical. Decisive close beyond invalidation level. |

**Confidence scoring guidelines:**
| Confidence | Meaning |
|-----------|---------|
| 90-100 | Near-unanimous signal alignment. Multiple confirmed divergences. Squeeze release + volume + trend. Rare. |
| 75-89 | Strong agreement across 3+ groups. Clear directional bias. Adequate volume confirmation. |
| 60-74 | Moderate agreement. Some conflicting signals but dominant direction clear. Act with caution. |
| 40-59 | Weak conviction. Significant conflicts. "Wait" preferred unless position management requires action. |
| 0-39 | Extreme uncertainty. Fragmented market. No action justified. |

---

### F. IRML Risk Governance

The Trader Agent respects the IRML as a **deterministic gatekeeper**. Risk boundaries are advisory constraints, not absolute blocks — but the Trader must justify any deviation.

| IRML Condition | Trader Behavior |
|---------------|----------------|
| `permission` = "Suspended" or "Emergency Stop" | Do NOT open new positions. Prefer "Wait" (flat) or protective "Hold"/"Close" of existing positions. |
| `permission` = "Restricted" or "High Caution" | Require materially stronger confluence before opening. Otherwise "Wait". |
| `exposure` = "Minimal" or "Zero" | Do not open. Size guidance is in IEPL, but the Trader should reflect this in its rationale. |
| `overall_risk` > `opportunity_score` | Prefer "Wait". Opening against a risk-opportunity inversion requires explicit justification in rationale. |
| `reward_risk.recommended_ratio` | Advisory minimum. If structural targets cannot plausibly meet this ratio, prefer "Wait". Deviation must be justified. |
| Dominant risk categories | Cited in `risk_notes`. Example: "Volatility risk at 88th percentile — widen stops, reduce sizing." |

**The Trader never overrides an Emergency Stop.** If permission is Emergency Stop, the ONLY valid actions are "Hold" (if already positioned) or "Wait" (if flat). "Close" is also acceptable for positioned trades during Emergency Stop.

---

### G. Heuristic Fallback

When no LLM API key is configured (`DEEPSEEK_API_KEY` missing, empty, or invalid), the system falls back to a deterministic 100-point confluence scoring model. The fallback produces the same `TraderDecision` output format.

**Fallback algorithm:**
```
1. Compute confluence score from ICSL (44 directional contributors)
2. Apply 7 gate multipliers (ADX, ATR, BBWP, HV, Volume, RVOL, Choppiness)
3. Gate by regime (IRCL):
   - Trending: require |confluence| > 20 for action
   - Range: require |confluence| > 40 for action, trend-following prohibited
   - Compression: action only if squeeze release detected
   - Expansion: require confluence > 15 + RVOL ≥ 1.2
   - Transitional: always "Wait"
4. Apply minimum score thresholds:
   - confluence > +60 → "Open Long"
   - confluence < −60 → "Open Short"
   - |confluence| ≤ 60 → "Wait"
5. Confidence = |confluence| (capped at 95)
6. Rationale = structured template citing regime, confluence score, dominant gate failures
7. Risk notes = IRML permission + dominant risk categories
```

The heuristic fallback is deterministic and reproducible — same inputs always produce same output. It does not match the nuance of the LLM path but provides a safe baseline when no AI is available.

**Heuristic output schema matches TraderDecision exactly:**
```json
{
  "action": "Wait",
  "confidence": 45,
  "rationale": "Confluence score +45 below action threshold of +60. Regime: Range — trend-following prohibited. Volume gate: RVOL 1.2 below institutional threshold 1.5.",
  "risk_notes": "IRML permission: Allowed. Volatility risk: Moderate (52nd percentile)."
}
```

---

## Integration

### Feeds Into
- **IEPL (Layer 9)** — TraderDecision action triggers entry/exit protocol
- **IPEL (Layer 10)** — AnalystDocument + TraderDecision recorded in trade journal for post-trade audit
- **Frontend Dashboard** — AnalystDocument displayed in AI Assistant panel; TraderDecision displayed in Decision panel

### Receives From
- **ITIL (Layer 1)** — 51 indicator DTOs (JSON array) for Analyst Agent
- **IRCL (Layer 2)** — Regime classification via MarketContext
- **ISML (Layer 3)** — S/R levels for structure analysis
- **ICSL (Layer 4)** — Confluence score + consensus for context
- **IDCL (Layer 5)** — 17 decision metrics for probabilistic context
- **ISIL (Layer 6)** — StatisticalContext for distribution/enrichment
- **IRML (Layer 7)** — Complete risk profile for Trader governance

### Cross-References
- [ITIL: §C Full Inventory](../layers/01-itil-technical-indicator.md) — 51 indicator DTOs consumed by Analyst
- [IDCL: §Trade Readiness](../layers/05-idcl-decision-context.md) — How trade_readiness relates to Trader confidence
- [IRML: §13 AI Integration](../layers/07-irmL-risk-management.md) — IRML JSON format injected into Trader payload
- [IEPL: §A Entry Protocol](../layers/09-iepl-execution-protocol.md) — How TraderDecision actions translate to execution
