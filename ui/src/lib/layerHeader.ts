// LayerHeader — spec + builders for the canonical MME tab chrome (v7.0-prod).
//
// Every MME tab now renders the same single badge + meta chip rail +
// status pill header, eliminating the L3/L6 contradictory-verdict and
// the empty/zero/error ambiguity that previously plagued the UI.
//
// Architectural rule: this file is a **pure view-layer adapter**. The
// canonical matrices (Metrics, Alignment, Analysis, Opportunity, Risk,
// Decision, Overview) remain the single source of truth. The builders
// here map matrix fields to view tokens; if a future matrix version
// renames a field (e.g. `overall_label` → `dominant_bias`) update the
// corresponding builder function in this file only.

import { DASHBOARD_COLORS, biasColor, directionColor, riskDangerColor, scoreColor, rrColor } from './dashboardColors';
import { COLORS } from './scoreStyles';
import type {
    AdvisoryMatrix,
    AlignmentMatrix,
    AnalysisMatrix,
    DecisionContext,
    GlobalBias,
    MarketBias,
    OpportunityMatrix,
    OverviewMatrix,
    RiskMatrix,
    TimeframeTelemetry,
} from '../types';

// ── Discriminated state ─────────────────────────────────────────────────
//
// The four states are visually distinct so the operator can never confuse
// "data is zero" with "data is missing" with "data is broken". The CSS
// classes in LayerHeader.module.css consume these tags directly.

export type ValueState = 'valid' | 'neutral' | 'empty' | 'error';

// ── Spec types ──────────────────────────────────────────────────────────

export interface BadgeSpec {
    label: string;
    sublabel?: string;
    color: string;
    background: string;
    state: ValueState;
}

export interface MetaChipSpec {
    label: string;
    value: string;
    /** Text colour for the value when state is `valid`. Ignored otherwise. */
    color: string;
    state: ValueState;
}

export interface LayerHeaderSpec {
    layerNumber: 1 | 2 | 3 | 4 | 5 | 6 | 7;
    layerName: string;
    badge: BadgeSpec;
    meta: MetaChipSpec[];
    status: 'live' | 'stale' | 'error' | 'loading';
}

// ── Helpers ─────────────────────────────────────────────────────────────

const EMPTY_DASH = '\u2014';

/** Hex colour → rgba() string with a given alpha. */
export function hexToRgba(hex: string, alpha: number): string {
    const cleaned = hex.replace('#', '');
    if (cleaned.length !== 6) return `rgba(255,255,255,${alpha})`;
    const r = parseInt(cleaned.slice(0, 2), 16);
    const g = parseInt(cleaned.slice(2, 4), 16);
    const b = parseInt(cleaned.slice(4, 6), 16);
    if ([r, g, b].some((n) => Number.isNaN(n))) return `rgba(255,255,255,${alpha})`;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** Empty badge factory. `—` in grey italic. */
export function emptyBadge(): BadgeSpec {
    return {
        label: EMPTY_DASH,
        color: DASHBOARD_COLORS.inactive,
        background: 'transparent',
        state: 'empty',
    };
}

export interface ChipOptions {
    /** When `true`, render `0` in `valid` colour rather than the neutral
     * amber `neutral` tone. Used for risk scores where zero is "perfectly safe". */
    zeroIsGood?: boolean;
    /** Number of fractional digits to render for a numeric `rawValue`.
     *  Defaults to `2` for floats and `0` for integer-valued chips. */
    digits?: number;
}

/**
 * Create a `MetaChipSpec` with automatic state discrimination:
 *   - `null` / `undefined` / empty string → `empty` (grey italic `--`)
 *   - numeric `0` → `neutral` (amber) unless `zeroIsGood` is set
 *   - everything else → `valid` (semantic colour from `colorFn(numeric)`)
 */
export function chip(
    label: string,
    rawValue: number | string | null | undefined,
    numericValue: number | null | undefined,
    colorFn: ((n: number) => string) | null,
    isCount = false,
    opts: ChipOptions = {},
): MetaChipSpec {
    if (rawValue === null || rawValue === undefined || rawValue === '') {
        return { label, value: EMPTY_DASH, color: DASHBOARD_COLORS.inactive, state: 'empty' };
    }
    if (typeof numericValue === 'number' && numericValue === 0) {
        if (opts.zeroIsGood) {
            // Risk-zero: green "0" — meaningful good news.
            const z = isCount ? '0' : formatChipNumber(0, opts.digits);
            return { label, value: z, color: '#22c55e', state: 'valid' };
        }
        return { label, value: isCount ? '0' : String(rawValue), color: COLORS.neutral, state: 'neutral' };
    }
    const color = colorFn && numericValue != null ? colorFn(numericValue) : COLORS.text;
    const display =
        isCount && typeof rawValue === 'number'
            ? String(rawValue)
            : typeof rawValue === 'number'
                ? formatChipNumber(rawValue, opts.digits)
                : String(rawValue);
    return { label, value: display, color, state: 'valid' };
}

function formatChipNumber(n: number, digits?: number): string {
    if (digits === undefined) {
        // Default: integers get 0 decimals, floats get 2 (rounded).
        digits = Number.isInteger(n) ? 0 : 2;
    }
    return n.toFixed(digits);
}

/** Map a `QualityLevel` (`Poor`/`Weak`/`Average`/`Good`/`Excellent`) to a colour. */
export function qualityColor(q: string | null | undefined): string {
    if (!q) return DASHBOARD_COLORS.inactive;
    switch (q) {
        case 'Excellent':
            return '#22c55e';
        case 'Good':
            return '#4ade80';
        case 'Average':
            return '#f59e0b';
        case 'Weak':
            return '#fbbf24';
        default:
            return '#ef4444';
    }
}

/** Pretty-print a PascalCase or SCREAMING_SNAKE_CASE enum token for display. */
export function prettifyEnum(value: string | null | undefined): string {
    if (!value) return EMPTY_DASH;
    return String(value).replace(/_/g, ' ').replace(/([a-z])([A-Z])/g, '$1 $2').trim();
}

export function signalCount(tf: TimeframeTelemetry | null | undefined): number {
    if (!tf) return 0;
    let n = 0;
    const inds = tf.indicators ?? {};
    for (const k in inds) n += (inds[k]?.signals ?? []).length;
    return n;
}

/** Infer a LayerHeader `status` from a TimeframeTelemetry + WS state. */
export function tfStatusFrom(
    tf: TimeframeTelemetry | null | undefined,
    wss: { wsMicro: WebSocket | null } | null | undefined,
): 'live' | 'stale' | 'loading' | 'error' {
    if (wss?.wsMicro && wss.wsMicro.readyState === WebSocket.OPEN && !tf) return 'loading';
    if (wss?.wsMicro && wss.wsMicro.readyState === WebSocket.CLOSED) return 'error';
    if (!tf) return 'loading';
    if (!tf.isCompleted) return 'loading';
    if (tf.pipelineState === 'FAILED') return 'error';
    if (tf.pipelineState === 'STALE') return 'stale';
    return 'live';
}

// ── Builders ────────────────────────────────────────────────────────────

// L1 — Metrics (per-timeframe)
// Real wire shape: `tf.context.overall_label`, `tf.context.overall_score`,
// `tf.context.regime`. The headline number is the per-TF overall score
// (used for the chip). The badge reads the regime label, which already
// encodes bias direction (Trend Bull / Trend Bear / Range / Expansion / …).
export function buildL1MetricsHeader(tf: TimeframeTelemetry | null | undefined): LayerHeaderSpec {
    const ctx = tf?.context ?? null;
    const label = ctx?.overall_label ?? null;
    const regime = ctx?.regime ?? null;
    const score = ctx?.overall_score ?? null;

    if (!tf || !label) {
        return {
            layerNumber: 1,
            layerName: 'Metrics',
            badge: emptyBadge(),
            meta: [],
            status: 'loading',
        };
    }
    const badgeLabel = prettifyEnum(label);
    const badgeColor = biasColor(label);
    const meta: MetaChipSpec[] = [
        chip('Score', score, score, scoreColor),
        chip('Signals', signalCount(tf), signalCount(tf), null, true),
    ];
    // Hide regime from chips when it's already implied by the badge label.
    const regimeImplied =
        (regime === 'TRENDING' && (label === 'BULLISH' || label === 'BEARISH')) ||
        (regime === 'RANGE' && label === 'NEUTRAL');
    if (regime && !regimeImplied) {
        meta.push(chip('Regime', prettifyEnum(regime), null, () => COLORS.textMuted));
    }
    return {
        layerNumber: 1,
        layerName: 'Metrics',
        badge: {
            label: badgeLabel,
            sublabel: regimeImplied ? undefined : regime ? prettifyEnum(regime) : undefined,
            color: badgeColor,
            background: hexToRgba(badgeColor, 0.08),
            state: 'valid',
        },
        meta,
        status: tf.isCompleted ? 'live' : 'loading',
    };
}

// L1 — Metrics (Multi-Timeframe synthetic header).
// Renders an MTF-specific spec when the operator switches the L1 sidebar
// to "MTF". The badge is fixed (`MTF SYNC`); the chips show cross-TF
// agreement + presence so the operator can compare against L2.
export function buildL1MtfHeader(alignment: AlignmentMatrix | null | undefined, overviewSync: string | null | undefined = null): LayerHeaderSpec {
    if (!alignment) {
        return {
            layerNumber: 1,
            layerName: 'Metrics · MTF',
            badge: emptyBadge(),
            meta: [],
            status: 'loading',
        };
    }
    const tfs = alignment.timeframes_present ?? 0;
    const agreement = alignment.trend_agreement_pct ?? 0;
    const cross = alignment.signal_cross_tf_count ?? 0;
    // v7.0-prod — colour the MTF SYNC badge by the cross-TF verdict so it
    // sits inside the strict 4-colour vocabulary (green for long, red for
    // short, amber for sideways/neutral, gray for no data). Cyan used to
    // be the sentinel colour for the cross-TF view; it now resolves
    // through the same `biasColor` mapping as the L2 badge.
    const mtfColor = biasColor(alignment.mtf_overall_label ?? null);
    return {
        layerNumber: 1,
        layerName: 'Metrics · MTF',
        badge: {
            label: 'MTF SYNC',
            sublabel: overviewSync ? prettifyEnum(overviewSync) : undefined,
            color: mtfColor,
            background: hexToRgba(mtfColor, 0.08),
            state: 'valid',
        },
        meta: [
            chip('TFs', `${tfs}/4`, tfs, null, true),
            chip('Agreement', `${agreement.toFixed(0)}%`, agreement, scoreColor),
            chip('Cross', cross, cross, null, true),
        ],
        status: tfs >= 3 ? 'live' : tfs >= 1 ? 'stale' : 'loading',
    };
}

// L2 — Alignment (cross-TF)
export function buildL2AlignmentHeader(a: AlignmentMatrix | null | undefined): LayerHeaderSpec {
    const label = a?.mtf_overall_label ?? null;
    const score = a?.mtf_overall_score ?? null;
    const agreement = a?.trend_agreement_pct ?? null;
    const tfs = a?.timeframes_present ?? null;

    if (!a || !label) {
        return { layerNumber: 2, layerName: 'Alignment', badge: emptyBadge(), meta: [], status: 'loading' };
    }
    return {
        layerNumber: 2,
        layerName: 'Alignment',
        badge: {
            label: prettifyEnum(label),
            color: biasColor(label),
            background: hexToRgba(biasColor(label), 0.08),
            state: 'valid',
        },
        meta: [
            chip('Score', score, score, scoreColor),
            chip('Agreement', agreement != null ? `${agreement.toFixed(0)}%` : null, agreement, scoreColor),
            chip('TFs', tfs != null ? `${tfs}/4` : null, tfs, null, true),
        ],
        status: 'live',
    };
}

// L3 — Analysis (no longer leaks into L4-L6).
// Regime chip is suppressed when redundant with the bias badge
// (e.g. bias=BULLISH ∧ regime=TRENDING_BULL is one fact, not two).
export function buildL3AnalysisHeader(a: AnalysisMatrix | null | undefined): LayerHeaderSpec {
    const bias = a?.bias ?? null;
    const regime = a?.market_regime ?? null;
    const quality = a?.market_quality ?? null;
    const stateConfidence = a?.state_confidence ?? null;
    const confidencePct = stateConfidence != null ? Math.round(stateConfidence * 100) : null;

    if (!a || !bias) {
        return { layerNumber: 3, layerName: 'Analysis', badge: emptyBadge(), meta: [], status: 'loading' };
    }
    const redundant =
        (bias === 'Bullish' && regime === 'TRENDING_BULL') ||
        (bias === 'Bearish' && regime === 'TRENDING_BEAR') ||
        (bias === 'StrongBullish' && regime === 'TRENDING_BULL') ||
        (bias === 'StrongBearish' && regime === 'TRENDING_BEAR') ||
        (bias === 'Neutral' && regime === 'RANGE');

    const meta: MetaChipSpec[] = [
        chip('Quality', quality, null, () => qualityColor(quality)),
        chip('Confidence', confidencePct != null ? `${confidencePct}%` : null, confidencePct, scoreColor),
    ];
    if (regime && !redundant) {
        meta.push(chip('Regime', prettifyEnum(regime), null, () => COLORS.textMuted));
    }
    return {
        layerNumber: 3,
        layerName: 'Analysis',
        badge: {
            label: prettifyEnum(bias),
            color: biasColor(bias),
            background: hexToRgba(biasColor(bias), 0.08),
            state: 'valid',
        },
        meta,
        status: 'live',
    };
}

// L4 — Opportunity (L4 only — never bleeds L3 bias or L5 risk).
// When no qualifying setup exists we surface `NO CLEAR SETUP` in amber
// (operator rule: zero opportunity is neutral, not good).
export function buildL4OpportunityHeader(
    o: OpportunityMatrix | null | undefined,
    bias: MarketBias | null | undefined = null,
): LayerHeaderSpec {
    const type = o?.primary_opportunity ?? null;
    const score = o?.opportunity_score ?? null;
    const quality = o?.setup_quality ?? null;
    const horizon = o?.time_horizon ?? null;
    const noClear = !type || type === 'NoClearOpportunity';

    if (noClear) {
        return {
            layerNumber: 4,
            layerName: 'Opportunity',
            badge: {
                label: 'NO CLEAR SETUP',
                color: COLORS.neutral,
                background: hexToRgba(COLORS.neutral, 0.08),
                state: 'neutral',
            },
            meta: [],
            status: 'live',
        };
    }

    const longRr = o?.long_expected_rr_internal ?? 0;
    const shortRr = o?.short_expected_rr_internal ?? 0;
    const dir: 'LONG' | 'SHORT' = longRr >= shortRr ? 'LONG' : 'SHORT';
    // Bias overrides RR-based direction when both LONG and SHORT are
    // available — the macro L3 bias is the operator's authoritative
    // directional call, not the per-side geometric computation.
    const effectiveDir: 'LONG' | 'SHORT' =
        bias === 'Bearish' || bias === 'StrongBearish' ? 'SHORT'
        : bias === 'Bullish' || bias === 'StrongBullish' ? 'LONG'
        : dir;
    const activeRr = effectiveDir === 'LONG' ? longRr : shortRr;

    return {
        layerNumber: 4,
        layerName: 'Opportunity',
        badge: {
            label: prettifyEnum(type),
            sublabel: quality ? prettifyEnum(quality) : undefined,
            color: directionColor(effectiveDir),
            background: hexToRgba(directionColor(effectiveDir), 0.08),
            state: 'valid',
        },
        meta: [
            chip('Score', score, score, scoreColor),
            chip('R:R', activeRr > 0 ? `1:${activeRr.toFixed(2)}` : null, activeRr, rrColor),
            chip('Horizon', horizon ? prettifyEnum(horizon) : null, null, () => COLORS.textMuted),
        ],
        status: 'live',
    };
}

// L5 — Risk. Three-colour badge palette (v7.0-prod):
//   gray   — no risk matrix loaded (emptyBadge)
//   blue   — medium-to-low risk  (score < 50)
//   amber  — medium-to-high risk (score >= 50)
//
// Risk has no intrinsic trade direction (it is a magnitude measure,
// not a verdict), so green is reserved exclusively for bullish setups
// and red is reserved exclusively for bearish setups. The numeric
// score chip INSIDE the meta rail still uses `riskDangerColor()` for
// its detailed magnitude banding, since chips communicate severity,
// not direction.
export function buildL5RiskHeader(r: RiskMatrix | null | undefined): LayerHeaderSpec {
    const overall = r?.overall_risk ?? null;
    const score = overall?.score ?? null;
    const level = overall?.level ?? null;
    const state = overall?.state ?? null;
    const activeDimCount = countActiveRiskDimensions(r);
    if (!overall) {
        return { layerNumber: 5, layerName: 'Risk', badge: emptyBadge(), meta: [], status: 'loading' };
    }
    const color = score != null && score < 50 ? '#22d3ee' : '#f59e0b';
    const background = hexToRgba(color, 0.08);
    return {
        layerNumber: 5,
        layerName: 'Risk',
        badge: {
            label: prettifyEnum(level),
            sublabel: state ? prettifyEnum(state) : undefined,
            color,
            background,
            state: 'valid',
        },
        meta: [
            chip('Score', score, score, riskDangerColor, false, { zeroIsGood: true }),
            chip('Dimensions', `${activeDimCount}/8`, activeDimCount, null, true),
        ],
        status: 'live',
    };
}

export function countActiveRiskDimensions(r: RiskMatrix | null | undefined): number {
    if (!r) return 0;
    const dims = [
        r.market_risk,
        r.volatility_risk,
        r.execution_liquidity_risk,
        r.structure_risk,
        r.momentum_risk,
        r.signal_risk,
        r.execution_risk,
        r.cascade_risk,
    ];
    return dims.filter((d) => !!d).length;
}

// L6 — Recommendation (never reads L3 bias). The badge is the operator's
// authoritative verdict; L3 is only an input. When `rank.top === HOLD`
// the chip rail reports N/A rather than 0.00 (mirrors the existing
// Recommendation panel rule).
export function buildL6DecisionHeader(input: {
    rank: { top: 'LONG' | 'SHORT' | 'HOLD'; headline: { state: string } };
    decisionContext: DecisionContext | null | undefined;
    advisory: AdvisoryMatrix | null | undefined;
}): LayerHeaderSpec {
    const { rank, decisionContext, advisory } = input;
    const confidence = advisory?.confidence_assessment ?? null;
    const rr = decisionContext?.expected_reward_risk_ratio ?? 0;
    const stance = advisory?.market_stance ?? null;
    const readiness = decisionContext?.trade_readiness ?? null;

    let label: string = rank.top;
    if (rank.headline.state === 'STAND_ASIDE' || decisionContext?.trade_readiness === 'STAND_ASIDE') {
        label = 'STAND ASIDE';
    } else if (rank.top === 'HOLD') {
        label = 'HOLD';
    }
    const color =
        label === 'LONG' ? DASHBOARD_COLORS.bullish
        : label === 'SHORT' ? DASHBOARD_COLORS.bearish
        : COLORS.neutral;
    const isHypothesis = label === 'HOLD' || label === 'STAND ASIDE';

    if (!rank || (!advisory && !decisionContext)) {
        return { layerNumber: 6, layerName: 'Recommendation', badge: emptyBadge(), meta: [], status: 'loading' };
    }

    const meta: MetaChipSpec[] = [
        chip('Confidence', confidence != null ? `${Math.round(confidence)}%` : null, confidence, scoreColor),
        isHypothesis
            ? chip('R:R', 'N/A', null, null)
            : chip('R:R', rr > 0 ? `1:${rr.toFixed(2)}` : null, rr, rrColor),
    ];
    if (stance && stance !== 'Neutral' && stance !== 'Avoid') {
        meta.push(chip('Stance', prettifyEnum(stance), null, () => COLORS.textMuted));
    }
    return {
        layerNumber: 6,
        layerName: 'Recommendation',
        badge: {
            label,
            sublabel: readiness && readiness !== 'READY' ? prettifyEnum(readiness) : undefined,
            color,
            background: hexToRgba(color, 0.08),
            state: 'valid',
        },
        meta,
        status: 'live',
    };
}

// L7 — Overview (system-wide). `systemic_risk_score = 0` is green.
// Status is sourced from the L7 fetch state (live when fresh, stale
// after >2× polling interval, error when the last attempt failed).
export function buildL7OverviewHeader(
    overview: OverviewMatrix | null | undefined,
    fetchState: { lastSuccessMs: number | null; lastErrorMs: number | null; now: number; pollIntervalMs: number },
): LayerHeaderSpec {
    const bias = overview?.global_market_bias ?? null;
    const health = overview?.market_health ?? null;
    const risk = overview?.systemic_risk_score ?? null;
    const count = overview?.instance_count ?? null;
    const sync = overview?.market_synchronization ?? null;

    let status: LayerHeaderSpec['status'] = 'loading';
    if (fetchState.lastErrorMs != null && (fetchState.lastSuccessMs == null || fetchState.lastErrorMs > fetchState.lastSuccessMs)) {
        status = 'error';
    } else if (fetchState.lastSuccessMs != null) {
        const ageMs = fetchState.now - fetchState.lastSuccessMs;
        status = ageMs > fetchState.pollIntervalMs * 2 ? 'stale' : 'live';
    }

    if (!overview || !bias) {
        return {
            layerNumber: 7,
            layerName: 'Overview',
            badge: emptyBadge(),
            meta: [],
            status,
        };
    }
    return {
        layerNumber: 7,
        layerName: 'Overview',
        badge: {
            label: prettifyEnum(bias as GlobalBias),
            sublabel: health ? prettifyEnum(health) : undefined,
            color: biasColor(bias),
            background: hexToRgba(biasColor(bias), 0.08),
            state: 'valid',
        },
        meta: [
            chip('Instances', count, count, null, true),
            chip('Sys Risk', risk, risk, riskDangerColor, false, { zeroIsGood: true }),
            chip('Sync', sync ? prettifyEnum(sync) : null, null, () => COLORS.textMuted),
        ],
        status,
    };
}
