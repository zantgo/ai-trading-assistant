// Export JSON helpers — build a deeply-nested snapshot of the currently
// selected timeframe's indicators + signals + context, ready to be
// serialised to a clipboard-friendly string. Used by the Metrics tab
// "Export JSON" button (mirrors the pattern in BottomTable.handleCopyJson).
//
// Exports per-TF L1 data (indicators, signals, context, volume profile,
// liquidity flow/cluster/signals) plus instance-level matrices (opportunity,
// advisory, analysis, risk, alignment) for completeness. The Trade Plan
// (L4/L6 synthesis) is excluded — it belongs to the Decision tab.

import type {
    ConfluentLevel,
    LiquidationCluster,
    LiquidationClusterMatrix,
    LiquidityFlow,
    LiquiditySignal,
    OpportunityMatrix,
    VolumeProfileSnapshot,
} from '../types';
import type {
    AdvisoryMatrix,
    AnalysisMatrix,
    IndicatorDto,
    IndicatorLifecycleStatus,
    IndicatorMeta,
    IndicatorSignal,
    RiskMatrix,
    TimeframeTelemetry,
} from '../types';

interface ExportPayload {
    exported_at: string;
    /** Which UI tab triggered this export — useful when the same payload is
     *  invoked from Metrics, Alignment, Opportunities, Risks, Analysis or
     *  Decision tabs. Empty string when not tagged. */
    source_tab: string;
    symbol: string;
    timeframe: {
        label: string;
        duration_seconds: number;
    };
    timestamp: number | null;
    mark_price: number | null;
    filter_state: {
        active_only: boolean;
        confirmed_plus_only: boolean;
        hide_gates: boolean;
        hide_overlays: boolean;
    };
    indicators: ExportIndicator[];
    signals_total: number;
    context: Record<string, unknown> | null;
    /** L4 OpportunityMatrix (entry zone, target zone, invalidation, RR, time horizon, confluent levels). */
    opportunity: OpportunityExport | null;
    /** L6 AdvisoryMatrix (directional guidance, entry/exit/protection/target strategies, confidence). */
    advisory: AdvisoryExport | null;
    /** L3 AnalysisMatrix (full — same as analysis). */
    analysis: AnalysisExport | null;
    /** L5 RiskMatrix. */
    risk: RiskExport | null;
    /** L2 AlignmentMatrix. */
    alignment: Record<string, unknown> | null;
    /** Per-TF Volume Profile snapshot. */
    volume_profile: VolumeProfileExport | null;
    /** Per-TF Liquidity flow (latest bar's liquidation events + cascade state). */
    liquidity_flow: LiquidityFlowExport | null;
    /** Per-TF Liquidation Cluster matrix (above/below cluster ladders). */
    cluster_matrix: ClusterMatrixExport | null;
    /** Per-TF Liquidity Signals array. */
    liquidity_signals: LiquiditySignal[];
    /** Per-timeframe pipeline lifecycle (INITIALIZING / LOADING / LIVE / STALE / FAILED). */
    pipeline_state: string | null;
    /** Whether the current snapshot is a completed candle (true) or a shadow/live tick (false). */
    is_completed: boolean;
}

// ── Subtypes ────────────────────────────────────────────────

interface ExportIndicator {
    key: string;
    display_name: string;
    group: string;
    class: string;
    raw: number | null;
    normalized: number;
    state: string;
    pending_candle: boolean;
    confidence_pct: number;
    signals: ExportSignal[];
    sub_values: Record<string, number> | null;
    indicator_lifecycle: ExportLifecycleStatus | null;
}

interface ExportLifecycleStatus {
    state: string;
    bars_seen: number;
    bars_required: number;
}

interface ExportSignal {
    kind: string;
    direction: string;
    status: string;
    label: string;
    strength: number;
    age_bars: number | undefined;
}

interface OpportunityExport {
    primary_opportunity: string;
    opportunity_score: number;
    setup_quality: string;
    forecast_confidence: number;
    time_horizon: string;
    entry_zone: { low: number; high: number } | null;
    target_zone: { low: number; high: number } | null;
    invalidation_level: number | null;
    invalidation_note: string;
    expected_rr_internal: number | null;
    contributing_signals: string[];
    profiles: OpportunityMatrix['profiles'];
    confluent_entry_levels: ConfluentLevel[];
    confluent_target_levels: ConfluentLevel[];
    confluent_invalidation_levels: ConfluentLevel[];
}

interface AdvisoryExport {
    directional_guidance: string;
    market_stance: string;
    strategy_environment: string;
    entry_guidance: string;
    exit_guidance: string;
    protection_strategy: string;
    target_strategy: string;
    stop_loss_distance_pct: number | null;
    trade_readiness: string;
    confidence_assessment: number;
    entry_danger: { score: number; level: string; state: string; confidence: number } | null;
    expected_reward_risk_ratio: number;
    expected_rr_internal: number | null;
    final_recommendation: string;
    contributing_indicators: string[];
}

interface AnalysisExport {
    symbol: string;
    bias: string;
    confidence: number;
    state_confidence: number;
    market_regime: string;
    trend_assessment: string;
    momentum_assessment: string;
    structure_assessment: string;
    volatility_assessment: string;
    volume_assessment: string;
    opportunity_analysis: string;
    market_quality: string;
    market_quality_score: number;
    timeframes_considered: number;
    supporting_signals: string[];
    contradicting_signals: string[];
}

interface RiskExport {
    symbol: string;
    overall: { score: number; level: string; state: string; confidence: number };
    by_dimension: Array<{ name: string; score: number; level: string; confidence: number }>;
    cascade_risk_score: number;
    overall_risk_score: number;
}

interface VolumeProfileExport {
    symbol: string;
    timeframe_slot: string;
    timeframe_secs: number;
    poc_price: number;
    value_area_high: number;
    value_area_low: number;
    total_volume: number;
    range_low: number;
    range_high: number;
    num_bins: number;
    timestamp_ms: number;
    /** Top 3 HVN (high-volume nodes) sorted by volume desc. */
    top_hvn: Array<{ price_low: number; price_high: number; volume: number; buy_volume: number; sell_volume: number; strength_x_mean: number }>;
    buy_total: number;
    sell_total: number;
    buy_sell_bias: number;
    /** "INSIDE VA" or "OUTSIDE VA n% of range". */
    current_position: { in_va: boolean; range_pos_pct: number };
}

interface LiquidityFlowExport {
    long_liquidations_usd: number;
    short_liquidations_usd: number;
    net_liquidation_usd: number;
    event_count: number;
    largest_event_usd: number;
    largest_event_price: number | null;
    largest_event_side: string | null;
    cascade_state: string;
    cascade_intensity: number;
}

interface ClusterMatrixExport {
    mid_price: number;
    cascade_asymmetry: number;
    total_long_oi_usd: number;
    total_short_oi_usd: number;
    estimation_confidence: number;
    leverage_assumptions: { source: string; buckets: number[]; weights: number[]; funding_modulation_active: boolean };
    top_above: Array<{ peak_price: number; distance_from_mid_pct: number; notional_usd: number; magnet_strength: number; cluster_kind: string }>;
    top_below: Array<{ peak_price: number; distance_from_mid_pct: number; notional_usd: number; magnet_strength: number; cluster_kind: string }>;
}

interface ExportIndicator {
    key: string;
    display_name: string;
    group: string;
    class: string;
    raw: number | null;
    normalized: number;
    state: string;
    pending_candle: boolean;
    confidence_pct: number;
    signals: ExportSignal[];
    sub_values: Record<string, number> | null;
    indicator_lifecycle: ExportLifecycleStatus | null;
}

interface ExportLifecycleStatus {
    state: string;
    bars_seen: number;
    bars_required: number;
}

interface ExportSignal {
    kind: string;
    direction: string;
    status: string;
    label: string;
    strength: number;
    age_bars: number | undefined;
}

const ABBR: Record<string, string> = {
    Divergence: 'DIV', Crossover: 'CRO', Threshold: 'TH', Breakout: 'BO',
    BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
    LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
    StackChange: 'STK', PatternForming: 'PAT',
};

function valueFormat(meta: IndicatorMeta): string {
    return meta.value_format;
}

function isSqueezeOnKey(meta: IndicatorMeta): boolean {
    return meta.key === 'squeeze' && valueFormat(meta) === 'onoff';
}

function isOnOffKey(meta: IndicatorMeta): boolean {
    return valueFormat(meta) === 'onoff';
}

function iRaw(indicators: Record<string, IndicatorDto>, key: string): number | null {
    return indicators?.[key]?.raw_value ?? null;
}

function iSub(
    indicators: Record<string, IndicatorDto>,
    key: string,
    sub: string,
): number | null {
    const subValues = indicators?.[key]?.values ?? null;
    const raw = subValues?.[sub];
    if (raw == null || Number.isNaN(raw)) return null;
    return raw;
}

function fmtNum(v: number | null, decimals = 2): string {
    if (v == null || Number.isNaN(v)) return '--';
    return v.toFixed(decimals);
}

function fmtPrice(v: number | null, markPrice: number): string {
    if (v == null || Number.isNaN(v)) return '--';
    const p = Math.abs(markPrice);
    let decimals: number;
    if (p >= 10000) decimals = 1;
    else if (p >= 1000) decimals = 2;
    else if (p >= 100) decimals = 3;
    else if (p >= 10) decimals = 4;
    else if (p >= 1) decimals = 6;
    else decimals = 8;
    return v.toFixed(decimals);
}

function rawVal(meta: IndicatorMeta, indicators: Record<string, IndicatorDto>): number | null {
    if (meta.value_source.startsWith('sub:')) {
        return iSub(indicators, meta.key, meta.value_source.slice(4));
    }
    return iRaw(indicators, meta.key);
}

function formatRaw(meta: IndicatorMeta, indicators: Record<string, IndicatorDto>, markPrice: number): number | null {
    if (isOnOffKey(meta)) {
        const onoff = isSqueezeOnKey(meta) ? isSqueezeOnValue(indicators) : rawVal(meta, indicators) != null;
        return onoff ? 1 : 0;
    }
    const v = rawVal(meta, indicators);
    if (v == null) return null;
    switch (valueFormat(meta)) {
        case 'percent1':  return Number(v.toFixed(1));
        case 'price':     {
            const p = Math.abs(markPrice);
            const decls = p >= 10000 ? 1 : p >= 1000 ? 2 : p >= 100 ? 3 : p >= 10 ? 4 : p >= 1 ? 6 : 8;
            return Number(v.toFixed(decls));
        }
        case 'ratio2':    return Number(v.toFixed(2));
        case 'decimals1': return Number(v.toFixed(1));
        case 'decimals4': return Number(v.toFixed(4));
        case 'decimals2':
        default:          return Number(v.toFixed(2));
    }
}

function isSqueezeOnValue(indicators: Record<string, IndicatorDto>): boolean {
    const dto = indicators?.['squeeze'];
    if (!dto) return false;
    return dto.state_label === 'COMPRESSION_COILING';
}

function confidence(indicators: Record<string, IndicatorDto>, key: string): number {
    const dto = indicators?.[key];
    if (!dto?.confidence) return 0;
    return Math.round(Math.abs(dto.confidence) * 100);
}

export interface ExportArgs {
    /** UI tab that triggered the export (e.g. 'metrics', 'alignment', ...). */
    sourceTab?: string;
    symbol: string;
    tfLabel: string;
    tfSecs: number;
    timestamp: number | null;
    markPrice: number;
    registry: IndicatorMeta[];
    tf: TimeframeTelemetry;
    filters: { activeOnly: boolean; confirmedPlusOnly: boolean; hideGates: boolean; hideOverlays: boolean };
    analysis: AnalysisMatrix | null;
    risk: RiskMatrix | null;
    alignment: Record<string, unknown> | null;
    opportunity: OpportunityMatrix | null;
    advisory: AdvisoryMatrix | null;
    volumeProfile: VolumeProfileSnapshot | null;
    liquidity: LiquidityFlow | null;
    cluster: LiquidationClusterMatrix | null;
    liquiditySignals: LiquiditySignal[];
    decisionContext: Record<string, unknown> | null;
}

export function buildMetricsExportJson(args: ExportArgs): string {
    const {
        sourceTab = '', symbol, tfLabel, tfSecs, timestamp, markPrice, registry, tf, filters,
        analysis, risk, alignment, opportunity, advisory, volumeProfile,
        liquidity, cluster, liquiditySignals, decisionContext,
    } = args;
    const inds = (tf?.indicators ?? {}) as Record<string, IndicatorDto>;
    const fibVals = (inds['fibonacci']?.values ?? {}) as Record<string, number | undefined>;
    const fibExtracted = (Object.keys(fibVals).length > 0)
        ? {
            gp_top: fibVals['gp_top'] ?? null,
            gp_bottom: fibVals['gp_bottom'] ?? null,
            ext_1618: fibVals['ext_1618'] ?? null,
            ext_2618: fibVals['ext_2618'] ?? null,
            retracement_coefficients: {
                fib_0236: fibVals['fib_0236'] ?? null,
                fib_0382: fibVals['fib_0382'] ?? null,
                fib_0500: fibVals['fib_0500'] ?? null,
                fib_0618: fibVals['fib_0618'] ?? null,
                fib_0660: fibVals['fib_0660'] ?? null,
                fib_0786: fibVals['fib_0786'] ?? null,
            },
            fibonacci_present: true,
        }
        : { fibonacci_present: false };

    const out: ExportPayload = {
        exported_at: new Date().toISOString(),
        source_tab: sourceTab,
        symbol,
        timeframe: { label: tfLabel, duration_seconds: tfSecs },
        timestamp,
        mark_price: isFinite(markPrice) && markPrice > 0 ? markPrice : null,
        filter_state: {
            active_only: filters.activeOnly,
            confirmed_plus_only: filters.confirmedPlusOnly,
            hide_gates: filters.hideGates,
            hide_overlays: filters.hideOverlays,
        },
        indicators: [],
        signals_total: 0,
        pipeline_state: (tf?.pipelineState ?? null) as string | null,
        is_completed: tf?.isCompleted ?? false,
        context: (tf?.context ?? null) as unknown as Record<string, unknown> | null,
        opportunity: exportOpportunity(opportunity, inds),
        advisory: exportAdvisory(advisory, decisionContext),
        analysis: exportAnalysis(analysis),
        risk: exportRisk(risk),
        alignment: alignment ?? null,
        volume_profile: exportVolumeProfile(volumeProfile, markPrice),
        liquidity_flow: exportLiquidityFlow(liquidity),
        cluster_matrix: exportClusterMatrix(cluster),
        liquidity_signals: (liquiditySignals ?? []).map((s) => ({
            kind: s.kind,
            direction: s.direction,
            strength: s.strength,
            confidence: s.confidence,
            evidence: s.evidence,
        })),
    };

    const uniqueLabels = new Set<string>();

    for (const m of registry) {
        const dto = inds[m.key];
        if (!dto) continue;
        const signals: ExportSignal[] = (dto.signals ?? []).map((s: IndicatorSignal) => ({
            kind: ABBR[s.kind] ?? s.kind,
            direction: s.direction,
            status: s.status,
            label: s.label,
            strength: s.strength,
            age_bars: s.age_bars,
        }));
        // Phase 8: count unique signal labels (not raw signal objects).
        // This matches the UI's SIGNALS badge count in the FacetTabs.
        for (const s of signals) uniqueLabels.add(s.label);
        const subValues: Record<string, number> = {};
        if (dto.values) {
            for (const [k, v] of Object.entries(dto.values)) {
                if (v != null && !Number.isNaN(v)) subValues[k] = v;
            }
            if (m.key === 'fibonacci') {
                subValues['__fib_extracted__'] = 1;
            }
        }
        const lc = tf.indicatorLifecycle?.[m.key];
        const lifecycleExport: ExportLifecycleStatus | null = lc ? {
            state: lc.state,
            bars_seen: lc.bars_seen,
            bars_required: lc.bars_required,
        } : null;
        const pending = !tf.isCompleted
            && lc?.state === 'Live'
            && !(m.updates_on_shadow ?? false);
        out.indicators.push({
            key: m.key,
            display_name: m.display_name,
            group: m.group,
            class: m.class,
            raw: formatRaw(m, inds, markPrice),
            normalized: dto.normalized ?? 0,
            state: dto.state_label ?? '--',
            pending_candle: pending,
            confidence_pct: confidence(inds, m.key),
            signals,
            sub_values: Object.keys(subValues).length > 0 ? subValues : null,
            indicator_lifecycle: lifecycleExport,
        });
    }

    out.signals_total = uniqueLabels.size;

    out.indicators.push({
        key: '__fibonacci_summary__',
        display_name: 'Fibonacci Levels (computed values)',
        group: 'Fibonacci',
        class: 'Leading',
        raw: null,
        normalized: inds['fibonacci']?.normalized ?? 0,
        state: inds['fibonacci']?.state_label ?? '--',
        confidence_pct: confidence(inds, 'fibonacci'),
        pending_candle: false,
        signals: [],
        sub_values: fibExtracted as unknown as Record<string, number>,
        indicator_lifecycle: null,
    });

    return JSON.stringify(out, null, 2);
}

// ── Per-block exporters ──────────────────────────────────────

function exportOpportunity(opp: OpportunityMatrix | null, indicators: Record<string, IndicatorDto>): OpportunityExport | null {
    if (!opp) return null;
    const fibVals = (indicators['fibonacci']?.values ?? {}) as Record<string, number | undefined>;
    return {
        primary_opportunity: opp.primary_opportunity,
        opportunity_score: opp.opportunity_score,
        setup_quality: opp.setup_quality,
        forecast_confidence: opp.forecast_confidence,
        time_horizon: opp.time_horizon,
        entry_zone: opp.entry_zone ? { low: opp.entry_zone.low, high: opp.entry_zone.high } : null,
        target_zone: opp.target_zone ? { low: opp.target_zone.low, high: opp.target_zone.high } : null,
        invalidation_level: opp.invalidation_level ?? null,
        invalidation_note: opp.invalidation_note,
        expected_rr_internal: opp.expected_rr_internal ?? null,
        contributing_signals: opp.contributing_signals,
        profiles: opp.profiles,
        confluent_entry_levels: opp.confluent_entry_levels ?? [],
        confluent_target_levels: opp.confluent_target_levels ?? [],
        confluent_invalidation_levels: opp.confluent_invalidation_levels ?? [],
        ...fibVals, // also include fib values inline here for redundancy
        __fib_inline__: true,
    } as OpportunityExport;
}

function exportAdvisory(
    adv: AdvisoryMatrix | null,
    decisionContext: Record<string, unknown> | null,
): AdvisoryExport | null {
    if (!adv) return null;
    const ed = (decisionContext as any)?.entry_danger;
    return {
        directional_guidance: adv.directional_guidance,
        market_stance: adv.market_stance,
        strategy_environment: adv.strategy_environment,
        entry_guidance: adv.entry_guidance,
        exit_guidance: adv.exit_guidance,
        protection_strategy: adv.protection_strategy,
        target_strategy: adv.target_strategy,
        stop_loss_distance_pct: (adv as any).stop_loss_distance_pct ?? null,
        trade_readiness: String((decisionContext as any)?.trade_readiness ?? 'UNKNOWN'),
        confidence_assessment: adv.confidence_assessment,
        expected_reward_risk_ratio: (decisionContext as any)?.expected_reward_risk_ratio ?? 0,
        expected_rr_internal: null,
        final_recommendation: adv.final_recommendation,
        contributing_indicators: (decisionContext as any)?.contributing_indicators ?? [],
        entry_danger: ed ? {
            score: ed.score ?? 0,
            level: ed.level ?? 'UNKNOWN',
            state: ed.state ?? 'UNKNOWN',
            confidence: ed.confidence ?? 0,
        } : null,
    };
}

function exportAnalysis(a: AnalysisMatrix | null): AnalysisExport | null {
    if (!a) return null;
    return {
        symbol: a.symbol,
        bias: a.bias,
        confidence: a.confidence,
        state_confidence: a.state_confidence,
        market_regime: a.market_regime,
        trend_assessment: a.trend_assessment,
        momentum_assessment: a.momentum_assessment,
        structure_assessment: a.structure_assessment,
        volatility_assessment: a.volatility_assessment,
        volume_assessment: a.volume_assessment,
        opportunity_analysis: a.opportunity_analysis,
        market_quality: a.market_quality,
        market_quality_score: a.market_quality_score,
        timeframes_considered: a.timeframes_considered,
        supporting_signals: a.supporting_signals ?? [],
        contradicting_signals: a.contradicting_signals ?? [],
    };
}

function exportRisk(r: RiskMatrix | null): RiskExport | null {
    if (!r) return null;
    const execLiq = r.execution_liquidity_risk ?? r.market_risk;
    return {
        symbol: r.symbol,
        overall: {
            score: r.overall_risk.score,
            level: r.overall_risk.level,
            state: r.overall_risk.state,
            confidence: r.overall_risk.confidence,
        },
        by_dimension: [
            { name: 'Market Risk',                  score: r.market_risk.score,        level: r.market_risk.level,        confidence: r.market_risk.confidence },
            { name: 'Volatility Risk',              score: r.volatility_risk.score,    level: r.volatility_risk.level,    confidence: r.volatility_risk.confidence },
            { name: 'Execution Liquidity Risk',     score: execLiq.score, level: execLiq.level, confidence: execLiq.confidence },
            { name: 'Structure Risk',               score: r.structure_risk.score,     level: r.structure_risk.level,     confidence: r.structure_risk.confidence },
            { name: 'Momentum Risk',                score: r.momentum_risk.score,      level: r.momentum_risk.level,      confidence: r.momentum_risk.confidence },
            { name: 'Signal Risk',                  score: r.signal_risk.score,        level: r.signal_risk.level,        confidence: r.signal_risk.confidence },
            { name: 'Execution Risk',               score: r.execution_risk.score,     level: r.execution_risk.level,     confidence: r.execution_risk.confidence },
            { name: 'Cascade Risk',                 score: r.cascade_risk?.score ?? 0, level: r.cascade_risk?.level ?? 'UNKNOWN', confidence: r.cascade_risk?.confidence ?? 0 },
        ],
        cascade_risk_score: r.cascade_risk?.score ?? 0,
        overall_risk_score: r.overall_risk.score,
    };
}

function exportVolumeProfile(vp: VolumeProfileSnapshot | null, markPrice: number): VolumeProfileExport | null {
    if (!vp) return null;
    const meanVol = vp.bins.length > 0
        ? vp.bins.reduce((a, b) => a + b.volume, 0) / vp.bins.length
        : 0;
    const topHvn = (meanVol > 0 ? vp.bins
        .filter((b) => b.volume >= 1.5 * meanVol)
        .sort((a, b) => b.volume - a.volume)
        .slice(0, 3) : vp.bins
        .slice()
        .sort((a, b) => b.volume - a.volume)
        .slice(0, 3));
    const buy = vp.bins.reduce((a, b) => a + b.buy_volume, 0);
    const sell = vp.bins.reduce((a, b) => a + b.sell_volume, 0);
    const total = buy + sell;
    const range = vp.range_high - vp.range_low;
    const rangePos = markPrice > 0 && range > 0 ? (markPrice - vp.range_low) / range : 0;
    const inVa = markPrice >= vp.value_area_low && markPrice <= vp.value_area_high;
    return {
        symbol: vp.symbol,
        timeframe_slot: vp.timeframe_slot,
        timeframe_secs: vp.timeframe_secs,
        poc_price: vp.poc_price,
        value_area_high: vp.value_area_high,
        value_area_low: vp.value_area_low,
        total_volume: vp.total_volume,
        range_low: vp.range_low,
        range_high: vp.range_high,
        num_bins: vp.num_bins,
        timestamp_ms: vp.timestamp_ms,
        top_hvn: topHvn.map((b) => ({
            price_low: b.price_low,
            price_high: b.price_high,
            volume: b.volume,
            buy_volume: b.buy_volume,
            sell_volume: b.sell_volume,
            strength_x_mean: meanVol > 0 ? Number((b.volume / meanVol).toFixed(2)) : 0,
        })),
        buy_total: buy,
        sell_total: sell,
        buy_sell_bias: total > 0 ? Number(((buy - sell) / total).toFixed(4)) : 0,
        current_position: {
            in_va: inVa,
            range_pos_pct: Number((rangePos * 100).toFixed(2)),
        },
    };
}

function exportLiquidityFlow(lf: LiquidityFlow | null): LiquidityFlowExport | null {
    if (!lf) return null;
    return {
        long_liquidations_usd: lf.long_liquidations_usd,
        short_liquidations_usd: lf.short_liquidations_usd,
        net_liquidation_usd: lf.net_liquidation_usd,
        event_count: lf.event_count,
        largest_event_usd: lf.largest_event_usd,
        largest_event_price: lf.largest_event_price ?? null,
        largest_event_side: lf.largest_event_side ?? null,
        cascade_state: lf.cascade_state,
        cascade_intensity: lf.cascade_intensity,
    };
}

function exportClusterMatrix(cm: LiquidationClusterMatrix | null): ClusterMatrixExport | null {
    if (!cm) return null;
    function topSide(arr: LiquidationCluster[] | undefined, dir: 'asc' | 'desc') {
        if (!arr) return [];
        return [...arr]
            .sort((a, b) => dir === 'asc' ? Math.abs(a.distance_from_mid_pct) - Math.abs(b.distance_from_mid_pct) : Math.abs(b.distance_from_mid_pct) - Math.abs(a.distance_from_mid_pct))
            .slice(0, 3);
    }
    return {
        mid_price: cm.mid_price,
        cascade_asymmetry: cm.cascade_asymmetry,
        total_long_oi_usd: cm.total_long_oi_usd,
        total_short_oi_usd: cm.total_short_oi_usd,
        estimation_confidence: cm.estimation_confidence,
        leverage_assumptions: {
            source: cm.leverage_assumptions.source,
            buckets: cm.leverage_assumptions.buckets,
            weights: cm.leverage_assumptions.weights,
            funding_modulation_active: cm.leverage_assumptions.funding_modulation_active,
        },
        top_above: topSide(cm.short_clusters, 'asc').map((c) => ({
            peak_price: c.peak_price,
            distance_from_mid_pct: c.distance_from_mid_pct,
            notional_usd: c.notional_usd,
            magnet_strength: c.magnet_strength,
            cluster_kind: c.cluster_kind,
        })),
        top_below: topSide(cm.long_clusters, 'asc').map((c) => ({
            peak_price: c.peak_price,
            distance_from_mid_pct: c.distance_from_mid_pct,
            notional_usd: c.notional_usd,
            magnet_strength: c.magnet_strength,
            cluster_kind: c.cluster_kind,
        })),
    };
}

// Used by callers via copy-to-clipboard. Returns true on success, false otherwise.
export async function copyJsonToClipboard(text: string): Promise<boolean> {
    try {
        await navigator.clipboard.writeText(text);
        return true;
    } catch (_) {
        return false;
    }
}

// ── Panel-level helpers ───────────────────────────────────────────
//
// Each panel exports the *full* snapshot — same payload shape as the Metrics
// export — but is tagged with `source_tab` so the recipient can identify the
// originator. This guarantees that no matter which tab the operator clicks
// the EXPORT DATA button on, the clipboard contains every value visible in
// the UI (metrics, indicators, alignment, opportunity, advisory, analysis,
// risk, decision, volume profile, liquidity flow, cluster matrix, liquidity
// signals). The Trade Plan (L4/L6 synthesis) is included via the advisory
// section.
//
// `null` is returned when the calling panel has no instance to export,
// so the button can short-circuit before invoking the clipboard.

export interface PanelExportArgs {
    /** Tag written into the JSON `source_tab` field. */
    sourceTab: string;
    /** Pair key (e.g. 'BTC-USDT') used to look up the active instance. */
    pairKey: string;
    /** Resolver hooks for instance-level data (mirrors the existing Metrics
     *  builder inputs) so each panel can hand in its own reactive bindings
     *  without needing a fresh `ExportArgs` shape. All resolvers return
     *  `null` / `[]` when the underlying data hasn't loaded yet. */
    resolvers: Omit<ExportArgs, 'sourceTab' | 'symbol' | 'tfLabel' | 'tfSecs' | 'timestamp' | 'markPrice' | 'registry' | 'tf' | 'filters'> & {
        symbol: string;
        tfLabel: string;
        tfSecs: number;
        timestamp: number | null;
        markPrice: number;
        registry: IndicatorMeta[];
        tf: TimeframeTelemetry;
        filters: { activeOnly: boolean; confirmedPlusOnly: boolean; hideGates: boolean; hideOverlays: boolean };
    };
}

/** Build the same Metrics export payload but tagged with the calling tab.
 *  Returns `null` if the pair key resolves to no instance (button will
 *  display "Copy failed"). */
export function buildPanelExportJson(args: PanelExportArgs): string | null {
    const r = args.resolvers;
    if (!r.symbol) return null;
    return buildMetricsExportJson({
        sourceTab: args.sourceTab,
        symbol: r.symbol,
        tfLabel: r.tfLabel,
        tfSecs: r.tfSecs,
        timestamp: r.timestamp,
        markPrice: r.markPrice,
        registry: r.registry,
        tf: r.tf,
        filters: r.filters,
        analysis: r.analysis,
        risk: r.risk,
        alignment: r.alignment,
        opportunity: r.opportunity,
        advisory: r.advisory,
        volumeProfile: r.volumeProfile,
        liquidity: r.liquidity,
        cluster: r.cluster,
        liquiditySignals: r.liquiditySignals,
        decisionContext: r.decisionContext,
    });
}
