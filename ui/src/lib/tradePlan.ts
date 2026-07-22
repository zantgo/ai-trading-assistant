// Trade Plan builders — pure helpers for the L4/L6 surfaces.
//
// `deriveTradePlan()` consumes the same wire-format payloads the existing
// AdvisoryPanel, OpportunitiesPanel, and StructuralAnchorsStrip already read,
// and produces a single `TradePlan` view that the Metrics tab and the
// Decision tab can render without re-implementing the math.
//
// The plan is **strategy-agnostic**: it is shape (entry/target/stop) not
// authorisation. The Trade Automation Engine decides whether to dispatch.
// Tools like `applyPlanToConsole()` pre-fill the BottomConsole bracket
// creator for manual confirmation.

import type {
    AdvisoryMatrix,
    AnalysisMatrix,
    ConfluentLevel,
    MarketContext,
    OpportunityMatrix,
    TimeframeTelemetry,
} from '../types';

export type SourceTag = 'FIB' | 'VP' | 'PP' | 'SR' | 'LIQ' | 'ATR' | 'NONE';

export interface TradeTarget {
    label: string;          // "TP1" / "TP2" / "TP3"
    price: number;
    sizePct: number;         // 0..100
    rrRatio: number | null; // R:R for this target vs entry mid
    source: 'L4_TARGET_ZONE' | 'FIB_EXT_1618' | 'FIB_EXT_2618' | 'CONFLUENT' | 'NONE';
    confluenceCount?: number;
}

export interface TradeStop {
    price: number;
    distancePct: number;     // 0..100  pct from entry mid
    method: 'STRUCTURE_BASED' | 'VOLATILITY_BASED' | 'ATR_BASED' | 'SR_BASED' | 'NONE';
    fallbackPrice?: number;
    source: 'L4_INVALIDATION' | 'CONFLUENT' | 'PCT_FALLBACK' | 'NONE';
    evidenceNote?: string;
}

export interface TradePlan {
    symbol: string;
    direction: 'LONG' | 'SHORT' | 'NEUTRAL';
    setupType: string;          // "TrendContinuation" / "Pullback" / etc
    setupScore: number;         // 0..100
    setupQuality: string;       // "PRIME" / "STRONG" / ...
    timeHorizon: 'SCALP' | 'INTRADAY' | 'SWING' | 'POSITION';
    readiness: 'READY' | 'FORMING' | 'WATCH' | 'STAND_ASIDE' | 'UNKNOWN';
    confidencePct: number;      // 0..100  (terminal confidence_assessment)
    rrRatio: number;            // expected_reward_risk_ratio
    riskDiscount: number;       // overall_risk.score
    entryMid: number;           // midpoint of entry zone
    entryZone: { low: number; high: number };
    entrySources: { tag: SourceTag; price: number; strength: number }[];
    targets: TradeTarget[];
    stop: TradeStop | null;
    entryGuidance: string;
    exitGuidance: string;
    targetStrategy: string;
    protectionStrategy: string;
    contributors: string[];
    actionable: boolean;
    actionabilityReason: string;
}

// ───────────────────────────── formatters ─────────────────────────────

function fmtPrice(num: number): number {
    if (!isFinite(num) || num <= 0) return 0;
    return Math.round(num * 100) / 100;
}

function clampPct(n: number): number {
    if (!isFinite(n)) return 50;
    return Math.min(100, Math.max(0, n));
}

function rr(entryMid: number, target: number, stop: number): number | null {
    if (entryMid <= 0 || stop <= 0 || target <= 0) return null;
    const risk = entryMid - stop;
    const reward = target - entryMid;
    if (risk <= 0) return null;
    return Math.round((reward / risk) * 100) / 100;
}

function horizonFactor(horizon: string): { targets: number; stopPct: number } {
    const h = (horizon ?? '').toUpperCase();
    if (h === 'SCALP') return { targets: 1, stopPct: 0.4 };
    if (h === 'INTRADAY') return { targets: 2, stopPct: 1.0 };
    if (h === 'POSITION') return { targets: 3, stopPct: 4.0 };
    return { targets: 3, stopPct: 2.0 }; // SWING default
}

// ───────────────────────────── core derivation ─────────────────────────────

export interface DeriveArgs {
    symbol: string;
    markPrice: number;
    opportunity: OpportunityMatrix | null;
    advisory: AdvisoryMatrix | null;
    analysis: AnalysisMatrix | null;
    decisionContext: { score?: number; bias?: string; trade_readiness?: string; entry_danger?: { level?: string }; expected_reward_risk_ratio?: number; contributing_indicators?: string[] } | null;
    /** Active timeframe telemetry — used for Fibonacci extension targets. */
    tf: TimeframeTelemetry | undefined;
    microTf: TimeframeTelemetry | undefined;
    /** Optional Risk Matrix overall_risk.score for the discount display. */
    overallRisk?: number;
}

export function deriveTradePlan(args: DeriveArgs): TradePlan {
    const { symbol, markPrice, opportunity, advisory, analysis, decisionContext, tf, microTf, overallRisk } = args;

    const ready: TradePlan['readiness'] = decisionContext?.trade_readiness === 'READY' ? 'READY'
        : decisionContext?.trade_readiness === 'FORMING' ? 'FORMING'
        : decisionContext?.trade_readiness === 'WATCH' ? 'WATCH'
        : decisionContext?.trade_readiness === 'STAND_ASIDE' ? 'STAND_ASIDE'
        : 'UNKNOWN';

    const direction: TradePlan['direction'] =
        advisory?.directional_guidance?.includes('Long') ? 'LONG'
        : advisory?.directional_guidance?.includes('Short') ? 'SHORT'
        : 'NEUTRAL';

    const setupType = opportunity?.primary_opportunity ?? 'NoClearOpportunity';
    const setupScore = Math.round(opportunity?.opportunity_score ?? 0);
    const setupQuality = opportunity?.setup_quality ?? '—';
    const horizonRaw = opportunity?.time_horizon ?? 'SWING';
    const timeHorizon = (['SCALP', 'INTRADAY', 'SWING', 'POSITION'].includes(horizonRaw)
        ? horizonRaw
        : 'SWING') as TradePlan['timeHorizon'];
    const horizonCfg = horizonFactor(timeHorizon);

    const confidencePct = Math.round(advisory?.confidence_assessment ?? 0);

    const decisionRr = decisionContext?.expected_reward_risk_ratio ?? opportunity?.expected_rr_internal ?? 0;
    const rrRatio = Math.round(decisionRr * 100) / 100;

    const entryZone = opportunity?.entry_zone;
    const entryMid = entryZone
        ? fmtPrice((entryZone.low + entryZone.high) / 2)
        : (markPrice > 0 ? fmtPrice(markPrice) : 0);

    // ── Targets ──────────────────────────────────────────
    const targets: TradeTarget[] = [];
    const sizeFor = (i: number, total: number) => total <= 0 ? 0 : Math.round((i === 0 ? 40 : (i === 1 ? 40 : 20)));

    if (entryMid > 0 && horizonCfg.targets >= 1) {
        const t1Price = opportunity?.target_zone?.high ?? 0;
        if (t1Price > 0) {
            targets.push({
                label: 'TP1',
                price: fmtPrice(t1Price),
                sizePct: 40,
                rrRatio: rr(entryMid, t1Price, opportunity?.invalidation_level ?? 0),
                source: 'L4_TARGET_ZONE',
            });
        } else {
            // Fall back to top confluent target
            const sortedConf = sortedConfluents(opportunity?.confluent_target_levels, 'desc');
            if (sortedConf[0]) {
                targets.push({
                    label: 'TP1',
                    price: fmtPrice(sortedConf[0].price),
                    sizePct: 40,
                    rrRatio: rr(entryMid, sortedConf[0].price, opportunity?.invalidation_level ?? 0),
                    source: 'CONFLUENT',
                    confluenceCount: sortedConf[0].confluence_count,
                });
            }
        }
    }

    if (entryMid > 0 && horizonCfg.targets >= 2 && targets.length === 1) {
        const fibVals = fibValues(tf);
        const ext1618 = fibVals.ext1618;
        const sortedConf = sortedConfluents(opportunity?.confluent_target_levels, 'desc');
        if (typeof ext1618 === 'number' && ext1618 > 0 && ext1618 > entryMid) {
            targets.push({
                label: 'TP2',
                price: fmtPrice(ext1618),
                sizePct: 40,
                rrRatio: rr(entryMid, ext1618, opportunity?.invalidation_level ?? 0),
                source: 'FIB_EXT_1618',
            });
        } else if (sortedConf.length >= 2) {
            targets.push({
                label: 'TP2',
                price: fmtPrice(sortedConf[1].price),
                sizePct: 40,
                rrRatio: rr(entryMid, sortedConf[1].price, opportunity?.invalidation_level ?? 0),
                source: 'CONFLUENT',
                confluenceCount: sortedConf[1].confluence_count,
            });
        }
    }

    if (entryMid > 0 && horizonCfg.targets >= 3 && targets.length === 2) {
        const fibVals = fibValues(tf);
        const ext2618 = fibVals.ext2618;
        const sortedConf = sortedConfluents(opportunity?.confluent_target_levels, 'desc');
        if (typeof ext2618 === 'number' && ext2618 > 0 && ext2618 > entryMid) {
            targets.push({
                label: 'TP3',
                price: fmtPrice(ext2618),
                sizePct: 20,
                rrRatio: rr(entryMid, ext2618, opportunity?.invalidation_level ?? 0),
                source: 'FIB_EXT_2618',
            });
        } else if (sortedConf.length >= 3) {
            targets.push({
                label: 'TP3',
                price: fmtPrice(sortedConf[2].price),
                sizePct: 20,
                rrRatio: rr(entryMid, sortedConf[2].price, opportunity?.invalidation_level ?? 0),
                source: 'CONFLUENT',
                confluenceCount: sortedConf[2].confluence_count,
            });
        }
    }

    // Backfill sizes to sum to 100 if targets overlap (sanity)
    const totalSizes = targets.reduce((s, t) => s + t.sizePct, 0);
    if (totalSizes > 0 && totalSizes !== 100 && targets.length > 0) {
        const last = targets[targets.length - 1];
        last.sizePct += 100 - totalSizes;
    }

    // ── Stop ────────────────────────────────────────────
    let stop: TradeStop | null = null;
    const inv = opportunity?.invalidation_level;
    if (entryMid > 0 && typeof inv === 'number' && inv > 0 && inv < entryMid) {
        const distancePct = ((entryMid - inv) / entryMid) * 100;
        const method = (advisory?.protection_strategy?.includes('Structure')
            ? 'STRUCTURE_BASED'
            : advisory?.protection_strategy?.includes('Volatility')
                ? 'VOLATILITY_BASED'
                : advisory?.protection_strategy?.includes('ATR')
                    ? 'ATR_BASED'
                    : advisory?.protection_strategy?.includes('SR')
                        ? 'SR_BASED'
                        : 'NONE') as TradeStop['method'];

        const sortedInvalConf = sortedConfluents(opportunity?.confluent_invalidation_levels, 'asc');
        const fallback = sortedInvalConf[0]?.price ?? 0;

        const stopPctAdvisory = (advisory as any)?.stop_loss_distance_pct != null
            ? ((advisory as any).stop_loss_distance_pct as number) * 100
            : null;
        const fallbackPrice = stopPctAdvisory != null && entryMid > 0
            ? entryMid * (1 - stopPctAdvisory / 100)
            : (fallback > 0 && fallback < entryMid ? fallback : undefined);

        stop = {
            price: fmtPrice(inv),
            distancePct: Math.round(distancePct * 100) / 100,
            method,
            fallbackPrice: fallbackPrice ? fmtPrice(fallbackPrice) : undefined,
            source: 'L4_INVALIDATION',
            evidenceNote: opportunity?.invalidation_note || undefined,
        };
    }

    // ── Entry sources (confluent) ───────────────────────
    const entrySources: TradePlan['entrySources'] = (opportunity?.confluent_entry_levels ?? [])
        .slice(0, 4)
        .map((l) => ({
            tag: tagFromSources(l.sources),
            price: fmtPrice(l.price),
            strength: l.strength,
        }));

    // ── Actionability ──────────────────────────────────
    const tfConsidered = analysis?.timeframes_considered ?? 0;
    const actionable =
        opportunity != null &&
        tfConsidered >= 1 &&
        setupScore >= 30 &&
        ready !== 'STAND_ASIDE' &&
        stop != null &&
        entryMid > 0;
    let actionabilityReason = '';
    if (!actionable) {
        if (!opportunity) actionabilityReason = 'Awaiting L4 opportunity matrix';
        else if (tfConsidered < 1) actionabilityReason = 'Awaiting cross-TF consensus';
        else if (setupScore < 30) actionabilityReason = `Setup score ${setupScore} below threshold`;
        else if (ready === 'STAND_ASIDE') actionabilityReason = 'Trade readiness: STAND ASIDE';
        else if (stop == null) actionabilityReason = 'No invalidation level';
        else actionabilityReason = 'Awaiting entry zone';
    } else {
        actionabilityReason = 'Actionable setup';
    }

    return {
        symbol,
        direction,
        setupType,
        setupScore,
        setupQuality,
        timeHorizon,
        readiness: ready,
        confidencePct,
        rrRatio,
        riskDiscount: Math.round(clampPct(overallRisk ?? 0)),
        entryMid,
        entryZone: {
            low: entryZone?.low ?? 0,
            high: entryZone?.high ?? 0,
        },
        entrySources,
        targets,
        stop,
        entryGuidance: advisory?.entry_guidance ?? '—',
        exitGuidance: advisory?.exit_guidance ?? '—',
        targetStrategy: advisory?.target_strategy ?? '—',
        protectionStrategy: advisory?.protection_strategy ?? '—',
        contributors: decisionContext?.contributing_indicators ?? [],
        actionable,
        actionabilityReason,
    };
}

// ───────────────────────────── helpers ─────────────────────────────

function sortedConfluents(
    arr: ConfluentLevel[] | undefined | null,
    dir: 'asc' | 'desc',
): ConfluentLevel[] {
    if (!arr) return [];
    return [...arr].sort((a, b) => dir === 'desc' ? b.price - a.price : a.price - b.price);
}

function tagFromSources(sources: string[]): SourceTag {
    for (const s of sources) {
        if (s === 'FIBONACCI') return 'FIB';
        if (s === 'VOLUME_PROFILE') return 'VP';
        if (s === 'PIVOT_POINTS') return 'PP';
        if (s === 'SUPPORT_RESISTANCE') return 'SR';
        if (s === 'LIQUIDITY_CLUSTER') return 'LIQ';
        if (s === 'ATR_FALLBACK') return 'ATR';
    }
    return 'NONE';
}

function fibValues(tf: TimeframeTelemetry | undefined): { ext1618: number | null; ext2618: number | null } {
    if (!tf) return { ext1618: null, ext2618: null };
    const v = tf.indicators?.['fibonacci']?.values as Record<string, number | undefined> | undefined;
    if (!v) return { ext1618: null, ext2618: null };
    return {
        ext1618: typeof v['ext_1618'] === 'number' ? v['ext_1618'] : null,
        ext2618: typeof v['ext_2618'] === 'number' ? v['ext_2618'] : null,
    };
}

// ───────────────────────────── console wiring ─────────────────────────────

export interface ConsoleBracket {
    label: 'TP1' | 'TP2' | 'TP3' | 'SL';
    price: number;
    sizePct: number;
}

export function planToConsoleBrackets(plan: TradePlan): ConsoleBracket[] {
    const out: ConsoleBracket[] = [];
    for (const t of plan.targets) {
        out.push({ label: t.label as 'TP1' | 'TP2' | 'TP3', price: t.price, sizePct: t.sizePct });
    }
    if (plan.stop) {
        out.push({ label: 'SL', price: plan.stop.price, sizePct: plan.stop.distancePct });
    }
    return out;
}
