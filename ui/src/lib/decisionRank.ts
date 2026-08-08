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
    MarketBias,
    OpportunityMatrix,
    OpportunityProfile,
    PriceRange,
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
    // `decisionContext.entry_danger` is now a RiskDimension-shaped object
    // (matches the wire and the Rust struct). Defensively extract the
    // scalar — handle the legacy bare-number shape that older snapshots
    // may still send.
    const entryDangerRaw = decisionContext?.entry_danger;
    const entryDanger =
        typeof entryDangerRaw === 'number'
            ? entryDangerRaw
            : entryDangerRaw?.score ?? 50;
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

    // GEOMETRIC OFFSET: If the macro trend is neutral (score=0), inspect the top-scored
    // latent setup using the top-level opportunity matrix zones as a fallback.
    let directionalOffset = 0;
    if (score === 0 && opportunity) {
        const profiles = opportunity.profiles ?? [];
        const qualifying = profiles.filter(
            (p) => p.preconditions_met >= 0 && p.opportunity_type !== 'NoClearOpportunity'
        );
        if (qualifying.length > 0) {
            const topProfile = [...qualifying].sort((a, b) => b.score - a.score)[0];

            const hasLong = opportunity.long_entry_zone && opportunity.long_entry_zone.low > 0;
            const hasShort = opportunity.short_entry_zone && opportunity.short_entry_zone.low > 0;

            let resolvedSide: 'LONG' | 'SHORT' | 'NEUTRAL' = 'NEUTRAL';
            if (hasLong && !hasShort) {
                resolvedSide = 'LONG';
            } else if (hasShort && !hasLong) {
                resolvedSide = 'SHORT';
            } else if (hasLong && hasShort) {
                const longRr = opportunity.long_expected_rr_internal ?? 0;
                const shortRr = opportunity.short_expected_rr_internal ?? 0;
                resolvedSide = longRr >= shortRr ? 'LONG' : 'SHORT';
            }

            if (resolvedSide === 'LONG') {
                directionalOffset = topProfile.score * 0.15;
            } else if (resolvedSide === 'SHORT') {
                directionalOffset = -topProfile.score * 0.15;
            }
        }
    }

    const effectiveScore = score !== 0 ? score : directionalOffset;
    const effectiveConfidence = scoreConfidence || 0.5;

    // ── 1. Raw signal scores (each ∈ [0, 100]) ─────────────────────────────
    // bias × score-derivation: positive score → long, negative → short,
    // HOLD absorbs entries that the gate should close (entry_danger high,
    // STAND_ASIDE readiness).
    const baseLong = clamp(0, 100, Math.max(0, effectiveScore) * effectiveConfidence);
    const baseShort = clamp(0, 100, Math.max(0, -effectiveScore) * effectiveConfidence);
    const baseHold = clamp(
        0,
        100,
        (entryDanger / 100) * 50,
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
    let hadSignal = false;
    const sum = long + short + hold;
    if (sum <= 0) {
        long = 34;
        short = 33;
        hold = 33;
    } else {
        hadSignal = true;
        const l = Math.round((long / sum) * 100);
        const sh = Math.round((short / sum) * 100);
        const h = 100 - l - sh;
        long = l;
        short = sh;
        hold = h;
    }
    if (hadSignal) {
        const MIN_PCT = 2;
        long = Math.max(long, MIN_PCT);
        short = Math.max(short, MIN_PCT);
        hold = Math.max(hold, MIN_PCT);
        const reSum = long + short + hold;
        long = Math.round((long / reSum) * 100);
        short = Math.round((short / reSum) * 100);
        hold = 100 - long - short;
    }

    // ── 4. Top action ─────────────────────────────────────────────────────
    // Degenerate-rank guard: if the leading action is below 35% probability,
    // the data does not support a directional call. Collapse to HOLD so the
    // operator does not see a confident "LONG" headline when the math
    // actually says "all three are about equal".
    let top: DecisionAction = 'HOLD';
    let topProb = hold;
    const maxProb = Math.max(long, short, hold);
    if (maxProb >= 35) {
        if (long === maxProb) {
            top = 'LONG';
            topProb = long;
        } else if (short === maxProb) {
            top = 'SHORT';
            topProb = short;
        }
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
    /**
     * Geometric-consistency flag — true when the displayed prices match
     * the trade direction implied by entry vs target vs invalidation.
     * A long with entry above target, or short with entry below target,
     * is geometrically inverted (typically means the wire's
     * `entry_zone` and `target_zone` describe the opposite direction).
     * The panel renders an informational note when this is false.
     */
    geometry_consistent: boolean;
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
 *
 * Each side reads its **per-direction** zones directly:
 *   - Long side  → `opportunity.long_entry_zone / long_target_zone / long_invalidation_level`
 *   - Short side → `opportunity.short_entry_zone / short_target_zone / short_invalidation_level`
 *
 * TP1 is the **nearest** profitable target (smallest absolute distance from
 * entry mid); TP2 is the farther target. The R:R and stop checks are
 * direction-aware so a SHORT geometry (entry below target, inval above entry)
 * cannot accidentally inherit LONG-only invariants.
 *
 * The legacy single-bias `entry_zone / target_zone / invalidation_level`
 * fields are intentionally NOT used here: they collapse to the SHORT side
 * for Neutral bias and to whichever side the active bias picks, which
 * produces the LONG-vs-SHORT geometry inversion the user reported.
 */
export function computeSymmetricSetups(inputs: MirrorInputs): {
    long: SymmetricSetup;
    short: SymmetricSetup;
} {
    const { opportunity, markPrice, topAction, readiness } = inputs;

    const empty: SymmetricSetup = {
        side: 'LONG',
        active: false,
        entry: null,
        targets: [],
        stop: null,
        status: 'inactive (no L4 opportunity)',
        rrRatio: null,
        geometry_consistent: false,
    };

    if (!opportunity || markPrice <= 0) {
        return {
            long: { ...empty, side: 'LONG' },
            short: { ...empty, side: 'SHORT' },
        };
    }

    const gate = readiness === 'STAND_ASIDE' || readiness === 'WATCH';
    const longActive = topAction === 'LONG' && !gate;
    const shortActive = topAction === 'SHORT' && !gate;

    // ── Per-side selector: reads `long_*` / `short_*` directly. Returns
    // null when the side has no usable zones (e.g. Neutral sentinel where
    // every level pins to close).
    const selectSide = (side: 'LONG' | 'SHORT') => {
        const entry = side === 'LONG' ? opportunity.long_entry_zone : opportunity.short_entry_zone;
        const target = side === 'LONG' ? opportunity.long_target_zone : opportunity.short_target_zone;
        const inv = side === 'LONG' ? opportunity.long_invalidation_level : opportunity.short_invalidation_level;
        if (!entry || entry.low <= 0 || entry.high <= 0 || inv <= 0) return null;
        const entryMid = (entry.low + entry.high) / 2;
        // Order TP1 = nearest to entry_mid, TP2 = farther. The wire layout
        // puts `target.low`/`target.high` in different visual order for LONG
        // vs SHORT (LONG: low is conservative/closer; SHORT: low is farther).
        // Sorting by absolute distance to entry_mid gives a stable
        // direction-agnostic "nearest first" ordering.
        const tpCandidates = [target.low, target.high].filter((p) => p > 0);
        if (tpCandidates.length === 0) return null;
        const sorted = [...tpCandidates].sort((a, b) =>
            Math.abs(a - entryMid) - Math.abs(b - entryMid),
        );
        const tp1 = sorted[0];
        const tp2 = sorted.length > 1 ? sorted[1] : tp1;
        return { entry, entryMid, target, inv, tp1, tp2 };
    };

    const build = (side: 'LONG' | 'SHORT', raw: ReturnType<typeof selectSide>): SymmetricSetup => {
        if (!raw) {
            return { ...empty, side };
        }
        const { entry, entryMid, target, inv, tp1, tp2 } = raw;
        const isLong = side === 'LONG';
        // Direction-aware stop check: LONG inval < entry_mid; SHORT inval > entry_mid.
        const validStop = isLong ? inv < entryMid : inv > entryMid;
        // Direction-aware reward: LONG target above entry_mid; SHORT below.
        const reward = isLong ? tp1 - entryMid : entryMid - tp1;
        const risk = isLong ? entryMid - inv : inv - entryMid;
        const stop = validStop ? level(inv, 'SL', `L4 ${side.toLowerCase()} invalidation`) : null;
        const rrRatio = validStop && reward > 0 && risk > 0
            ? Math.round((reward / risk) * 100) / 100
            : null;
        const geometry_consistent = validStop && reward > 0 && risk > 0;

        const isActive = (isLong ? longActive : shortActive);
        const status = isActive
            ? readiness
            : gate
                ? `inactive (gated: ${readiness})`
                : `inactive (top action is not ${side})`;

        const targets: SetupLevel[] = [];
        if (tp1 > 0) targets.push(level(tp1, 'TP1', 'nearest profitable target'));
        if (tp2 > 0 && tp2 !== tp1) targets.push(level(tp2, 'TP2', 'farther profitable target'));

        return {
            side,
            active: isActive,
            entry: level(
                entryMid,
                'ENTRY',
                `${fmtPx(entry.low, markPrice)}–${fmtPx(entry.high, markPrice)}`,
            ),
            targets,
            stop,
            status,
            rrRatio,
            geometry_consistent,
        };
    };

    const longRaw = selectSide('LONG');
    const shortRaw = selectSide('SHORT');

    return {
        long: build('LONG', longRaw),
        short: build('SHORT', shortRaw),
    };
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

export function entryDangerLevel(d: number): 'EXTREME' | 'HIGH' | 'MODERATE' | 'LOW' | 'VERY_LOW' {
    if (d >= 80) return 'EXTREME';
    if (d >= 60) return 'HIGH';
    if (d >= 40) return 'MODERATE';
    if (d >= 20) return 'LOW';
    return 'VERY_LOW';
}

/**
 * Resolve a profile's actual trade direction from `DirectionFamily` ×
 * `Analysis.bias`. Returns `'NEUTRAL'` for Neutral families or when
 * the macro bias is itself Neutral.
 *
 * `TrendRiding` profiles follow the macro bias; `CounterTrend` profiles
 * reverse it. This is the wire-side source of truth — the heuristic
 * substring match that previously lived in the RecommendationPanel is
 * removed in favour of this helper.
 */
export function selectProfileSide(
    profile: OpportunityProfile | null | undefined,
    macroBias: MarketBias | null | undefined,
): 'LONG' | 'SHORT' | 'NEUTRAL' {
    if (!profile || !macroBias) return 'NEUTRAL';
    const family = profile.direction_family ?? null;
    const isBullish = macroBias === 'Bullish' || macroBias === 'StrongBullish';
    const isBearish = macroBias === 'Bearish' || macroBias === 'StrongBearish';
    switch (family) {
        case 'TrendRiding':
            if (isBullish) return 'LONG';
            if (isBearish) return 'SHORT';
            return 'NEUTRAL';
        case 'CounterTrend':
            if (isBullish) return 'SHORT';
            if (isBearish) return 'LONG';
            return 'NEUTRAL';
        case 'Neutral':
        case null:
        case undefined:
            return 'NEUTRAL';
        default:
            return 'NEUTRAL';
    }
}

export interface ProfileZones {
    side: 'LONG' | 'SHORT';
    entry: PriceRange;
    target: PriceRange;
    invalidation: number;
    /** Direction-aware R:R (positive reward / positive risk). `null` when degenerate. */
    rr: number | null;
    /** `true` iff the displayed prices match the side's geometric invariant. */
    geometry_consistent: boolean;
}

/**
 * Read the resolved per-side zones directly from an `OpportunityProfile`.
 * Returns `null` when the profile has no zones for that side (legacy
 * payloads, Neutral family, or un-met preconditions).
 */
export function profileZones(
    profile: OpportunityProfile | null | undefined,
    side: 'LONG' | 'SHORT',
): ProfileZones | null {
    if (!profile) return null;
    const entry = side === 'LONG' ? profile.long_entry_zone : profile.short_entry_zone;
    const target = side === 'LONG' ? profile.long_target_zone : profile.short_target_zone;
    const inv = side === 'LONG' ? profile.long_invalidation_level : profile.short_invalidation_level;
    if (!entry || !target || entry.low <= 0 || entry.high <= 0 || !inv || inv <= 0) {
        return null;
    }
    const entryMid = (entry.low + entry.high) / 2;
    const reward = side === 'LONG' ? target.low - entryMid : entryMid - target.high;
    const risk = side === 'LONG' ? entryMid - inv : inv - entryMid;
    const geometry_consistent = reward > 0 && risk > 0;
    const rr = geometry_consistent ? Math.round((reward / risk) * 100) / 100 : null;
    return { side, entry, target, invalidation: inv, rr, geometry_consistent };
}

/**
 * Read the aggregated per-direction bracket from `OpportunityMatrix`.
 * Used as a **fallback** when per-profile zones are absent — the
 * aggregated fields always have a value (even the Neutral sentinel
 * pinned to close when bias is Neutral).
 */
export function aggregateZones(
    opportunity: OpportunityMatrix | null | undefined,
    side: 'LONG' | 'SHORT',
): ProfileZones | null {
    if (!opportunity) return null;
    const entry = side === 'LONG' ? opportunity.long_entry_zone : opportunity.short_entry_zone;
    const target = side === 'LONG' ? opportunity.long_target_zone : opportunity.short_target_zone;
    const inv = side === 'LONG' ? opportunity.long_invalidation_level : opportunity.short_invalidation_level;
    if (!entry || !target || entry.low <= 0 || entry.high <= 0 || !inv || inv <= 0) {
        return null;
    }
    return profileZones(
        {
            opportunity_type: '__aggregate__',
            score: opportunity.opportunity_score,
            preconditions_met: 0,
            preconditions_total: 0,
            notes: '',
            direction_family: null,
            long_entry_zone: entry,
            long_target_zone: target,
            long_invalidation_level: inv,
            long_expected_rr_internal: null,
            short_entry_zone: entry,
            short_target_zone: target,
            short_invalidation_level: inv,
            short_expected_rr_internal: null,
            trade_viability: null,
        } as OpportunityProfile,
        side,
    );
}

/**
 * Resolve the zones for the top setup, falling back from per-profile to
 * aggregated when the per-profile zones are absent. Returns:
 *   - `zones`: never null when `opportunity` is present (uses aggregate
 *     fallback so every Top Setup card carries ENTRY/TARGET/SL/R:R).
 *   - `rr`: the per-side R:R from `opportunity.long_expected_rr_internal`
 *     or `short_expected_rr_internal` (the wire-side truth); falls back
 *     to the geometric R:R derived from the zones when both are zero.
 *   - `side`: the resolved direction for the top setup, or `'NEUTRAL'`
 *     when no resolvable direction exists.
 *   - `viability`: `'Actionable' | 'DirectionalNeutral' | 'GeometryInverted' | 'NoClear'`.
 *   - `rationale`: a single user-facing line (cleaned of raw/ratio).
 *
 * The Recommendation panel calls this helper so it always renders
 * ENTRY/TARGET/SL/R:R — even when the verdict is HOLD or the top
 * profile has no actionable per-side zones.
 */
export interface TopSetupSummary {
    opportunity_type: string;
    score: number;
    preconditions_met: number;
    preconditions_total: number;
    direction: 'LONG' | 'SHORT' | 'NEUTRAL';
    viability: 'Actionable' | 'DirectionalNeutral' | 'GeometryInverted' | 'NoClear';
    zones: ProfileZones | null;
    rr: number | null;
    rationale: string;
}

export function topSetupSummary(
    opportunity: OpportunityMatrix | null | undefined,
    analysis: AnalysisMatrix | null | undefined,
): TopSetupSummary | null {
    if (!opportunity) return null;
    const profiles = opportunity.profiles ?? [];
    const qualifying = profiles
        .filter((p) => p.preconditions_met > 0 && p.opportunity_type !== 'NoClearOpportunity')
        .slice()
        .sort((a, b) => b.score - a.score);
    const top = qualifying[0];
    if (!top) {
        return null;
    }
    const macroBias = analysis?.bias ?? null;
    const side = selectProfileSide(top, macroBias);
    // Try per-profile zones first; fall back to aggregated.
    let zones = side === 'NEUTRAL' ? null : profileZones(top, side);
    if (!zones) {
        const fallbackSide = side === 'NEUTRAL' ? 'LONG' : side;
        zones = aggregateZones(opportunity, fallbackSide);
    }
    // Per-side R:R from the wire (canonical), fall back to the
    // geometric R:R derived from the zones.
    const wireRr =
        side === 'LONG'
            ? top.long_expected_rr_internal
            : side === 'SHORT'
              ? top.short_expected_rr_internal
              : 0;
    const rr = (wireRr && wireRr > 0)
        ? Math.round(wireRr * 100) / 100
        : zones?.rr ?? null;
    // Viability — wire-side default to `NoClear` when missing.
    const viability = (top.trade_viability ?? 'NoClear') as TopSetupSummary['viability'];
    // Clean rationale (no raw/ratio debug strings).
    const rationale = `${top.opportunity_type}: preconditions ${top.preconditions_met}/${top.preconditions_total}`;
    return {
        opportunity_type: top.opportunity_type,
        score: top.score,
        preconditions_met: top.preconditions_met,
        preconditions_total: top.preconditions_total,
        direction: side,
        viability,
        zones,
        rr,
        rationale,
    };
}

/**
 * Resolve the zones for ANY qualifying profile, falling back from
 * per-profile to aggregated. Used by the Opportunities panel so every
 * qualifying card carries ENTRY/TARGET/SL/R:R.
 */
export function profileSummary(
    profile: OpportunityProfile | null | undefined,
    opportunity: OpportunityMatrix | null | undefined,
    analysis: AnalysisMatrix | null | undefined,
): {
    side: 'LONG' | 'SHORT' | 'NEUTRAL';
    zones: ProfileZones | null;
    rr: number | null;
    viability: 'Actionable' | 'DirectionalNeutral' | 'GeometryInverted' | 'NoClear';
} {
    if (!profile) {
        return { side: 'NEUTRAL', zones: null, rr: null, viability: 'NoClear' };
    }
    const macroBias = analysis?.bias ?? null;
    const side = selectProfileSide(profile, macroBias);
    let zones = side === 'NEUTRAL' ? null : profileZones(profile, side);
    if (!zones) {
        const fallbackSide = side === 'NEUTRAL' ? 'LONG' : side;
        zones = aggregateZones(opportunity, fallbackSide);
    }
    const wireRr =
        side === 'LONG'
            ? profile.long_expected_rr_internal
            : side === 'SHORT'
              ? profile.short_expected_rr_internal
              : 0;
    const rr = (wireRr && wireRr > 0)
        ? Math.round(wireRr * 100) / 100
        : zones?.rr ?? null;
    const viability = (profile.trade_viability ?? 'NoClear') as 'Actionable' | 'DirectionalNeutral' | 'GeometryInverted' | 'NoClear';
    return { side, zones, rr, viability };
}

function buildRationale(r: RationaleInputs): string[] {
    const out: string[] = [];

    // 1. Bias + score
    // Use `r.bias` literally to keep the prose aligned with the wire's
    // `decision_context.bias` (NEUTRAL/BULLISH/BEARISH). The math (see
    // score sign in `computeDecisionRank`) can produce a directional
    // call even when bias is NEUTRAL — call that out so the operator
    // understands why the hero disagrees with the bias string.
    const driverSuffix =
        r.bias === 'NEUTRAL' && r.top !== 'HOLD'
            ? ` (math-driven by confluence score ${r.score.toFixed(0)}; wire bias reads NEUTRAL)`
            : '';
    out.push(
        `${r.bias} bias, confluence score ${r.score.toFixed(0)} (L2 tradability_dim + L3 quality + L4 opportunity)${driverSuffix}`,
    );

    // 2. Setup identification
    out.push(
        `Setup: ${r.setupType} (L4 score ${Math.round(r.oppScore)}, ${r.setupQuality})`,
    );

    // 3. Trade readiness gate — narrative now matches the actual gate
    // logic. Only `entry_danger ≥ 70` truly caps scores; the FORMING /
    // WATCH / READY states are downstream of `expected_reward_risk_ratio`.
    if (r.readiness === 'STAND_ASIDE') {
        out.push(
            `Trade readiness = STAND_ASIDE because entry_danger ${r.entryDanger.toFixed(0)} (${entryDangerLevel(r.entryDanger)}) ≥ 70`,
        );
    } else if (r.readiness === 'FORMING' && r.expectedRr < 1.0) {
        out.push(
            `Trade readiness = FORMING — risk-discounted R:R ${r.expectedRr.toFixed(2)} (< 1.0) capped the directional score`,
        );
    } else if (r.readiness === 'FORMING') {
        out.push(
            `Trade readiness = FORMING — entry_danger ${r.entryDanger.toFixed(0)} (${entryDangerLevel(r.entryDanger)}) is acceptable but market state is mid-form`,
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
        out.push(`Risk-discounted R:R ${r.expectedRr.toFixed(2)} (< 1.0) — ${r.top} score capped at 60% of pre-normalized total`);
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
