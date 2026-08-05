import type { PriceRange } from '../types';

export type NoValueReason =
    | 'geometry_inverted'
    | 'sl_at_entry'
    | 'sl_inside_entry'
    | 'target_on_wrong_side'
    | 'no_valid_bracket'
    | 'inactive_side';

export type RrStatus =
    | { kind: 'value'; rr: number }
    | { kind: 'none'; reason: NoValueReason }
    | { kind: 'error'; message: string };

export interface RiskRewardDisplay {
    status: RrStatus;
    /** Formatted string for display, e.g. "R:R 1 : 2.00" or "R:R —" or "R:R ⚠" */
    display: string;
    /** Tooltip with worked-example math, reason, or error message */
    tooltip: string;
    /** CSS badge class: 'ok' = green, 'none' = muted, 'error' = warning */
    badgeClass: 'ok' | 'none' | 'error';
}

const REASONS: Record<NoValueReason, string> = {
    geometry_inverted: 'No valid bracket — target & stop-loss on same side of entry',
    sl_at_entry: 'Stop-loss is at entry midpoint — zero risk',
    sl_inside_entry: 'Stop-loss is inside the entry zone',
    target_on_wrong_side: 'Target is on the wrong side of current price',
    no_valid_bracket: 'No confluent levels and no synthetic fallback',
    inactive_side: 'Setup is on the inactive side for the current bias',
};

export function computeRiskReward(
    entry: PriceRange,
    target: PriceRange,
    invalidation: number,
    side: 'LONG' | 'SHORT',
    close: number,
): RiskRewardDisplay {
    if (!entry || !target || invalidation <= 0 || close <= 0) {
        return {
            status: { kind: 'error', message: 'Missing or non-positive entry/target/invalidation/close' },
            display: 'R:R \u26a0',
            tooltip: 'Unable to compute R:R — missing or non-positive values',
            badgeClass: 'error',
        };
    }
    if (entry.low <= 0 || entry.high <= 0 || target.low <= 0 || target.high <= 0) {
        return {
            status: { kind: 'error', message: 'Non-positive entry or target zone values' },
            display: 'R:R \u26a0',
            tooltip: 'Unable to compute R:R — non-positive zone values',
            badgeClass: 'error',
        };
    }

    // SL inside entry zone
    if (invalidation >= entry.low && invalidation <= entry.high) {
        return {
            status: { kind: 'none', reason: 'sl_inside_entry' },
            display: 'R:R \u2014',
            tooltip: REASONS.sl_inside_entry,
            badgeClass: 'none',
        };
    }

    const entryMid = (entry.low + entry.high) / 2;
    const targetMid = (target.low + target.high) / 2;

    // SL at entry mid
    const riskDir = entryMid - invalidation;
    if (Math.abs(riskDir) < 0.0001 * Math.max(Math.abs(entryMid), 1)) {
        return {
            status: { kind: 'none', reason: 'sl_at_entry' },
            display: 'R:R \u2014',
            tooltip: REASONS.sl_at_entry,
            badgeClass: 'none',
        };
    }

    // Target on wrong side
    if (side === 'LONG' && targetMid <= close) {
        return {
            status: { kind: 'none', reason: 'target_on_wrong_side' },
            display: 'R:R \u2014',
            tooltip: REASONS.target_on_wrong_side,
            badgeClass: 'none',
        };
    }
    if (side === 'SHORT' && targetMid >= close) {
        return {
            status: { kind: 'none', reason: 'target_on_wrong_side' },
            display: 'R:R \u2014',
            tooltip: REASONS.target_on_wrong_side,
            badgeClass: 'none',
        };
    }

    const reward = side === 'LONG' ? targetMid - entryMid : entryMid - targetMid;
    const risk = side === 'LONG' ? entryMid - invalidation : invalidation - entryMid;

    // Geometry inverted
    if (reward <= 0 || risk <= 0) {
        return {
            status: { kind: 'none', reason: 'geometry_inverted' },
            display: 'R:R \u2014',
            tooltip: REASONS.geometry_inverted,
            badgeClass: 'none',
        };
    }

    const rr = reward / risk;
    if (!Number.isFinite(rr)) {
        return {
            status: { kind: 'error', message: 'Non-finite R:R (division by zero or overflow)' },
            display: 'R:R \u26a0',
            tooltip: 'Unable to compute R:R — non-finite result',
            badgeClass: 'error',
        };
    }

    const normalized = normalizeRiskTo1(rr);
    const math = buildTooltip(entry, target, invalidation, side, risk, reward, rr);
    return {
        status: { kind: 'value', rr },
        display: `R:R 1 : ${normalized}`,
        tooltip: math,
        badgeClass: 'ok',
    };
}

export function discountRiskReward(
    rr: RiskRewardDisplay,
    overallRiskPct: number,
): RiskRewardDisplay {
    if (rr.status.kind !== 'value') return rr;
    const discounted = rr.status.rr * (1 - overallRiskPct / 100);
    if (discounted <= 0) {
        return {
            status: { kind: 'none', reason: 'no_valid_bracket' },
            display: 'R:R \u2014',
            tooltip: `No valid R:R after ${overallRiskPct.toFixed(0)}% risk discount`,
            badgeClass: 'none',
        };
    }
    const normalized = normalizeRiskTo1(discounted);
    return {
        status: { kind: 'value', rr: discounted },
        display: `R:R 1 : ${normalized}`,
        tooltip: `${rr.tooltip}\n\nDiscounted by overall risk ${overallRiskPct.toFixed(0)}%`,
        badgeClass: 'ok',
    };
}

function normalizeRiskTo1(rr: number): string {
    if (rr >= 9.99) return '9.99+';
    if (rr >= 5) return rr.toFixed(1);
    return rr.toFixed(2);
}

function buildTooltip(
    entry: PriceRange,
    target: PriceRange,
    inv: number,
    side: string,
    risk: number,
    reward: number,
    rr: number,
): string {
    const entryMid = ((entry.low + entry.high) / 2).toFixed(0);
    const targetMid = ((target.low + target.high) / 2).toFixed(0);
    const invStr = inv.toFixed(0);
    const riskStr = risk.toFixed(0);
    const rewardStr = reward.toFixed(0);

    return [
        `Entry: $${entryMid} \u00b7 Stop: $${invStr} \u00b7 Target: $${targetMid}`,
        `Risk: $${riskStr} \u00b7 Reward: $${rewardStr}`,
        `Risk:Reward = 1:${(rr).toFixed(2)} \u00b7 side: ${side}`,
    ].join('\n');
}
