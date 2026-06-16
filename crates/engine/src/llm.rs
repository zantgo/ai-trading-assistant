use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const CHAT_SYSTEM_PROMPT: &str = r#"You are a conversational, professional trading assistant specializing in cryptocurrency technical analysis. You have real-time context of current market indicators (RSI, MACD, Squeeze Momentum, ADX, ATR, EMAs, Bollinger Bands, VWAP) and the user's current position. Your role is to help the user understand market conditions and make informed manual trading decisions.

Guidelines:
- Answer user questions concisely (2-4 sentences).
- Use the provided indicator context to support your answers.
- Never give financial advice or guarantee outcomes. Always frame responses as analysis, not directives.
- When the user asks about a specific indicator, explain what its current value means in the current market context.
- Be professional yet approachable. Use plain language where possible."#;

const MASTER_ORCHESTRATOR_PROMPT: &str = r#"GROUND TRUTH DIRECTIVE: You are provided with compiled deterministic telemetry (market regime, RVOL, support/resistance lines, and the calculated 8-factor confluence point score). Treat these as absolute facts computed by the Rust analytics engine. Do not recalculate. Your job is to analyze the qualitative confluence of these facts alongside the agent thought logs, decision memory, and historical trades, and output your strategic decision.

You are the Master AI Trading Orchestrator. Your role is to synthesize individual technical indicator inputs, analyze general price action structure, and formulate a definitive trading recommendation using the 8-Factor Weighted Point-Scoring Protocol.

RULES:
- If Position is Long or Short, only recommend Hold or Close. Never recommend opening a new position when one is already held.
- If Position is None, only recommend Wait, Open Long, or Open Short.
- Evaluate the provided price sequence to understand the trend structure. Use the provided support and resistance levels to frame your analysis.
- The Macro (15m) trend is the ULTIMATE FILTER: Long entries require BULLISH macro trend, Short entries require BEARISH macro trend.
- Consider the Phase 1 indicator signals as expert sub-agent opinions. Weight them by their alignment with each other and with price action.
- Apply the 8-Factor Weighted Scoring: RSI(10pt), RSI Divergence(20pt), MACD(10pt), MACD Divergence(10pt), S/R(10pt), Macro Trend(20pt), 200EMA(10pt), Chart Patterns(10pt). Total possible: 90 points.

DIVERGENCE CONFIRMATION RULES (CRITICAL):
- RSI and MACD divergences detected by Phase 1 agents are "POTENTIAL" signals — they are NOT yet actionable.
- A potential bullish divergence becomes CONFIRMED only when a candle close decisively breaks BELOW the nearest active Support level (S₁ or S₂) with at least 0.2% tolerance.
- A potential bearish divergence becomes CONFIRMED only when a candle close decisively breaks ABOVE the nearest active Resistance level (R₁ or R₂) with at least 0.2% tolerance.
- When a divergence is merely potential (unconfirmed), treat it as secondary confluence only. Do NOT base a primary trade recommendation on an unconfirmed divergence.
- When a divergence is confirmed, it carries full scoring weight.
- CRITICAL: All indicator signals have two phases — TRIGGER (setup forming during live candle) and CONFIRMED (candle closed, setup locked). NEVER recommend trading on a trigger phase signal. Only CONFIRMED signals are actionable. The `is_completed` field distinguishes between live and completed snapshots.

MACD MOMENTUM RULES (CRITICAL):
- MACD crossover signals are filtered by the ZERO-LINE: a bullish crossover is ONLY valid when macd_line < 0 (below zero). A bearish crossover is ONLY valid when macd_line > 0 (above zero).
- EXTREME HIGH REJECTION: A bullish crossover at extreme positive values (above the configured macd_extreme_high_threshold) signals late-stage FOMO or liquidation spikes. These MUST be identified and rejected as valid entry setups.
- HISTOGRAM CONTRACTION: Evaluate whether histogram bars are expanding (momentum building) or contracting (momentum exhausting). When the histogram has contracted by more than the configured threshold (default 30%) from its peak since the last crossover, recommend early position close — this is a stronger exit signal than waiting for an opposite crossover.
- CONFLUENCE: A MACD crossover signal is significantly stronger when confirmed by RSI exiting oversold/overbought territory or an active RSI Divergence, AND when price has broken or is holding the nearest Support/Resistance level.

ADX TREND STRENGTH RULES (CRITICAL):
- ADX < 20: CONGESTION — trend-following entries PROHIBITED. Return SIDEWAYS/Wait.
- ADX 20-25: EMERGING — entries at REDUCED allocation. Validate with slope.
- ADX 25-40: STRONG — max allocation, institutional momentum.
- ADX > 40: EXTREME — block new entries. Exit immediately if slope turns negative 2 bars consecutively (Hard Hook climax).
- SLOPE VALIDATION: DI crossover valid only when ADX slope POSITIVE (accelerating). Reject flat/declining ADX crossovers as false signals.

SQUEEZE MOMENTUM RULES (CRITICAL):
- SQUEEZE ON (red dots): Positions PROHIBITED. Market is coiling energy. Wait for release.
- SQUEEZE RELEASE: Entry ONLY on first candle after squeeze release (dot changes red→green). Momentum bar must confirm direction: above zero for Long, below zero for Short.
- MIN DURATION GATE: Breakout valid only if squeeze lasted ≥5 consecutive candles. Shorter squeezes = "Premature Breakout" → reject.
- MOMENTUM DECELERATION EXIT: Holding a position and momentum bar shifts from expanding to contracting (light green→dark green for Longs, dark red→bright red for Shorts) → EXIT immediately. Leading signal before any crossover fires.
- momentum_direction phases: BullishAcceleration (enter/hold long), BullishDeceleration (exit long), BearishAcceleration (enter/hold short), BearishDeceleration (exit short), Flat (no action).

ATR VOLATILITY CONTEXT RULES:
- atr_volatility_regime dictates trade style: EXPANDING → favor trend-following / breakouts. CONTRACTING → favor mean-reversion / range-bound. STABLE → either valid.
- Dynamic Stop-Loss: Place stop at Entry ± (ATR × Multiplier). This puts the stop outside normal market noise, not at an arbitrary fixed level.
- Position Sizing: High ATR (expanding) → reduce size to maintain constant risk. Low ATR (contracting) → can increase size. Formula: Size = (Capital × Risk%) / (ATR × Multiplier).
- Breakout recommendations (Squeeze release, ADX DI crossover) require EXPANDING or STABLE volatility. Reject breakouts during CONTRACTING.

VOLUME & RVOL CONFIRMATION RULES (CRITICAL):
- RVOL = Current Volume / 20-period Average Volume.
- RVOL < 1.0: CONSOLIDATION — reject all breakout/trend signals. Fakeout territory.
- RVOL 1.0-1.5: NORMAL — standard execution.
- RVOL ≥ 1.5: INSTITUTIONAL — required to validate S/R breaks, Squeeze releases, and MACD crossovers.
- RVOL ≥ 3.0: EXHAUSTION CLIMAX — block new entries. Tighten stops. Consider immediate exit.
- Any S/R breakout without RVOL ≥ 1.5 is a "head fake" — reject as invalid.

EMA STACKING RULES (CRITICAL):
- ema_stack_state determines market structure: bullish (Price > EMA10 > EMA50 > EMA100 > EMA200), bearish (Price < EMA10 < EMA50 < EMA100 < EMA200), or tangled (no sequential order).
- Bullish Stack: ONLY Long positions permitted. Buy pullbacks to EMA10-EMA50 value zone.
- Bearish Stack: ONLY Short positions permitted. Short rallies to EMA10-EMA50 zone.
- Tangled Stack: ALL trend-following entries PAUSED. EMAs are crossing/falttening. Range-bound only.
- EMA 200 is the ULTIMATE MACRO FILTER: Long rejected if price < EMA200. Short rejected if price > EMA200.
- Structural Invalidation: Close below EMA100 or EMA200 on Long → exit immediately. Close above on Short → exit immediately.

VWAP INSTITUTIONAL RULES:
- vwap_bias: premium (>0.1% above VWAP), discount (>0.1% below), equilibrium (within ±0.1%).
- Premium → bullish intraday bias. Institutions selling into strength. Discount → bearish bias.
- Institutional Pullback: Bullish EMA stack + price > VWAP → buy VWAP touch/wick + close above EMA10.
- Bearish EMA stack + price < VWAP → short VWAP rally/touch + close below EMA10.
- Ranging regime (tangled EMAs, ADX < 20) → VWAP is the TP₁ mean-reversion target.

SUPPORT & RESISTANCE ROLE-REVERSAL RULES:
- S/R levels are horizontal liquidity zones from pivot highs/lows over scan_range_candles.
- Resistance broken above flip_tolerance (default 0.3%) → flips to Support. Support broken below → flips to Resistance.
- Flip events tracked with timestamps and counts. Merge protocol preserves flip memory across cycles.
- Price within 0.5% of active Support → bullish confluence. Within 0.5% of active Resistance → bearish confluence.
- S/R breakout entries require RVOL ≥ 1.5 for institutional volume confirmation. Low-volume breaks = head fakes.

FIBONACCI GOLDEN POCKET RULES:
- Fibonacci levels computed from the most recent major swing leg detected via pivot scanning.
- Golden Pocket (61.8%–66.0% retracement) is the highest-probability institutional entry zone.
- Bullish: price pulls back from swing high into GP → enter Long when RSI/Squeeze confirm reversal.
- Bearish: price rallies from swing low into GP → enter Short when confirmation indicators align.
- 1.618 Extension = primary TP₁/TP₂ target. 2.618 Extension = ultimate TP₃ target (parabolic climax).
- Extensions carry more weight during EXPANDING ATR regimes.

BBWP VOLATILITY PERCENTILE RULES:
- BBWP < 10% (COMPRESSION): Extreme consolidation. Stored energy preparing for explosive breakout. Boost breakout signals (Squeeze release, pattern breakouts).
- BBWP > 90% (EXHAUSTION CLIMAX): Parabolic overextension. Block new trend entries. Tighten stops on active positions. Trend flatlining imminent.
- BBWP 10-90%: Normal volatility. Standard entry rules.
- Squeeze release entries valid only if BBWP recently registered < 10% (coiled energy confirmation).

CHART PATTERN RULES:
- Patterns detected mathematically from pivot linear regression.
- FallingWedge / BullishTriangle / AscendingChannel → bullish bias.
- RisingWedge / BearishTriangle / DescendingChannel → bearish bias.
- Breakout requires candle close beyond regression line + RVOL ≥ 1.5 for confirmation.
- Without volume confirmation, breakout is a potential fakeout.

- When RECENT TRADE EXECUTION HISTORY is provided below, review past mistakes and adjust your recommendation to avoid repeating identical errors (e.g., if a prior trade failed due to entering before candle close, explicitly recommend waiting for candle confirmation).
- When trade history is NOT provided, base your decision solely on current market data.
- Output strictly JSON, no markdown fences, no conversational preambles.

OUTPUT SCHEMA:
{
  "general_trend": "UPWARD" | "DOWNWARD" | "SIDEWAYS",
  "support_and_resistance": {
    "structural_analysis": "A concise explanation of how the provided support/resistance levels constrain or influence the current price action. Note any recent S/R role-reversals. If a divergence is active, explicitly note whether the S/R boundary has been broken for confirmation. 1-2 sentences."
  },
  "indicator_synthesis": {
    "summary_count": "e.g., '5 Bullish, 1 Bearish, 2 Sideways'",
    "evaluation": "How the indicators converge or diverge from raw price action trend. Explain if the majority of signals support or conflict with the trend direction. Reference the weighted factor score. If divergences are present, classify them as Potential or Confirmed based on S/R break status. 2-3 sentences."
  },
  "position_recommendation": {
    "action": "Hold" | "Close" | "Wait" | "Open Long" | "Open Short",
    "rationale": "Provide a highly clear, professional, conversational operational reasoning guiding the user on their next step given their position entry price, current price action, support/resistance constraints, macro trend filter, and Fibonacci/extension levels. Explicitly state whether any divergence referenced is Potential or Confirmed. If trade history was provided, reference specific past mistakes you are avoiding. 2-4 sentences."
  }
}"#;

const JOURNAL_AGENT_PROMPT: &str = r#"You are a disciplined Post-Trade Performance Auditor. Your task is to critically evaluate a completed trade and produce a structured retrospective.

ANALYSIS GUIDELINES:
- Evaluate the quality of execution, not just the outcome. A losing trade with excellent discipline can score high; a winning trade entered recklessly should score low.
- Common execution mistakes to look for: FOMO entries, trading against the macro trend, failing to wait for candle confirmation, improper scale-in pacing, moving stop-losses, over-sizing.
- Common successes: patience, strict level alignment, disciplined scale-ins, proper risk management, letting winners run.
- Generate a concise 3-4 sentence retrospective (final_analysis) summarizing what went right or wrong.
- Assign an execution_score from 0.0 (abysmal execution, total disregard for process) to 10.0 (exemplary discipline and perfect process adherence).

OUTPUT strictly JSON, no markdown fences, no conversational preambles:
{
  "final_analysis": "3-4 sentence retrospective covering key mistakes or successes",
  "execution_score": 0.0
}"#;

const MULTI_TF_MASTER_ORCHESTRATOR_PROMPT: &str = r#"GROUND TRUTH DIRECTIVE: You are provided with compiled deterministic telemetry (market regime, RVOL, support/resistance lines, and the calculated 8-factor confluence point score) from the Rust analytics engine. Treat these as absolute mathematical facts. Do not recalculate. Your job is to analyze the qualitative confluence of these facts alongside the agent thought logs, decision memory, and historical trades, and output your strategic decision.

You are the Master AI Multi-Timeframe Trading Orchestrator. Your role is to analyze a structured dataset representing market data across five independent timescales: Short-Term (15s), Mid-Term (1m), Long-Term (5m), Macro (15m), and SuperMacro (1h).

DIAGNOSTIC PROCESS:
1. Trend Confluence: Examine the direction and indicators of each timeframe. Note if they are aligned or in conflict.
   - SuperMacro (1h): Defines the maximum structural trend limit and major macro value areas.
   - Macro (15m): The primary directional trend filter used to determine Bullish or Bearish trading bias.
   - Long-Term (5m): The execution timeframe for order placement, indicator calculations, and point-scoring.
   - Mid-Term (1m): Identifies intermediate swing context and local momentum crossovers.
   - Short-Term (15s): Highlights short-term momentum and precision entry/exit execution timings.
2. Signal Consolidation: Synthesize all five levels to determine overall market bias.
3. Decision Matching: Apply standard risk mitigation parameters. The Macro (15m) trend defines the trading bias — NEVER recommend a position opposing the Macro trend.
4. Weighted 8-Factor Scoring: Evaluate using the weighted scoring protocol (RSI=10, RSI Div=20, MACD=10, MACD Div=10, S/R=10, Trend=20, 200EMA=10, Patterns=10, total max=90 points).

DIVERGENCE CONFIRMATION RULES (CRITICAL):
- RSI and MACD divergences detected by Phase 1 agents are "POTENTIAL" signals — they are NOT yet actionable.
- A potential bullish divergence becomes CONFIRMED only when a candle close decisively breaks BELOW the nearest active Support level (S₁ or S₂) with at least 0.2% tolerance.
- A potential bearish divergence becomes CONFIRMED only when a candle close decisively breaks ABOVE the nearest active Resistance level (R₁ or R₂) with at least 0.2% tolerance.
- When a divergence is merely potential (unconfirmed), treat it as secondary confluence only. Do NOT base a primary trade recommendation on an unconfirmed divergence.
- Confirmation is timeframe-adaptive: the break must occur on the same timeframe where the divergence was detected.
- When a divergence is confirmed, it carries full scoring weight.
- TRIGGER vs CONFIRMED: All signals have two phases — TRIGGER (live candle, repaintable) and CONFIRMED (candle closed, locked). Only CONFIRMED signals are actionable for trade decisions.

MACD MOMENTUM RULES (CRITICAL):
- MACD crossover signals are filtered by the ZERO-LINE: bullish crossover ONLY valid when macd_line < 0. Bearish crossover ONLY valid when macd_line > 0.
- EXTREME HIGH REJECTION: Bullish crossovers at extreme positive values signal late-stage FOMO / liquidation spikes. Identify and reject them.
- HISTOGRAM CONTRACTION: Expanding histogram = momentum building. Contracting histogram (≥30% from peak) = momentum exhausting → recommend early position close before an opposite crossover prints.
- CONFLUENCE: Crossovers confirmed by RSI exiting oversold/overbought OR active RSI Divergence, plus price breaking/holding the nearest S/R level → significantly higher confidence.

ADX TREND STRENGTH RULES (CRITICAL):
- ADX < 20 (CONGESTION): Trend-following entries PROHIBITED. Return SIDEWAYS/Wait.
- ADX 20-25 (EMERGING): Entries allowed at reduced allocation. Caution.
- ADX 25-40 (STRONG): Fully favorable. Maximum allocation.
- ADX > 40 (EXTREME): Block new entries. If holding, evaluate immediate exit when ADX slope turns negative for 2 consecutive bars (Hard Hook).
- SLOPE VALIDATION: DI crossover only valid when ADX slope POSITIVE (accelerating). Flat/declining ADX = false signal.
- When ADX above 40 and slope negative for 2 bars → trigger EXHAUSTION EXIT, do not wait for DI crossover.

SQUEEZE MOMENTUM RULES (CRITICAL):
- SQUEEZE ON (red dots): No positions. Wait for release.
- SQUEEZE RELEASE: Entry only on release candle (red→green dot). Momentum must confirm direction.
- MIN DURATION GATE: ≥5 consecutive squeeze-on candles required. Shorter = reject.
- MOMENTUM DECELERATION: bar shifts expanding→contracting = EXIT immediately. Leading exit signal.
- Phases: BullishAcceleration, BullishDeceleration, BearishAcceleration, BearishDeceleration, Flat.

ATR VOLATILITY CONTEXT RULES:
- atr_volatility_regime: EXPANDING → favor breakouts. CONTRACTING → favor mean-reversion. STABLE → either.
- Dynamic SL: Entry ± (ATR × Multiplier). Size: (Capital × Risk%) / (ATR × Multiplier).
- Breakout recommendations require EXPANDING or STABLE. Reject during CONTRACTING.

VOLUME & RVOL CONFIRMATION RULES (CRITICAL):
- RVOL = Current Volume / 20-period Average Volume.
- RVOL < 1.0: CONSOLIDATION — reject ALL breakout/trend signals. Low participation = fakeout territory.
- RVOL 1.0-1.5: NORMAL — standard execution at normal allocation.
- RVOL ≥ 1.5: INSTITUTIONAL — required to validate S/R breaks, Squeeze releases, and MACD crossovers beneath zero.
- RVOL ≥ 3.0: EXHAUSTION CLIMAX — block new entries. Consider immediate exit. Trend climax approaching.
- Any S/R breakout candle with RVOL < 1.5 is a "head fake" and must be rejected as invalid.

EMA STACKING RULES (CRITICAL):
- ema_stack_state: bullish (Price > EMA10 > EMA50 > EMA100 > EMA200), bearish (inverse), tangled.
- Bullish Stack → ONLY Long. Bearish Stack → ONLY Short. Tangled → ALL trend entries PAUSED.
- EMA 200 is ULTIMATE FILTER: Long rejected below, Short rejected above.
- Close below EMA100/200 on Long → exit. Close above on Short → exit.

VWAP INSTITUTIONAL RULES:
- vwap_bias: premium (>0.1% above VWAP) / discount (>0.1% below) / equilibrium.
- Bullish stack + pullback to VWAP touch + close above EMA10 → LONG entry.
- Bearish stack + rally to VWAP touch + close below EMA10 → SHORT entry.
- Ranging regime → VWAP is TP₁ mean-reversion target.

SUPPORT & RESISTANCE ROLE-REVERSAL RULES:
- S/R levels are horizontal liquidity zones from pivot highs/lows.
- Resistance broken above flip_tolerance → flips to Support. Support broken below → flips to Resistance.
- Flip memory preserved via merge protocol. Price within 0.5% of active level → confluence.
- S/R breakouts require RVOL ≥ 1.5. Low-volume breaks = head fakes.

FIBONACCI GOLDEN POCKET RULES:
- Golden Pocket (61.8%-66.0%) is highest-probability institutional entry zone.
- Bullish: pullback into GP + RSI/Squeeze confirmation → Long. Bearish: rally into GP + confirmation → Short.
- 1.618 Ext = TP₁/TP₂, 2.618 Ext = TP₃ (parabolic climax). Carry more weight during EXPANDING ATR.

BBWP VOLATILITY PERCENTILE RULES:
- BBWP < 10%: COMPRESSION — boost breakout signals. BBWP > 90%: EXHAUSTION — block entries, tighten stops.
- Squeeze releases valid only if BBWP recently dipped below 10%.

CHART PATTERN RULES:
- FallingWedge / BullishTriangle / AscendingChannel → bullish. RisingWedge / BearishTriangle / DescendingChannel → bearish.
- Breakout requires close beyond regression line + RVOL ≥ 1.5.

RULES:
- If Position is Long or Short, only recommend Hold or Close.
- If Position is None, only recommend Wait, Open Long, or Open Short.
- Phase 1 results are labeled by timeframe prefix (short-, mid-, long-, macro-, supermacro-) before the indicator name.
- Macro (15m) and SuperMacro (1h) signals carry the most weight for structural direction.
- Short-term signals identify local entry/exit timing only.
- When timeframes conflict, default to the majority view with a preference for the longer timeframe.
- The Macro (15m) trend is the ULTIMATE FILTER: a Long entry requires Macro trend = BULLISH; a Short entry requires Macro trend = BEARISH.
- When RECENT TRADE EXECUTION HISTORY is provided below, review past mistakes and adjust your recommendation to avoid repeating identical errors (e.g., if a prior trade failed due to entering before candle close, explicitly recommend waiting for candle confirmation).
- When trade history is NOT provided, base your decision solely on current market data.
- Output strictly JSON, no markdown fences.

OUTPUT SCHEMA:
{
  "general_trend": "UPWARD" | "DOWNWARD" | "SIDEWAYS",
  "support_and_resistance": {
    "structural_analysis": "Brief description of how price levels constrain action across all five timeframes. If divergences are active, explicitly note whether S/R boundaries have been broken for confirmation. 1-2 sentences."
  },
  "indicator_synthesis": {
    "summary_count": "e.g., 'Short: 3/7 Bullish, Mid: 4/7 Bearish, Long: 5/7 Bullish, Macro: 2/7 Bearish, SuperMacro: 3/7 Bearish'",
    "evaluation": "How indicators from all five timeframes converge or diverge. Mention which timeframe dominates the consensus. Reference the 8-factor weighted point score. If divergences are present, classify them as Potential or Confirmed. 2-3 sentences."
  },
  "position_recommendation": {
    "action": "Hold" | "Close" | "Wait" | "Open Long" | "Open Short",
    "rationale": "Clear operational reasoning synthesizing multi-timeframe signals, macro trend filter, divergence confirmation status, and weighted factor score into an actionable trade decision. Reference specific Fibonacci Golden Pocket levels or support/resistance levels if relevant. 2-4 sentences."
  }
}"#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    #[allow(dead_code)]
    total_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualIndicatorResult {
    pub indicator_name: String,
    pub signal: String,
    pub reason: String,
    #[serde(default)]
    pub divergence_status: Option<String>,
    #[serde(default)]
    pub divergence_type: Option<String>,
    #[serde(default)]
    pub is_confirmed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalResult {
    pub final_analysis: String,
    pub execution_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupportResistance {
    #[serde(default)]
    pub detected_support_levels: Vec<String>,
    #[serde(default)]
    pub detected_resistance_levels: Vec<String>,
    #[serde(default)]
    pub structural_analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSynthesis {
    pub summary_count: String,
    pub evaluation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRecommendation {
    pub action: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterOrchestratorResult {
    pub general_trend: String,
    #[serde(default)]
    pub support_and_resistance: SupportResistance,
    pub indicator_synthesis: IndicatorSynthesis,
    pub position_recommendation: PositionRecommendation,
    #[serde(default)]
    pub eight_factor_score: i32,
    #[serde(default)]
    pub allocation_pct: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentEvaluationResult<T> {
    pub thought: String,
    pub data: T,
}

// ─── Sub-Agent Output Schemas ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrendAgentData {
    pub directional_bias: String,
    pub confidence_score: i32,
    pub ema_slope_alignment: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolatilityAgentData {
    pub regime_classification: String,
    pub volatility_score: i32,
    pub suggest_stop_multiplier: f64,
    pub is_actionable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructureAgentData {
    pub support_proximity_pct: f64,
    pub resistance_proximity_pct: f64,
    pub golden_pocket_status: String,
    pub structural_score: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskAgentData {
    pub suggested_sizing_pct: f64,
    pub leverage: i32,
    pub exposure_score: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionAgentData {
    pub recommended_action: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MasterOrchestrationData {
    pub market_regime: String,
    pub eight_factor_score: i32,
    pub decision: String,
    pub allocation_pct: f64,
    pub rationale: String,
}

// ─── Sub-Agent System Prompts ──────────────────────────────────────

pub const TREND_AGENT_PROMPT: &str = r#"You are the Trend Agent. Your task is to evaluate multi-timeframe EMA stacking states, price-to-EMA200 distance, and macro trend biases (15m/1h).
Calculate trend direction and trend acceleration. Output strictly a JSON object containing "thought" and "data" with fields "directional_bias", "confidence_score", and "ema_slope_alignment".
Use the following enum values only: directional_bias = BULLISH | BEARISH | NEUTRAL; confidence_score = 0 to 100; ema_slope_alignment = "aligned" | "diverging" | "flat". Output strictly JSON, no markdown fences."#;

pub const VOLATILITY_AGENT_PROMPT: &str = r#"You are the Volatility Agent. Evaluate BBWP percentile, ATR slope, Squeeze Momentum duration, and release trigger status.
Determine the current volatility regime (Expanding, Contracting, Stable, Compression) and suggest stops. Output strictly a JSON object with "thought" and "data" containing "regime_classification", "volatility_score", "suggest_stop_multiplier", and "is_actionable".
Use the following enum values only: regime_classification = COMPRESSION | EXPANSION | TRENDING | RANGE; volatility_score = 0 to 100. Output strictly JSON, no markdown fences."#;

pub const STRUCTURE_AGENT_PROMPT: &str = r#"You are the Structure Agent. Evaluate pivot highs/lows, support/resistance lines, Fibonacci Golden Pocket bounds, and linear regression channels.
Track level breaks and manage S/R role-reversals. Output strictly a JSON object with "thought" and "data" containing "support_proximity_pct", "resistance_proximity_pct", "golden_pocket_status", and "structural_score".
Use the following enum values only: golden_pocket_status = "above" | "below" | "inside"; structural_score = 0 to 100. Output strictly JSON, no markdown fences."#;

pub const RISK_AGENT_PROMPT: &str = r#"You are the Risk Agent. Evaluate total portfolio cash, open risk, suggested leverage, and correlation exposure across pairs.
Normalize position sizing and calculate suggested capital allocation. Output strictly a JSON object with "thought" and "data" containing "suggested_sizing_pct", "leverage", and "exposure_score".
Use the following ranges: suggested_sizing_pct = 0.0 to 100.0; leverage = 1 to 50; exposure_score = 0 to 100. Output strictly JSON, no markdown fences."#;

pub const POSITION_AGENT_PROMPT: &str = r#"You are the Position Management Agent. Evaluate current active position state (entry price, average entry price, unrealized P&L, stop-loss, and take-profit targets).
Recommend position modifications (Hold, Close, Scale-In, Reduce, Invalidate). Output strictly a JSON object with "thought" and "data" containing "recommended_action" and "rationale".
Use the following enum values only: recommended_action = HOLD | CLOSE | SCALE | REDUCE. Output strictly JSON, no markdown fences."#;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PairTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl PairTokenUsage {
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokenTracker {
    pub per_pair: HashMap<String, PairTokenUsage>,
    pub global: PairTokenUsage,
}

impl TokenTracker {
    pub fn accumulate(&mut self, pair_key: Option<&str>, input: u64, output: u64) {
        self.global.input_tokens += input;
        self.global.output_tokens += output;
        if let Some(key) = pair_key {
            let entry = self.per_pair.entry(key.to_string()).or_insert(PairTokenUsage { input_tokens: 0, output_tokens: 0 });
            entry.input_tokens += input;
            entry.output_tokens += output;
        }
    }

    pub fn get_per_pair(&self, pair_key: &str) -> PairTokenUsage {
        self.per_pair.get(pair_key).cloned().unwrap_or(PairTokenUsage { input_tokens: 0, output_tokens: 0 })
    }

    pub fn reset(&mut self) {
        self.per_pair.clear();
        self.global = PairTokenUsage { input_tokens: 0, output_tokens: 0 };
    }
}

pub struct LlmClient {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) indicators_guide: String,
    pub(crate) token_tracker: Arc<Mutex<TokenTracker>>,
}

impl LlmClient {
    pub fn from_env() -> (Self, bool) {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map(|k| k.trim().to_string())
            .unwrap_or_default();

        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let model = std::env::var("DEEPSEEK_MODEL")
            .unwrap_or_else(|_| "deepseek-chat".into());

        let indicators_guide = std::fs::read_to_string("docs/indicators-guide.md")
            .unwrap_or_else(|_| String::new());

        let key_present = !api_key.is_empty();

        (LlmClient {
            base_url,
            api_key,
            model,
            indicators_guide,
            token_tracker: Arc::new(Mutex::new(TokenTracker::default())),
        }, key_present)
    }

    pub fn from_dotenv() -> Result<Self, String> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| "DEEPSEEK_API_KEY not found in .env file. Create a .env file at the project root with: DEEPSEEK_API_KEY=sk-...".to_string())?;

        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            return Err("DEEPSEEK_API_KEY is empty in .env file. Set your DeepSeek API key.".to_string());
        }
        if !api_key.starts_with("sk-") {
            return Err(format!(
                "DEEPSEEK_API_KEY does not look like a valid DeepSeek key (should start with 'sk-'). Got: {}...",
                &api_key[..api_key.len().min(10)]
            ));
        }

        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let model = std::env::var("DEEPSEEK_MODEL")
            .unwrap_or_else(|_| "deepseek-chat".into());

        let indicators_guide = std::fs::read_to_string("docs/indicators-guide.md")
            .unwrap_or_else(|_| "No indicators guide found.".to_string());

        Ok(LlmClient {
            base_url,
            api_key,
            model,
            indicators_guide,
            token_tracker: Arc::new(Mutex::new(TokenTracker::default())),
        })
    }

    pub fn set_api_key(&mut self, key: String) {
        self.api_key = key;
    }

    pub fn set_indicators_guide(&mut self, guide: String) {
        self.indicators_guide = guide;
    }

    pub fn get_token_tracker(&self) -> Arc<Mutex<TokenTracker>> {
        self.token_tracker.clone()
    }

    pub fn reset_token_usage(&self) {
        if let Ok(mut tracker) = self.token_tracker.lock() {
            tracker.reset();
        }
    }

    pub fn get_token_usage_for_pair(&self, pair_key: &str) -> PairTokenUsage {
        self.token_tracker.lock()
            .map(|t| t.get_per_pair(pair_key))
            .unwrap_or(PairTokenUsage { input_tokens: 0, output_tokens: 0 })
    }

    pub async fn validate_key(&self) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("No API key configured".to_string());
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to reach DeepSeek API: {}", e))?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable>".into());

        Err(format!(
            "DeepSeek API rejected the key (HTTP {}). Verify your DEEPSEEK_API_KEY in .env\nResponse: {}",
            status, body
        ))
    }

    fn track_usage(&self, pair_key: Option<&str>, usage: &Option<Usage>) {
        if let Some(u) = usage {
            if let Ok(mut tracker) = self.token_tracker.lock() {
                tracker.accumulate(pair_key, u.prompt_tokens, u.completion_tokens);
            }
        }
    }

    pub async fn run_indicator_agent(
        &self,
        indicator_name: &str,
        indicator_section: &str,
        user_context: &str,
        pair_key: Option<&str>,
    ) -> Result<IndividualIndicatorResult, String> {
        let system_prompt = format!(
            r#"You are a highly analytical trading sub-agent specializing strictly in evaluating the technical indicator: {}.
Refer to the provided rules in the indicator reference for interpretation thresholds.

INDICATOR REFERENCE RULES:
{}

CONTEXT:
Analyze the provided current market data. You must output a clean JSON structure conforming to the following schema:

{{
  "indicator_name": "{}",
  "signal": "BULLISH" | "BEARISH" | "SIDEWAYS",
  "reason": "Provide a brief 1-2 sentence explanation of your decision using the rules and the provided numerical parameters."
}}

RULES:
- Respond with JSON ONLY. Do not write markdown fences, preamble, or commentary.
- Be completely deterministic. Use the numerical parameters and apply them strictly against the criteria in the reference docs."#,
            indicator_name, indicator_section, indicator_name
        );

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: system_prompt },
                ChatMessage { role: "user".into(), content: user_context.to_string() },
            ],
            temperature: 0.1,
            response_format: Some(ResponseFormat { format_type: "json_object".into() }),
            max_tokens: 512,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("LLM API request failed for {}: {}", indicator_name, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("LLM API returned {} for {}: {}", status, indicator_name, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response for {}: {}", indicator_name, e))?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| format!("LLM response for {} had no choices", indicator_name))?
            .message
            .content
            .clone();

        let result: IndividualIndicatorResult = serde_json::from_str(&content)
            .map_err(|e| format!(
                "Failed to parse LLM JSON output for {}: {}. Raw content: {}",
                indicator_name, e, content
            ))?;

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    pub async fn run_master_orchestrator(
        &self,
        position: &str,
        entry_price: &str,
        prices: &[f64],
        symbol: &str,
        phase_one_results_json: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        journal_context: Option<&str>,
        pair_key: Option<&str>,
    ) -> Result<MasterOrchestratorResult, String> {
        let prices_str = serde_json::to_string(prices)
            .map_err(|e| format!("Failed to serialize prices: {}", e))?;

        let supports_str = serde_json::to_string(support_levels)
            .unwrap_or_else(|_| "[]".into());
        let resistances_str = serde_json::to_string(resistance_levels)
            .unwrap_or_else(|_| "[]".into());

        let entry_info = if entry_price.is_empty() || entry_price == "0" || entry_price == "0.00" {
            "None (no open position)".to_string()
        } else {
            format!("${}", entry_price)
        };

        let journal_section = match journal_context {
            Some(ctx) if !ctx.is_empty() => format!("\n\n{}", ctx),
            _ => String::new(),
        };

        let user_message = format!(
            "CURRENT MARKET ASSET: {}\n\
             USER'S OPEN POSITION: {}\n\
             USER'S ENTRY PRICE: {}\n\
             RAW PRICE HISTORY (last {} closes): {}\n\
             COMPUTED SUPPORT LEVELS: {}\n\
             COMPUTED RESISTANCE LEVELS: {}\n\
             PHASE 1 INDIVIDUAL INDICATOR AGENT SIGNALS:\n{}{}",
            symbol, position, entry_info, prices.len(), prices_str,
            supports_str, resistances_str,
            phase_one_results_json,
            journal_section,
        );

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: MASTER_ORCHESTRATOR_PROMPT.into() },
                ChatMessage { role: "user".into(), content: user_message },
            ],
            temperature: 0.3,
            response_format: Some(ResponseFormat { format_type: "json_object".into() }),
            max_tokens: 1024,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Master orchestrator request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("Master orchestrator API returned {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse master orchestrator response: {}", e))?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("Master orchestrator response had no choices")?
            .message
            .content
            .clone();

        let mut result: MasterOrchestratorResult = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse master orchestrator JSON: {}. Raw content: {}", e, content))?;

        result.support_and_resistance = SupportResistance {
            detected_support_levels: support_levels.to_vec(),
            detected_resistance_levels: resistance_levels.to_vec(),
            structural_analysis: result.support_and_resistance.structural_analysis,
        };

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    pub async fn run_multi_timeframe_orchestrator(
        &self,
        position: &str,
        entry_price: &str,
        symbol: &str,
        phase_one_results_json: &str,
        support_levels: &[String],
        resistance_levels: &[String],
        journal_context: Option<&str>,
        pair_key: Option<&str>,
    ) -> Result<MasterOrchestratorResult, String> {
        let supports_str = serde_json::to_string(support_levels)
            .unwrap_or_else(|_| "[]".into());
        let resistances_str = serde_json::to_string(resistance_levels)
            .unwrap_or_else(|_| "[]".into());

        let entry_info = if entry_price.is_empty() || entry_price == "0" || entry_price == "0.00" {
            "None (no open position)".to_string()
        } else {
            format!("${}", entry_price)
        };

        let journal_section = match journal_context {
            Some(ctx) if !ctx.is_empty() => format!("\n\n{}", ctx),
            _ => String::new(),
        };

        let user_message = format!(
            "CURRENT MARKET ASSET: {}\n\
             USER'S OPEN POSITION: {}\n\
             USER'S ENTRY PRICE: {}\n\
             COMPUTED SUPPORT LEVELS: {}\n\
             COMPUTED RESISTANCE LEVELS: {}\n\
             PHASE 1 MULTI-TIMEFRAME INDICATOR AGENT SIGNALS (short/mid/long prefix):\n{}{}",
            symbol, position, entry_info,
            supports_str, resistances_str,
            phase_one_results_json,
            journal_section,
        );

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: MULTI_TF_MASTER_ORCHESTRATOR_PROMPT.into() },
                ChatMessage { role: "user".into(), content: user_message },
            ],
            temperature: 0.3,
            response_format: Some(ResponseFormat { format_type: "json_object".into() }),
            max_tokens: 1024,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Multi-TF orchestrator request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("Multi-TF orchestrator API returned {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse multi-TF orchestrator response: {}", e))?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("Multi-TF orchestrator response had no choices")?
            .message
            .content
            .clone();

        let mut result: MasterOrchestratorResult = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse multi-TF orchestrator JSON: {}. Raw content: {}", e, content))?;

        result.support_and_resistance = SupportResistance {
            detected_support_levels: support_levels.to_vec(),
            detected_resistance_levels: resistance_levels.to_vec(),
            structural_analysis: result.support_and_resistance.structural_analysis,
        };

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    pub async fn run_journal_agent(
        &self,
        trade_context: &str,
        pair_key: Option<&str>,
    ) -> Result<JournalResult, String> {
        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: JOURNAL_AGENT_PROMPT.into() },
                ChatMessage { role: "user".into(), content: trade_context.to_string() },
            ],
            temperature: 0.2,
            response_format: Some(ResponseFormat { format_type: "json_object".into() }),
            max_tokens: 512,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Journal agent request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("Journal agent API returned {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse journal agent response: {}", e))?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("Journal agent response had no choices")?
            .message
            .content
            .clone();

        let mut result: JournalResult = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse journal agent JSON: {}. Raw content: {}", e, content))?;

        result.execution_score = result.execution_score.clamp(0.0, 10.0);

        self.track_usage(pair_key, &usage);

        Ok(result)
    }

    pub fn get_guide_section(&self, indicator_name: &str) -> String {
        let section_number = match indicator_name {
            "RSI" => "1.",
            "MACD" => "2.",
            "SQUEEZE" => "3.",
            "ADX" => "4.",
            "BOLLINGER_ATR" => "5.",
            "VOLUME_EMA" => "6.",
            "VWAP" => "7.",
            _ => return "No rules found.".to_string(),
        };

        let lines: Vec<&str> = self.indicators_guide.lines().collect();
        let mut start_idx = None;
        let mut end_idx = None;

        for (i, line) in lines.iter().enumerate() {
            if line.starts_with(&format!("## {}", section_number)) {
                start_idx = Some(i);
            }
            if start_idx.is_some() && end_idx.is_none() && i > start_idx.unwrap() {
                if line.starts_with("## ") && !line.starts_with(&format!("## {}", section_number)) {
                    end_idx = Some(i);
                }
                if line.starts_with("---") && i > start_idx.unwrap() + 5 {
                    end_idx = Some(i);
                }
            }
        }

        match (start_idx, end_idx) {
            (Some(s), Some(e)) => lines[s..e].join("\n"),
            (Some(s), None) => lines[s..].join("\n"),
            _ => "Section not found in indicators guide.".to_string(),
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>, pair_key: Option<&str>) -> Result<String, String> {
        let mut full_messages: Vec<ChatMessage> = vec![
            ChatMessage {
                role: "system".into(),
                content: CHAT_SYSTEM_PROMPT.into(),
            },
        ];
        full_messages.extend(messages);

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: full_messages,
            temperature: 0.7,
            response_format: None,
            max_tokens: 1024,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("LLM API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("LLM API returned {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        let usage = chat_response.usage;

        let content = chat_response
            .choices
            .first()
            .ok_or("LLM response had no choices")?
            .message
            .content
            .clone();

        self.track_usage(pair_key, &usage);

        Ok(content)
    }

    pub async fn run_domain_agent<T>(
        &self,
        agent_name: &str,
        system_prompt: &str,
        user_context: &str,
        pair_key: Option<&str>,
    ) -> Result<AgentEvaluationResult<T>, String>
    where
        T: for<'de> serde::Deserialize<'de> + serde::Serialize + Clone,
    {
        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: system_prompt.to_string() },
                ChatMessage { role: "user".into(), content: user_context.to_string() },
            ],
            temperature: 0.1,
            response_format: Some(ResponseFormat { format_type: "json_object".into() }),
            max_tokens: 1024,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("LLM API request failed for {}: {}", agent_name, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(format!("LLM API returned {} for {}: {}", status, agent_name, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response for {}: {}", agent_name, e))?;

        let usage = chat_response.usage;
        self.track_usage(pair_key, &usage);

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| format!("LLM response for {} had no choices", agent_name))?
            .message
            .content
            .clone();

        let parsed_result: AgentEvaluationResult<T> = serde_json::from_str(&content)
            .map_err(|e| format!(
                "Failed to parse JSON output for {}: {}. Raw content: {}",
                agent_name, e, content
            ))?;

        Ok(parsed_result)
    }
}

// ─── Multi-Agent Pipeline Results ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentResults {
    pub trend: AgentEvaluationResult<TrendAgentData>,
    pub volatility: AgentEvaluationResult<VolatilityAgentData>,
    pub structure: AgentEvaluationResult<StructureAgentData>,
    pub risk: AgentEvaluationResult<RiskAgentData>,
    pub position: AgentEvaluationResult<PositionAgentData>,
}

impl MultiAgentResults {
    pub fn to_legacy_signals(&self) -> Vec<IndividualIndicatorResult> {
        let trend_bias = &self.trend.data.directional_bias;
        let trend_thought = &self.trend.thought;
        let vol_thought = &self.volatility.thought;
        let vol_regime = &self.volatility.data.regime_classification;

        let squeeze_signal = match vol_regime.as_str() {
            "COMPRESSION" => "SIDEWAYS".to_string(),
            _ => trend_bias.clone(),
        };

        let adx_signal = match vol_regime.as_str() {
            "RANGE" => "SIDEWAYS".to_string(),
            _ => trend_bias.clone(),
        };

        vec![
            IndividualIndicatorResult {
                indicator_name: "short-RSI".to_string(),
                signal: trend_bias.clone(),
                reason: trend_thought.clone(),
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "mid-MACD".to_string(),
                signal: trend_bias.clone(),
                reason: trend_thought.clone(),
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "long-SQUEEZE".to_string(),
                signal: squeeze_signal,
                reason: vol_thought.clone(),
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "macro-ADX".to_string(),
                signal: adx_signal,
                reason: vol_thought.clone(),
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "supermacro-VWAP".to_string(),
                signal: trend_bias.clone(),
                reason: trend_thought.clone(),
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
        ]
    }
}
