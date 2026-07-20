// Level-kind classification — buckets the 14+ LevelTest-emitting indicators
// into 8 semantic categories so the Levels facet can group them by the
// kind of price structure they expose rather than by indicator.
//
// Categories (chosen to mirror how a trader thinks about price structure):
//   - Pivot       — Pivot Points (7 levels: pivot, R1-R3, S1-S3)
//   - Fibonacci   — Fibonacci retracement / extension levels
//   - SR          — Support / Resistance horizontal zones (sr_engine)
//   - Vwap        — VWAP + Anchored VWAP
//   - ChannelMid  — Bollinger middle, Donchian/Keltner/StdDev channel mids
//   - Ichimoku    — Ichimoku cloud edges (Tenkan, Kijun, Senkou A/B)
//   - VolumeNode  — Volume Profile HVN / LVN nodes
//   - SmcZone     — SMC FVG and Order Block zones
//   - Other       — Supertrend line (dynamic S/R), anything that doesn't
//                   fit the above

export type LevelKind =
    | 'Pivot'
    | 'Fibonacci'
    | 'SR'
    | 'Vwap'
    | 'ChannelMid'
    | 'Ichimoku'
    | 'VolumeNode'
    | 'SmcZone'
    | 'Other';

export interface LevelKindMeta {
    key: LevelKind;
    label: string;
    description: string;
    accent: string;
}

/** Visual order in the Levels facet accordion. */
export const LEVEL_KIND_ORDER: LevelKind[] = [
    'Pivot',
    'Fibonacci',
    'SR',
    'Vwap',
    'ChannelMid',
    'Ichimoku',
    'VolumeNode',
    'SmcZone',
    'Other',
];

export const LEVEL_KIND_META: Record<LevelKind, LevelKindMeta> = {
    Pivot:      { key: 'Pivot',      label: 'Pivot Points',  description: 'R1-R3, S1-S3, central pivot',          accent: '#60a5fa' },
    Fibonacci:  { key: 'Fibonacci',  label: 'Fibonacci',     description: 'Retracement / extension coefficients',  accent: '#a78bfa' },
    SR:         { key: 'SR',         label: 'Support / Resistance', description: 'Horizontal S/R zones (sr_engine)', accent: '#22d3ee' },
    Vwap:       { key: 'Vwap',       label: 'VWAP',          description: 'VWAP + anchored VWAP',                  accent: '#34d399' },
    ChannelMid: { key: 'ChannelMid', label: 'Channel Mid',   description: 'Bollinger/Donchian/Keltner/StdDev mid', accent: '#fbbf24' },
    Ichimoku:   { key: 'Ichimoku',   label: 'Ichimoku Cloud',description: 'Tenkan, Kijun, Senkou A/B edges',       accent: '#ec4899' },
    VolumeNode: { key: 'VolumeNode', label: 'Volume Profile',description: 'HVN / LVN nodes',                       accent: '#fb923c' },
    SmcZone:    { key: 'SmcZone',    label: 'SMC Zones',     description: 'Fair Value Gaps + Order Blocks',        accent: '#f472b6' },
    Other:      { key: 'Other',      label: 'Other',         description: 'Dynamic levels (e.g. Supertrend line)', accent: 'rgba(255,255,255,0.4)' },
};

/** Maps a producer indicator key to its level category. */
export function classifyLevelKey(indicatorKey: string): LevelKind {
    switch (indicatorKey) {
        case 'pivot_points':
            return 'Pivot';
        case 'fibonacci':
            return 'Fibonacci';
        case 'support_resistance':
            return 'SR';
        case 'vwap':
        case 'anchored_vwap':
            return 'Vwap';
        case 'bollinger':
        case 'donchian':
        case 'keltner':
        case 'stddev_channel':
            return 'ChannelMid';
        case 'ichimoku':
            return 'Ichimoku';
        case 'volume_profile':
            return 'VolumeNode';
        case 'smc_fvg':
        case 'smc_order_blocks':
            return 'SmcZone';
        case 'supertrend':
        default:
            return 'Other';
    }
}

/** Returns metadata for a level kind (fallback to Other). */
export function levelKindMeta(kind: LevelKind | string | undefined): LevelKindMeta {
    if (!kind) return LEVEL_KIND_META.Other;
    return (LEVEL_KIND_META as Record<string, LevelKindMeta>)[kind] ?? LEVEL_KIND_META.Other;
}

/**
 * Try to derive the specific named level from a signal's label.
 *
 * Examples:
 *   PIVOT_R2_RESISTANCE_TEST     → { kind: 'Pivot',     name: 'R2' }
 *   PIVOT_S3_SUPPORT_TEST        → { kind: 'Pivot',     name: 'S3' }
 *   PIVOT_CENTRAL_TEST           → { kind: 'Pivot',     name: 'Pivot' }
 *   SUPERTREND_RESISTANCE_TEST   → { kind: 'Other',     name: 'Supertrend' }
 *   SMC_OB_BULLISH_TEST          → { kind: 'SmcZone',   name: 'Bullish OB' }
 *   SMC_FVG_LEVEL_TEST           → { kind: 'SmcZone',   name: 'FVG' }
 *   BOLLINGER_MIDDLE_TEST        → { kind: 'ChannelMid',name: 'BB Middle' }
 *   ICHIMOKU_KIJUN_TEST          → { kind: 'Ichimoku',  name: 'Kijun' }
 */
export interface ParsedLevel {
    kind: LevelKind;
    name: string;
    role: 'support' | 'resistance' | 'neutral';
}

export function parseLevelLabel(
    indicatorKey: string,
    label: string | undefined | null,
): ParsedLevel {
    const kind = classifyLevelKey(indicatorKey);
    const l = (label ?? '').toUpperCase();

    let name = label ?? '--';
    let role: 'support' | 'resistance' | 'neutral' = 'neutral';

    if (kind === 'Pivot') {
        if (l.includes('CENTRAL')) name = 'Pivot';
        else if (l.includes('R1')) name = 'R1';
        else if (l.includes('R2')) name = 'R2';
        else if (l.includes('R3')) name = 'R3';
        else if (l.includes('S1')) name = 'S1';
        else if (l.includes('S2')) name = 'S2';
        else if (l.includes('S3')) name = 'S3';
        else name = 'Pivot';
        if (l.includes('RESISTANCE')) role = 'resistance';
        else if (l.includes('SUPPORT')) role = 'support';
    } else if (kind === 'SmcZone') {
        if (l.includes('BULLISH_OB') || l.includes('OB_BULLISH')) name = 'Bullish OB';
        else if (l.includes('BEARISH_OB') || l.includes('OB_BEARISH')) name = 'Bearish OB';
        else if (l.includes('FVG')) name = 'FVG';
        else name = 'SMC Zone';
        role = l.includes('BULLISH') ? 'support' : l.includes('BEARISH') ? 'resistance' : 'neutral';
    } else if (kind === 'Ichimoku') {
        if (l.includes('TENKAN')) name = 'Tenkan';
        else if (l.includes('KIJUN')) name = 'Kijun';
        else if (l.includes('SENKOU_A')) name = 'Senkou A';
        else if (l.includes('SENKOU_B')) name = 'Senkou B';
        else name = 'Ichimoku Edge';
    } else if (kind === 'ChannelMid') {
        if (l.includes('MIDDLE')) name = indicatorKey === 'bollinger' ? 'BB Middle' : `${indicatorKey} Mid`;
        else name = indicatorKey;
    } else if (kind === 'SR') {
        name = l.includes('DEMAND') ? 'Demand Zone' : l.includes('SUPPLY') ? 'Supply Zone' : 'S/R Zone';
        role = l.includes('DEMAND') ? 'support' : l.includes('SUPPLY') ? 'resistance' : 'neutral';
    } else if (kind === 'Vwap') {
        name = indicatorKey === 'anchored_vwap' ? 'Anchored VWAP' : 'VWAP';
    } else if (kind === 'VolumeNode') {
        name = l.includes('HVN') ? 'HVN' : l.includes('LVN') ? 'LVN' : 'Volume Node';
    } else if (kind === 'Fibonacci') {
        const m = l.match(/FIB[_ ]?(\d+\.?\d*)|(\d+\.?\d*)[_ ]?FIB/i);
        name = m ? `Fib ${m[1] ?? m[2]}` : 'Fib Level';
    } else {
        if (l.includes('RESISTANCE')) { name = indicatorKey; role = 'resistance'; }
        else if (l.includes('SUPPORT')) { name = indicatorKey; role = 'support'; }
    }

    return { kind, name, role };
}
