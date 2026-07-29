// Decision Rank — pure helper for the unified Decision-tab hero.
//
// `computeDecisionRank()` consumes the same wire-format payloads the Advisory
// panel already reads (AdvisoryMatrix + DecisionContext + OpportunityMatrix)
// and produces a single `DecisionRank` view containing:
//
//   - three normalized probabilities (long / short / hold) summing to 100
//   - the top action + a hero label that is always consistent with the
//     `trade_readiness` gate (so the UI never says "LONG — READY" while the
//     gate is "STAND ASIDE")
//   - a bulleted rationale list explaining the decision
//
// This is the **frontend-only consolidation layer** that resolves the
// long-standing UX bug where three competing badges (trade_readiness,
// directional_guidance, market_stance) appeared at the top of the Decision
// tab and could visually contradict each other. The three fields are
// orthogonal axes (gate vs bias vs environment), not redundant, so the bug
// is perceptual: this helper merges them into a single coherent verdict.

import type {
    AdvisoryMatrix,
    AnalysisMatrix,
    DecisionContext,
    OpportunityMatrix,
} from '../types';

export type DecisionAction = 'LONG' | 'SHORT' | 'HOLD';
export type DecisionState = 'READY' | 'FORMING' | 'WATCH' | 'STAND_ASIDE';

export interface RankSide {
    probability: number;          // 0..100, integer, normalised to sum 100
    reasons: string[];
}

export interface DecisionRank {
    long: RankSide;
    short: RankSide;
    hold: RankSide;
    top: DecisionAction;          // argmax of (long, short, hold)
    top_prob: number;
    /** The hero label & colour. */
    headline: {
        action: DecisionAction | 'STAND_ASIDE';
        label: string;
        state: DecisionState;
        confidence_pct: number;
    };
    /** Flat bulleted list of why-the-decision strings, ordered by importance. */
    rationale: string[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry-point
// ─────────────────────────────────────────────────────────────────────────────

export interface DecisionRankInputs {
    advisory: AdvisoryMatrix | null;
    decisionContext: DecisionContext | null;
    opportunity: OpportunityMatrix | null;
    analysis: AnalysisMatrix | null;
}

export function computeDecisionRank(inputs: DecisionRankInputs): DecisionRank {
    const { advisory, decisionContext, opportunity, analysis } = inputs;

    // Defaults — empty / pre-warmup state.
    const score = decisionContext?.score ?? 0;
    const scoreConfidence = decisionContext?.score_confidence ?? 0;
    const bias = decisionContext?.bias ?? 'NEUTRAL';
    const entryDanger = decisionContext?.entry_danger ?? 50;
    const expectedRr = decisionContext?.expected_reward_risk_ratio ?? 0;
    const readiness = (decisionContext?.trade_readiness ?? 'STAND_ASIDE') as DecisionState;
    const confidence = advisory?.confidence_assessment ?? 0;
    const guidance = advisory?.directional_guidance ?? 'Neutral';
    const stance = advisory?.market_stance ?? 'Neutral';
    const supporters = decisionContext?.contributing_indicators ?? [];
    const oppScore = opportunity?.opportunity_score ?? 0;
    const setupQuality = opportunity?.setup_quality ?? '—';
    const setupType = opportunity?.primary_opportunity ?? 'NoClearOpportunity';
    const supportingSignals = analysis?.supporting_signals ?? [];
    const contradictingSignals = analysis?.contradicting_signals ?? [];

    // ── 1. Raw signal scores (each ∈ [0, 100]) ─────────────────────────────
    // bias × score-derivation: positive score → long, negative → short,
    // HOLD absorbs entries that the gate should close (entry_danger high,
    // STAND_ASIDE readiness).
    const baseLong = clamp(0, 100, Math.max(0, score) * scoreConfidence);
    const baseShort = clamp(0, 100, Math.max(0, -score) * scoreConfidence);
    const baseHold = clamp(
        0,
        100,
        (entryDanger / 100) * 50 + (readiness === 'WATCH' || readiness === 'STAND_ASIDE' ? 50 : 0),
    );

    // ── 2. Bias / guidance / stance modulation ─────────────────────────────
    let long = baseLong;
    let short = baseShort;
    let hold = baseHold;

    const g = guidance.toLowerCase();
    const s = stance.toLowerCase();

    if (g.includes('long')) {
        long *= 1.2;
        short *= 0.5;
    } else if (g.includes('short')) {
        short *= 1.2;
        long *= 0.5;
    }

    if (s === 'aggressive' || s === 'constructive') {
        if (g.includes('long')) long *= 1.15;
        else if (g.includes('short')) short *= 1.15;
    }
    if (s === 'avoid') {
        long *= 0.5;
        short *= 0.5;
        hold *= 1.5;
    }

    if (expectedRr < 1.0) {
        if (g.includes('long')) long *= 0.6;
        else if (g.includes('short')) short *= 0.6;
    }

    long = clamp(0, 100, long);
    short = clamp(0, 100, short);
    hold = clamp(0, 100, hold);

    // ── 3. Renormalize to sum to 100 (largest absorbs rounding residual) ──
    const sum = long + short + hold;
    if (sum <= 0) {
        long = 34;
        short = 33;
        hold = 33;
    } else {
        const l = Math.round((long / sum) * 100);
        const sh = Math.round((short / sum) * 100);
        const h = 100 - l - sh;
        long = l;
        short = sh;
        hold = h;
    }

    // ── 4. Top action ─────────────────────────────────────────────────────
    let top: DecisionAction = 'HOLD';
    let topProb = hold;
    if (long >= short && long >= hold) {
        top = 'LONG';
        topProb = long;
    } else if (short >= long && short >= hold) {
        top = 'SHORT';
        topProb = short;
    }

    // ── 5. Headline (gate-aware) ───────────────────────────────────────────
    let headline: DecisionRank['headline'];
    if (readiness === 'STAND_ASIDE') {
        headline = {
            action: 'STAND_ASIDE',
            label: `HOLD — STAND ASIDE`,
            state: 'STAND_ASIDE',
            confidence_pct: Math.round(confidence),
        };
    } else if (top === 'HOLD') {
        headline = {
            action: 'HOLD',
            label: readiness === 'READY' ? 'HOLD — READY (awaiting trigger)' : `HOLD — ${readiness}`,
            state: readiness,
            confidence_pct: Math.round(confidence),
        };
    } else {
        headline = {
            action: top,
            label: `${top} — ${readiness}`,
            state: readiness,
            confidence_pct: Math.round(confidence),
        };
    }

    // ── 6. Rationale ──────────────────────────────────────────────────────
    const rationale = buildRationale({
        score,
        bias,
        readiness,
        guidance,
        stance,
        entryDanger,
        expectedRr,
        confidence,
        oppScore,
        setupQuality,
        setupType,
        supporters,
        supportingSignals,
        contradictingSignals,
        top,
    });

    return {
        long: { probability: long, reasons: [] },
        short: { probability: short, reasons: [] },
        hold: { probability: hold, reasons: [] },
        top,
        top_prob: topProb,
        headline,
        rationale,
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// Setup mirroring
// ─────────────────────────────────────────────────────────────────────────────

export interface SetupLevel {
    label: 'ENTRY' | 'TP1' | 'TP2' | 'SL';
    price: number;
    note?: string;
}

export interface SymmetricSetup {
    side: 'LONG' | 'SHORT';
    active: boolean;
    entry: SetupLevel | null;
    targets: SetupLevel[];
    stop: SetupLevel | null;
    status: string;
    rrRatio: number | null;
}

export interface MirrorInputs {
    opportunity: OpportunityMatrix | null;
    markPrice: number;
    topAction: DecisionAction;
    readiness: DecisionState;
}

function level(price: number, label: SetupLevel['label'], note?: string): SetupLevel {
    return { label, price, note };
}

function fmtPx(n: number, mp: number): string {
    if (n == null || !isFinite(n) || n <= 0) return '—';
    if (mp >= 1000) return `$${n.toFixed(0)}`;
    if (mp >= 1) return `$${n.toFixed(2)}`;
    return `$${n.toFixed(4)}`;
}

function fmtDistancePct(current: number, base: number): string {
    if (base <= 0 || current <= 0) return '—';
    const pct = ((current - base) / base) * 100;
    return `${pct >= 0 ? '+' : ''}${pct.toFixed(2)}%`;
}

/**
 * Build a symmetric Long / Short setup pair from the L4 Opportunity Matrix.
 * The active side (matching the top action) is the canonical L4 reading;
 * the inactive side is the price-mirror around `markPrice` so the operator
 * can see what the opposite trade would look like.
 */
export function computeSymmetricSetups(inputs: MirrorInputs): {
    long: SymmetricSetup;
    short: SymmetricSetup;
} {
    const { opportunity, markPrice, topAction, readiness } = inputs;
    const err = (label: string): string => label;

    if (!opportunity || markPrice <= 0) {
        const empty: SymmetricSetup = {
            side: 'LONG',
            active: false,
            entry: null,
            targets: [],
            stop: null,
            status: err('inactive (no L4 opportunity)'),
            rrRatio: null,
        };
        return { long: { ...empty, side: 'LONG' }, short: { ...empty, side: 'SHORT' } };
    }

    const entryLow = opportunity.entry_zone?.low ?? 0;
    const entryHigh = opportunity.entry_zone?.high ?? 0;
    const entryMid = entryLow > 0 && entryHigh > 0 ? (entryLow + entryHigh) / 2 : markPrice;
    const tp1 = opportunity.target_zone?.high ?? 0;
    const tp2 = opportunity.target_zone?.low ?? 0;
    const invalidation = opportunity.invalidation_level ?? 0;

    const gate = readiness === 'STAND_ASIDE' || readiness === 'WATCH';
    const longActive = topAction === 'LONG' && !gate;
    const shortActive = topAction === 'SHORT' && !gate;

    // ── Long setup (canonical) ────────────────────────────────────────────
    const longTargets: SetupLevel[] = [];
    if (tp1 > 0) longTargets.push(level(tp1, 'TP1', 'L4 target zone high'));
    if (tp2 > 0 && tp2 !== tp1) longTargets.push(level(tp2, 'TP2', 'L4 target zone low'));
    const longStop: SetupLevel | null =
        invalidation > 0 && invalidation < entryMid
            ? level(invalidation, 'SL', 'L4 invalidation')
            : null;
    const longRr = longStop && tp1 > 0 && entryMid > 0
        ? Math.round(((tp1 - entryMid) / Math.max(entryMid - longStop.price, 1e-9)) * 100) / 100
        : null;

    const longSetup: SymmetricSetup = {
        side: 'LONG',
        active: longActive,
        entry: entryLow > 0 && entryHigh > 0
            ? level(entryMid, 'ENTRY', `${fmtPx(entryLow, markPrice)}–${fmtPx(entryHigh, markPrice)}`)
            : null,
        targets: longTargets,
        stop: longStop,
        status: longActive
            ? readiness
            : gate
                ? `inactive (gated: ${readiness})`
                : 'inactive (top action is not LONG)',
        rrRatio: longRr,
    };

    // ── Short setup (mirror around markPrice) ─────────────────────────────
    const shortEntry = entryMid > 0 && markPrice > 0 ? 2 * markPrice - entryMid : 0;
    const shortEntryLow = entryLow > 0 && markPrice > 0 ? 2 * markPrice - entryHigh : 0;
    const shortEntryHigh = entryLow > 0 && markPrice > 0 ? 2 * markPrice - entryLow : 0;
    const shortTp1 = tp1 > 0 && markPrice > 0 ? 2 * markPrice - tp1 : 0;
    const shortTp2 = tp2 > 0 && markPrice > 0 ? 2 * markPrice - tp2 : 0;
    const shortInv = invalidation > 0 && markPrice > 0 ? 2 * markPrice - invalidation : 0;

    const shortTargets: SetupLevel[] = [];
    if (shortTp1 > 0) shortTargets.push(level(shortTp1, 'TP1', 'mirror of L4 target high'));
    if (shortTp2 > 0 && shortTp2 !== shortTp1) shortTargets.push(level(shortTp2, 'TP2', 'mirror of L4 target low'));
    const shortStop: SetupLevel | null =
        shortInv > 0 && shortInv > shortEntry
            ? level(shortInv, 'SL', 'mirror of L4 invalidation')
            : null;
    const shortRr = shortStop && shortTp1 > 0 && shortEntry > 0
        ? Math.round(((shortEntry - shortTp1) / Math.max(shortStop.price - shortEntry, 1e-9)) * 100) / 100
        : null;

    const shortSetup: SymmetricSetup = {
        side: 'SHORT',
        active: shortActive,
        entry: shortEntry > 0
            ? level(
                shortEntry,
                'ENTRY',
                `${fmtPx(shortEntryHigh, markPrice)}–${fmtPx(shortEntryLow, markPrice)}`,
            )
            : null,
        targets: shortTargets,
        stop: shortStop,
        status: shortActive
            ? readiness
            : gate
                ? `inactive (gated: ${readiness})`
                : 'inactive (top action is not SHORT)',
        rrRatio: shortRr,
    };

    return { long: longSetup, short: shortSetup };
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function clamp(lo: number, hi: number, x: number): number {
    return Math.min(hi, Math.max(lo, x));
}

interface RationaleInputs {
    score: number;
    bias: string;
    readiness: DecisionState;
    guidance: string;
    stance: string;
    entryDanger: number;
    expectedRr: number;
    confidence: number;
    oppScore: number;
    setupQuality: string;
    setupType: string;
    supporters: string[];
    supportingSignals: string[];
    contradictingSignals: string[];
    top: DecisionAction;
}

function entryDangerLevel(d: number): string {
    if (d >= 80) return 'EXTREME';
    if (d >= 60) return 'HIGH';
    if (d >= 40) return 'MODERATE';
    if (d >= 20) return 'LOW';
    return 'VERY_LOW';
}

function buildRationale(r: RationaleInputs): string[] {
    const out: string[] = [];

    // 1. Bias + score
    out.push(
        `${r.bias} bias, confluence score ${r.score.toFixed(0)} (L2 tradability_dim + L3 quality + L4 opportunity)`,
    );

    // 2. Setup identification
    out.push(
        `Setup: ${r.setupType} (L4 score ${Math.round(r.oppScore)}, ${r.setupQuality})`,
    );

    // 3. Trade readiness gate
    if (r.readiness === 'STAND_ASIDE') {
        out.push(
            `Trade readiness = STAND_ASIDE because entry_danger ${r.entryDanger.toFixed(0)} (${entryDangerLevel(r.entryDanger)}) ≥ 70`,
        );
    } else if (r.readiness === 'FORMING') {
        out.push(
            `Trade readiness = FORMING — entry_danger ${r.entryDanger.toFixed(0)} (${entryDangerLevel(r.entryDanger)}) capped the long/short score`,
        );
    } else if (r.readiness === 'WATCH') {
        out.push(
            `Trade readiness = WATCH — entry_danger ${r.entryDanger.toFixed(0)} (${entryDangerLevel(r.entryDanger)}) watches for confirmation`,
        );
    } else {
        out.push(`Trade readiness = READY (entry_danger ${r.entryDanger.toFixed(0)})`);
    }

    // 4. R:R
    if (r.expectedRr < 1.0) {
        out.push(`Risk-discounted R:R ${r.expectedRr.toFixed(2)} (< 1.0) — ${r.top} score capped at 60%`);
    } else {
        out.push(`Risk-discounted R:R ${r.expectedRr.toFixed(2)}`);
    }

    // 5. Contributing indicators
    if (r.supporters.length > 0) {
        out.push(`Contributing indicators: ${r.supporters.slice(0, 6).join(', ')}`);
    }

    // 6. Supporting signals from L3
    if (r.supportingSignals.length > 0) {
        out.push(`L3 supporting signals: ${r.supportingSignals.slice(0, 4).join(', ')}`);
    }

    // 7. Contradicting signals (if any)
    if (r.contradictingSignals.length > 0) {
        out.push(`Contradicting signals: ${r.contradictingSignals.slice(0, 3).join(', ')}`);
    }

    // 8. Stance × guidance
    out.push(`Directional guidance: ${r.guidance}  ·  Market stance: ${r.stance}`);

    return out;
}
