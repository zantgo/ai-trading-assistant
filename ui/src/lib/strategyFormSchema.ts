// strategyFormSchema — curated metadata for the schema-driven strategy
// editor (v10.1).
//
// The strategy JSON is still the single source of truth; this module only
// decorates the generic form renderer with human labels, units, ranges and
// enum options. Every field NOT listed here still renders (with an
// auto-generated label) — the editor has the same power as the raw JSON,
// just typed controls instead of textareas.

export interface FieldMeta {
    label?: string;
    help?: string;
    unit?: string;
    min?: number;
    max?: number;
    step?: number;
    options?: string[];
}

/** Keyed by JSON path relative to the strategy ROOT (`tae.risk.sl_mode`).
 *  `*` matches any array index. */
export const STRATEGY_FORM_SCHEMA: Record<string, FieldMeta> = {
    'tae.enabled': { label: 'TAE enabled', help: 'Master switch for the automation executor.' },
    'tae.allocation_pct': { label: 'Allocation', unit: '% of equity', min: 1, max: 100, step: 1 },
    'tae.min_net_rr': { label: 'Min net R:R', help: 'Entries below this fee-adjusted reward/risk are refused.', min: 0, max: 20, step: 0.1 },
    'tae.max_open_positions': { label: 'Max open positions', min: 1, max: 100, step: 1 },
    'tae.entry_mode': {
        label: 'Entry mode',
        options: ['zone_midpoint', 'zone_edge', 'zone_any', 'market_on_ready', 'chase'],
        help: 'Where the entry order rests relative to the target zone.',
    },
    'tae.instant_fill_policy': { label: 'Instant fill policy', options: ['take_better', 'cancel'] },
    'tae.invalidate_on': { label: 'Invalidate on', options: ['direction_flip', 'setup_gone'] },
    'tae.spread_gate_bps': { label: 'Spread gate', unit: 'bps', min: 0, max: 100, step: 0.5 },
    'tae.max_setup_age_bars': { label: 'Max setup age', unit: 'bars', min: 1, max: 500, step: 1 },
    'tae.reentry_cooldown_bars': { label: 'Re-entry cooldown', unit: 'bars', min: 0, max: 500, step: 1 },
    'tae.confirmation_bars': { label: 'Confirmation bars', help: 'Hold before submitting the entry.', min: 0, max: 50, step: 1 },
    'tae.pending_entry_expiry_bars': { label: 'Pending entry expiry', unit: 'bars', min: 1, max: 500, step: 1 },
    'tae.chase_max_atr': { label: 'Chase max ATR', help: 'Market-chase only within this distance past the zone edge.', min: 0, max: 10, step: 0.1 },
    'tae.chase_score_floor': { label: 'Chase score floor', min: 0, max: 100, step: 1 },
    'tae.tp_placement': { label: 'TP placement', options: ['zone_near_edge', 'zone_midpoint', 'zone_far_edge'] },
    'tae.sl_mode': { label: 'SL mode', options: ['invalidation', 'invalidation_padded', 'atr_anchored'] },
    'tae.min_sl_atr': { label: 'Min SL distance', unit: '× ATR', min: 0, max: 10, step: 0.1 },
    'tae.min_reprice_delta_atr': { label: 'Min reprice delta', unit: '× ATR', min: 0, max: 5, step: 0.1 },
    'tae.slippage_bps': { label: 'Slippage', unit: 'bps', help: 'Deterministic fill cost on top of the half-spread (paper + backtests).', min: 0, max: 100, step: 0.5 },
    'tae.direction_policy': { label: 'Direction policy', options: ['both', 'long_only', 'short_only'] },
    'tae.setup_gone_policy': { label: 'Setup-gone posture', options: ['balanced', 'strict', 'risky'] },
    'tae.signal_exit': { label: 'Signal exit', options: ['market', 'pullback'] },
    'tae.sizing.vol_scale.mode': { label: 'Vol-scale mode', options: ['auto', 'fixed'] },
    'tae.sizing.vol_scale.override_factor': { label: 'Vol-scale override', min: 0.1, max: 10, step: 0.05 },
    'tae.sizing.step_down.after_losses': { label: 'After consecutive losses', min: 1, max: 20, step: 1 },
    'tae.sizing.step_down.reduce_pct': { label: 'Step-down reduction', unit: '%', min: 0, max: 100, step: 1 },
    'tae.risk.trailing.atr_mult': { label: 'Trailing ATR mult', min: 0.5, max: 10, step: 0.1 },
    'pae.verdict.alpha': { label: 'Alpha (α)', min: 0.001, max: 0.5, step: 0.001 },
    'pae.verdict.monte_carlo_runs': { label: 'Monte Carlo runs', min: 100, max: 1_000_000, step: 1000 },
    'pae.verdict.min_trades_for_verdict': { label: 'Min trades for verdict', min: 1, max: 1000, step: 1 },
    'pae.risk_math.risk_free_rate_pct': { label: 'Risk-free rate', unit: '% / year', min: 0, max: 25, step: 0.05 },
    'l4.costs.taker_fee_bps': { label: 'Taker fee', unit: 'bps', min: 0, max: 200, step: 0.5 },
    'l4.costs.slippage_bps': { label: 'Slippage', unit: 'bps', min: 0, max: 200, step: 0.5 },
    'l4.costs.funding_bps': { label: 'Funding', unit: 'bps', min: 0, max: 200, step: 0.5 },
};

/** Humanize a JSON key: `sl_mode` → `SL mode`, `min_sl_atr` → `Min SL atr`. */
export function humanLabel(key: string): string {
    const words = key.replaceAll('_', ' ').split(' ');
    if (words.length === 0) return key;
    const first = words[0];
    if (/^[A-Z]+$/.test(first) && first.length <= 4) {
        words[0] = first; // keep acronyms (SL, TP, ATR, RSI…)
    } else {
        words[0] = first.charAt(0).toUpperCase() + first.slice(1);
    }
    return words.join(' ');
}

/** Resolve curated metadata for a field path relative to the strategy
 *  root. Array indices become `*` for lookup; the last segment is also
 *  tried without parent context (fallback). */
export function fieldMeta(path: (string | number)[]): FieldMeta | null {
    const dotted = path
        .map((p) => (typeof p === 'number' ? '*' : p))
        .join('.');
    const direct = STRATEGY_FORM_SCHEMA[dotted];
    if (direct) return direct;
    // Fallback: last-segment-only lookup (tae.* etc. reuse).
    const last = path[path.length - 1];
    if (typeof last === 'string') {
        const key = `tae.${last}`;
        const viaTae = STRATEGY_FORM_SCHEMA[key];
        if (viaTae) return viaTae;
    }
    return null;
}
