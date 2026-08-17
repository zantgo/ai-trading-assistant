// OPPORTUNITY SUMMARY — natural-language paragraph generator (v7.0).
//
// Single source of truth for the Opportunities panel's top summary card
// AND the opportunity tab export (`summary` block) so screen and
// clipboard can never disagree. Template-driven from the wire matrix —
// no fabricated numbers, no backend change.

import type { OpportunityMatrix } from '../types';

/** Kicker label rendered by SummaryCard on the Opportunities panel. */
export const OPPORTUNITY_SUMMARY_LABEL = 'OPPORTUNITY SUMMARY';

const CONVICTION: Array<{ min: number; word: string }> = [
    { min: 85, word: 'high-conviction' },
    { min: 70, word: 'strong-conviction' },
    { min: 50, word: 'moderate-conviction' },
    { min: 30, word: 'low-conviction' },
    { min: 0, word: 'low-conviction' },
];

/** Prettify a PascalCase opportunity token for prose ("TrendContinuation"
 *  → "trend-continuation"). */
export function opportunityProseLabel(token: string): string {
    const cleaned = String(token || '')
        .replace(/([A-Z])/g, ' $1')
        .trim()
        .toLowerCase()
        .replace(/\s+/g, '-');
    return cleaned || 'opportunity';
}

function convictionWord(score: number | null | undefined): string {
    if (score == null || !isFinite(score)) return 'uncertain-conviction';
    for (const tier of CONVICTION) {
        if (score >= tier.min) return tier.word;
    }
    return 'uncertain-conviction';
}

function horizonWord(horizon: string | null | undefined): string {
    const h = String(horizon ?? '').toLowerCase();
    if (!h) return 'the current horizon';
    if (h.includes('intraday') || h.includes('scalp')) return 'an intraday horizon';
    if (h.includes('swing')) return 'a swing horizon';
    if (h.includes('position')) return 'a position horizon';
    return `a ${h} horizon`;
}

/**
 * Build the natural-language OPPORTUNITY SUMMARY paragraph.
 *
 * Grammar (three sentences when real data exists):
 *   1. "The market is in a {conviction} {opportunity} phase."
 *   2. "Setup quality is rated {quality} over {horizon}."
 *   3. "{n} candidate profile{s} evaluated, the strongest scoring
 *      {best} with {met}/{total} preconditions met."
 *
 * Awaiting fallback mirrors the other panels' initialization copy.
 */
export function buildOpportunitySummary(opportunity: OpportunityMatrix | null | undefined): string {
    if (!opportunity) {
        return 'Awaiting opportunity data — this summary will describe the active opportunity landscape once the opportunity matrix populates.';
    }
    const primary = opportunity.primary_opportunity ?? '';
    if (!primary || primary === 'NoClearOpportunity') {
        return 'No clear opportunity is present — the market is not offering an actionable setup across the evaluated profiles.';
    }
    const conviction = convictionWord(opportunity.opportunity_score);
    const kind = opportunityProseLabel(primary);
    const quality = opportunity.setup_quality ? String(opportunity.setup_quality) : 'unrated';
    const horizon = horizonWord(opportunity.time_horizon);
    const profiles = opportunity.profiles ?? [];
    const qualifying = profiles.filter((p) => p.preconditions_met > 0 && p.opportunity_type !== 'NoClearOpportunity');
    const best = qualifying.length > 0
        ? qualifying.reduce((a, b) => (b.score > a.score ? b : a))
        : null;

    const s1 = `The market is in a ${conviction} ${kind} phase.`;
    const s2 = `Setup quality is rated ${quality} over ${horizon}.`;
    const s3 = best
        ? `${profiles.length} candidate profile${profiles.length === 1 ? '' : 's'} evaluated, the strongest scoring ${best.score.toFixed(0)} with ${best.preconditions_met}/${best.preconditions_total} preconditions met.`
        : `${profiles.length} candidate profile${profiles.length === 1 ? '' : 's'} evaluated, none currently meeting their preconditions.`;
    return `${s1} ${s2} ${s3}`;
}
