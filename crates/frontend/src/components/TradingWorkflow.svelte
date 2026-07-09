<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import {
        STAGE_META, STAGE_ORDER, stageForKey, categoryForKey,
        groupIndicatorsByStage,
        type DecisionStage,
    } from '../lib/decisionStages';
    import type {
        IndicatorDto, IndicatorMeta, IndicatorMap, IndicatorSignal,
        TimeframeTelemetry, DecisionContext, PositionState,
    } from '../types';
    import styles from './TradingWorkflow.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();

    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);
    const stageBuckets = $derived(groupIndicatorsByStage(registry));

    let activeStage = $state<DecisionStage>('Setup');
    let activeTf = $state<keyof typeof TF_KEYS>('microTerm');
    let entryPrice = $state('');
    let stopLoss = $state('');
    let monitoringData = $state<any>(null);
    let monitoringLoading = $state(false);
    let monitoringTimer: ReturnType<typeof setInterval> | null = null;

    type TfKey = 'microTerm' | 'fastTerm' | 'slowTerm' | 'macroTerm';
    const TF_KEYS = { microTerm: true, fastTerm: true, slowTerm: true, macroTerm: true };
    const TF_TABS: { key: TfKey; label: string; secs: number }[] = [
        { key: 'microTerm', label: 'MICRO', secs: 60 },
        { key: 'fastTerm', label: 'FAST', secs: 180 },
        { key: 'slowTerm', label: 'SLOW', secs: 300 },
        { key: 'macroTerm', label: 'MACRO', secs: 900 },
    ];

    const activeTelemetry = $derived<TimeframeTelemetry | null>(
        pair ? pair[activeTf] as TimeframeTelemetry : null,
    );
    const activeIndicators = $derived<IndicatorMap>(
        activeTelemetry?.indicators ?? {},
    );
    const priceRef = $derived(parseFloat(activeTelemetry?.priceText ?? '0') || 0);

    const stageMetaFor = (s: DecisionStage) => STAGE_META[s];

    function indicatorMetasForStage(stage: DecisionStage): IndicatorMeta[] {
        for (const [s, metas] of stageBuckets) {
            if (s === stage) return metas;
        }
        return [];
    }

    function getWeight(meta: IndicatorMeta): number {
        const p = pair;
        if (!p) return meta.default_weight;
        const tfWeights = p.indicatorWeights[activeTf];
        if (tfWeights && tfWeights[meta.key] !== undefined) return tfWeights[meta.key];
        return meta.default_weight;
    }

    function setWeight(meta: IndicatorMeta, val: number) {
        const p = pair;
        if (!p) return;
        if (!p.indicatorWeights[activeTf]) p.indicatorWeights[activeTf] = {};
        if (val === meta.default_weight) {
            delete p.indicatorWeights[activeTf][meta.key];
        } else {
            p.indicatorWeights[activeTf][meta.key] = val;
        }
    }

    function normColor(normalized: number): string {
        const mag = Math.min(Math.abs(normalized), 1);
        if (mag >= 0.9) return '#a855f7';
        if (normalized > 0.1) return `rgb(16, ${Math.round(120 + 135 * mag)}, 129)`;
        if (normalized < -0.1) return `rgb(${Math.round(180 + 59 * mag)}, 68, 68)`;
        return '#94a3b8';
    }

    function dirColor(d: string): string {
        return d === 'Bullish' ? '#10b981' : d === 'Bearish' ? '#ef4444' : '#94a3b8';
    }

    function signalLabel(s: IndicatorSignal): string {
        const kindMap: Record<string, string> = {
            Divergence: 'DIV', Crossover: 'XOVER', Threshold: 'THR', Breakout: 'BO',
            BandTouch: 'BAND', ZeroLineCross: 'ZERO', CompressionRelease: 'SQZ',
            LevelTest: 'LVL', TrendFlip: 'FLIP', VolumeClimax: 'CLIMAX',
            StackChange: 'STK', PatternForming: 'PAT',
        };
        return kindMap[s.kind] ?? s.kind.slice(0, 4);
    }

    function dominantSignal(signals?: IndicatorSignal[]): IndicatorSignal | null {
        if (!signals?.length) return null;
        const rank: Record<string, number> = { Confirmed: 3, Active: 2, Potential: 1 };
        return [...signals].sort((a, b) =>
            (rank[b.status] - rank[a.status]) || (b.strength - a.strength),
        )[0];
    }

    function fmtRaw(dto: IndicatorDto | undefined, meta: IndicatorMeta): string {
        if (!dto || dto.raw_value == null) return '--';
        switch (meta.value_format) {
            case 'percent1': return `${dto.raw_value.toFixed(1)}%`;
            case 'price': return `$${dto.raw_value.toFixed(2)}`;
            case 'ratio2': return dto.raw_value.toFixed(2);
            case 'decimals1': return dto.raw_value.toFixed(1);
            case 'decimals4': return dto.raw_value.toFixed(4);
            case 'onoff': return meta.key === 'squeeze' ? (dto.normalized > 0 ? 'ON' : 'OFF') : (dto.raw_value !== 0 ? 'ON' : 'OFF');
            default: return dto.raw_value.toFixed(2);
        }
    }

    function dcBarFill(val: number): string {
        return `${Math.min(Math.abs(val), 1) * 100}%`;
    }

    function dcBarLeft(val: number): string {
        return val >= 0 ? '50%' : `${50 - Math.min(Math.abs(val), 1) * 50}%`;
    }

    async function fetchMonitoring() {
        if (monitoringLoading) return;
        monitoringLoading = true;
        try {
            const res = await fetch(`/api/monitor/active-trades?symbol=${encodeURIComponent(pairKey)}`);
            if (res.ok) monitoringData = await res.json();
        } catch (_) { /* transient */ }
        monitoringLoading = false;
    }

    onMount(() => {
        fetchMonitoring();
        monitoringTimer = setInterval(fetchMonitoring, 5000);
    });
    onDestroy(() => {
        if (monitoringTimer) clearInterval(monitoringTimer);
    });

    $effect(() => {
        if (activeStage === 'Monitoring') fetchMonitoring();
    });

    function buildExportPayload(): object {
        const tfKeys: TfKey[] = ['microTerm', 'fastTerm', 'slowTerm', 'macroTerm'];
        const tfLabel: Record<TfKey, string> = { microTerm: 'MICRO', fastTerm: 'FAST', slowTerm: 'SLOW', macroTerm: 'MACRO' };
        const p = pair;
        const stages: Record<string, Record<string, any[]>> = {};

        for (const stage of STAGE_ORDER) {
            const metas = indicatorMetasForStage(stage);
            const perTf: Record<string, any[]> = {};
            for (const tf of tfKeys) {
                const tele = p ? p[tf] as TimeframeTelemetry : null;
                const map = tele?.indicators ?? {};
                const items = metas.map((m) => {
                    const dto = map[m.key];
                    const w = p?.indicatorWeights?.[tf]?.[m.key] ?? m.default_weight;
                    return {
                        key: m.key,
                        display_name: m.display_name,
                        weight: w,
                        group: m.group,
                        class: m.class,
                        raw_value: dto?.raw_value ?? null,
                        normalized: dto?.normalized ?? 0,
                        state_label: dto?.state_label ?? 'UNKNOWN',
                        confidence: dto?.confidence ?? null,
                        signals: dto?.signals?.map((s) => ({
                            kind: s.kind,
                            direction: s.direction,
                            status: s.status,
                            strength: s.strength,
                            label: s.label,
                        })) ?? [],
                    };
                });
                perTf[tfLabel[tf]] = items;
            }

            if (stage === 'Execution' && p) {
                for (const tf of tfKeys) {
                    const dc = (p[tf] as TimeframeTelemetry).decisionContext;
                    if (dc) {
                        (perTf[tfLabel[tf]] as any).push({
                            key: 'decision_context',
                            display_name: 'Decision Context',
                            bullish_probability: dc.bullish_probability,
                            bearish_probability: dc.bearish_probability,
                            directional_bias: dc.directional_bias,
                            consensus: dc.consensus,
                            confluence: dc.confluence,
                            risk_level: dc.risk_level,
                            reward_risk_ratio: dc.reward_risk_ratio,
                            trade_readiness: dc.trade_readiness,
                            trade_quality: dc.trade_quality,
                            regime_confidence: dc.regime_confidence,
                        });
                    }
                }
            }

            stages[stage] = perTf;
        }

        return {
            symbol: p?.symbol ?? pairKey,
            price: priceRef || null,
            position: { direction: entryPrice ? 'Long' : 'None', entry_price: entryPrice || null, stop_loss: stopLoss || null },
            timestamp: new Date().toISOString(),
            stages,
        };
    }

    async function exportJSON() {
        try {
            const payload = buildExportPayload();
            const text = JSON.stringify(payload, null, 2);
            await navigator.clipboard.writeText(text);
            exportCopied = true;
            setTimeout(() => (exportCopied = false), 2000);
        } catch (_) {
            // clipboard may not be available
        }
    }

    let exportCopied = $state(false);

    function fmtUsd(v: number | null | undefined): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }
    function fmtPct(v: number | null | undefined): string {
        if (v == null || isNaN(v)) return '0.00%';
        return (v >= 0 ? '+' : '') + v.toFixed(2) + '%';
    }
    function pnlClass(v: number): string {
        if (v > 0) return styles.pnlPos;
        if (v < 0) return styles.pnlNeg;
        return '';
    }
</script>

<div class={styles.container}>
    <!-- Stage tabs bar -->
    <div class={styles.topBar}>
        <div class={styles.stageTabs}>
            {#each STAGE_ORDER as stage}
                <button
                    class="{styles.stageTab} {activeStage === stage ? styles.stageTabActive : ''}"
                    onclick={() => (activeStage = stage)}
                >
                    <span class={styles.stageNum}>
                        {#if STAGE_ORDER.indexOf(stage) === 0}①
                        {:else if STAGE_ORDER.indexOf(stage) === 1}②
                        {:else if STAGE_ORDER.indexOf(stage) === 2}③
                        {:else if STAGE_ORDER.indexOf(stage) === 3}④
                        {:else}⑤
                        {/if}
                    </span>
                    <span class={styles.stageLabel}>{stageMetaFor(stage).title}</span>
                    <span class={styles.stageSubtitle}>{stageMetaFor(stage).subtitle}</span>
                </button>
            {/each}
        </div>
        <button class={styles.exportBtn} onclick={exportJSON} title="Copy all stages JSON to clipboard">
            {exportCopied ? '✓ Copied' : '📋 Export JSON'}
        </button>
    </div>

    <!-- Timeframe sub-tabs -->
    <div class={styles.tfBar}>
        {#each TF_TABS as tf}
            <button
                class="{styles.tfTab} {activeTf === tf.key ? styles.tfTabActive : ''}"
                onclick={() => (activeTf = tf.key)}
            >
                <span class={styles.tfLabel}>{tf.label}</span>
                <span class={styles.tfSecs}>{tf.secs}s</span>
                {#if activeTelemetry}
                    <span class={styles.tfPrice}>
                        {activeTf === tf.key && activeTelemetry.priceText !== '--'
                            ? '$' + activeTelemetry.priceText
                            : ''}
                    </span>
                {/if}
            </button>
        {/each}
    </div>

    <!-- Content -->
    <div class={styles.content}>
        {#if activeStage === 'Setup' || activeStage === 'Trigger' || activeStage === 'Confirmation'}
            {@const metas = indicatorMetasForStage(activeStage)}
            {#if metas.length === 0}
                <div class={styles.emptyState}>No indicators registered for this stage.</div>
            {:else}
                <div class={styles.cardGrid}>
                    {#each metas as meta (meta.key)}
                        {@const dto = activeIndicators[meta.key]}
                        {@const dom = dominantSignal(dto?.signals ?? undefined)}
                        {@const n = dto?.normalized ?? 0}
                        {@const nc = normColor(n)}
                        {@const w = getWeight(meta)}
                        <div class={styles.card} style="border-left: 3px solid {meta.color}">
                            <div class={styles.cardHeader}>
                                <span class={styles.cardDot} style="background:{meta.color}"></span>
                                <span class={styles.cardName}>{meta.display_name}</span>
                                <input
                                    class={styles.weightInput}
                                    type="number"
                                    step="0.1"
                                    min="0"
                                    max="5"
                                    value={w}
                                    oninput={(e) => setWeight(meta, parseFloat(e.currentTarget.value) || 0)}
                                    title="Indicator weight (default: {meta.default_weight})"
                                />
                                <span class={styles.cardClass}>{meta.class.slice(0, 3).toUpperCase()}</span>
                                {#if !meta.directional}
                                    <span class={styles.cardGate} title="Non-directional">◐</span>
                                {/if}
                            </div>

                            {#if meta.directional}
                                <div class={styles.cardGauge}>
                                    <span class={styles.gaugeTrack}>
                                        <span class={styles.gaugeZero}></span>
                                        <span
                                            class={styles.gaugeFill}
                                            style="left:{dcBarLeft(n)};width:{dcBarFill(n)};background:{nc}"
                                        ></span>
                                    </span>
                                    <span class={styles.gaugeVal} style="color:{nc}">
                                        {n >= 0 ? '+' : ''}{n.toFixed(2)}
                                    </span>
                                </div>
                            {:else}
                                <div class={styles.cardRawVal}>{fmtRaw(dto, meta)}</div>
                            {/if}

                            <div class={styles.cardFooter}>
                                <span class={styles.cardState} style="color:{nc}">
                                    {dto?.state_label ?? 'UNKNOWN'}
                                </span>
                                {#if meta.directional}
                                    <span class={styles.cardRaw} title="Raw value">{fmtRaw(dto, meta)}</span>
                                {/if}
                                {#if dto?.confidence != null}
                                    <span class={styles.cardConf}>{Math.round(dto.confidence * 100)}%</span>
                                {/if}
                                {#if dom}
                                    <span
                                        class={styles.cardSig}
                                        style="color:{dirColor(dom.direction)};border-color:{dirColor(dom.direction)}"
                                        title="{dom.kind} · {dom.direction} · {dom.status} · strength {(dom.strength * 100).toFixed(0)}%"
                                    >
                                        {signalLabel(dom)}
                                    </span>
                                {/if}
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}

        {:else if activeStage === 'Execution'}
            <!-- Execution: Confluence + Decision Context + Position -->
            <div class={styles.execLayout}>
                <!-- Position selector -->
                <div class={styles.execCard}>
                    <div class={styles.execCardTitle}>POSITION CONTEXT</div>
                    <div class={styles.positionSelector}>
                        <label class={activeStage === 'Execution' ? styles.posLabel : ''}>
                            <input type="radio" bind:group={app.currentPosition} value="None" /> None
                        </label>
                        <label>
                            <input type="radio" bind:group={app.currentPosition} value="Long" /> Long
                        </label>
                        <label>
                            <input type="radio" bind:group={app.currentPosition} value="Short" /> Short
                        </label>
                    </div>
                    {#if app.currentPosition !== 'None'}
                        <div class={styles.posInputs}>
                            <div class={styles.posField}>
                                <label for="twEp">Entry Price ($)</label>
                                <input id="twEp" type="number" step="any" bind:value={entryPrice} placeholder="0.00" />
                            </div>
                            <div class={styles.posField}>
                                <label for="twSl">Stop Loss ($)</label>
                                <input id="twSl" type="number" step="any" bind:value={stopLoss} placeholder="0.00" />
                            </div>
                        </div>
                    {/if}
                </div>

                <!-- Confluence across all timeframes -->
                <div class={styles.execCard}>
                    <div class={styles.execCardTitle}>CONFLUENCE · MTF SYNTHESIS</div>
                    <div class={styles.confluenceGrid}>
                        {#each TF_TABS as tf}
                            {@const tele = pair ? pair[tf.key] as TimeframeTelemetry : null}
                            {@const dc = tele?.decisionContext}
                            {#if dc}
                                <div class={styles.confluenceCell}>
                                    <span class={styles.confluenceCellLabel}>{tf.label}</span>
                                    <div class={styles.confluenceGauge}>
                                        <span class={styles.gaugeTrack}>
                                            <span class={styles.gaugeZero}></span>
                                            <span
                                                class={styles.gaugeFill}
                                                style="left:{dcBarLeft(dc.confluence)};width:{dcBarFill(dc.confluence)};background:{normColor(dc.confluence)}"
                                            ></span>
                                        </span>
                                        <span class={styles.gaugeVal} style="color:{normColor(dc.confluence)}">
                                            {dc.confluence >= 0 ? '+' : ''}{dc.confluence.toFixed(2)}
                                        </span>
                                    </div>
                                    <div class={styles.confluenceMetrics}>
                                        <span class={styles.cmItem}>Bias {dc.directional_bias >= 0 ? '+' : ''}{dc.directional_bias.toFixed(2)}</span>
                                        <span class={styles.cmItem}>Readiness {(dc.trade_readiness * 100).toFixed(0)}%</span>
                                        <span class={styles.cmItem}>Quality {(dc.trade_quality * 100).toFixed(0)}%</span>
                                    </div>
                                </div>
                            {:else}
                                <div class={styles.confluenceCell}>
                                    <span class={styles.confluenceCellLabel}>{tf.label}</span>
                                    <span class={styles.noData}>No data</span>
                                </div>
                            {/if}
                        {/each}
                    </div>
                </div>

                <!-- Decision Context detail (active timeframe) -->
                {#if activeTelemetry?.decisionContext}
                    {@const dc = activeTelemetry.decisionContext}
                    <div class={styles.execCard}>
                        <div class={styles.execCardTitle}>DECISION CONTEXT — {TF_TABS.find(t => t.key === activeTf)?.label}</div>
                        <div class={styles.dcGrid}>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Bullish Prob</span>
                                <span class={styles.dcVal} style="color: {normColor(dc.bullish_probability)}">
                                    {(dc.bullish_probability * 100).toFixed(0)}%
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Bearish Prob</span>
                                <span class={styles.dcVal} style="color: {normColor(-dc.bearish_probability)}">
                                    {(dc.bearish_probability * 100).toFixed(0)}%
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Bias</span>
                                <span class={styles.dcVal} style="color: {normColor(dc.directional_bias)}">
                                    {dc.directional_bias >= 0 ? '+' : ''}{dc.directional_bias.toFixed(2)}
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Consensus</span>
                                <span class={styles.dcVal} style="color: {normColor(dc.consensus)}">
                                    {dc.consensus >= 0 ? '+' : ''}{dc.consensus.toFixed(2)}
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Risk Level</span>
                                <span class={styles.dcVal} style="color: {normColor(-dc.risk_level)}">
                                    {(dc.risk_level * 100).toFixed(0)}%
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>R:R Ratio</span>
                                <span class={styles.dcVal} style="color: {dc.reward_risk_ratio >= 2 ? '#10b981' : dc.reward_risk_ratio >= 1 ? '#ffa726' : '#ef4444'}">
                                    {dc.reward_risk_ratio.toFixed(2)}
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Rec. Stop</span>
                                <span class={styles.dcVal}>{fmtUsd(dc.recommended_stop)}</span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Readiness</span>
                                <span class={styles.dcVal} style="color: {dc.trade_readiness >= 0.7 ? '#10b981' : dc.trade_readiness >= 0.4 ? '#ffa726' : '#ef4444'}">
                                    {(dc.trade_readiness * 100).toFixed(0)}%
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Quality</span>
                                <span class={styles.dcVal} style="color: {dc.trade_quality >= 0.7 ? '#10b981' : dc.trade_quality >= 0.4 ? '#ffa726' : '#ef4444'}">
                                    {(dc.trade_quality * 100).toFixed(0)}%
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Regime Conf</span>
                                <span class={styles.dcVal} style="color: {dc.regime_confidence >= 0.7 ? '#10b981' : dc.regime_confidence >= 0.4 ? '#ffa726' : '#ef4444'}">
                                    {(dc.regime_confidence * 100).toFixed(0)}%
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Trend Persist</span>
                                <span class={styles.dcVal} style="color: {dc.trend_persistence >= 0.7 ? '#10b981' : dc.trend_persistence >= 0.4 ? '#ffa726' : '#ef4444'}">
                                    {(dc.trend_persistence * 100).toFixed(0)}%
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Exp Range 1b</span>
                                <span class={styles.dcVal}>{dc.expected_range_1bar.toFixed(4)}</span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Market Quality</span>
                                <span class={styles.dcVal} style="color: {dc.market_quality >= 0.7 ? '#10b981' : '#ffa726'}">
                                    {(dc.market_quality * 100).toFixed(0)}%
                                </span>
                            </div>
                        </div>
                    </div>
                {:else}
                    <div class={styles.execCard}>
                        <div class={styles.emptyState}>Awaiting completed candle data for decision context.</div>
                    </div>
                {/if}
            </div>

        {:else if activeStage === 'Monitoring'}
            <div class={styles.monitorLayout}>
                {#if !monitoringData}
                    <div class={styles.emptyState}>Loading position data...</div>
                {:else if !monitoringData.has_active_position}
                    <div class={styles.emptyState}>
                        <span class={styles.emptyIcon}>○</span>
                        <span>No active position</span>
                        <span class={styles.mutedText}>Open a trade to begin monitoring</span>
                    </div>
                {:else}
                    <!-- Position Summary -->
                    <div class={styles.monitorCard}>
                        <div class={styles.execCardTitle}>POSITION SUMMARY</div>
                        <div class={styles.dcGrid}>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Direction</span>
                                <span class="{styles.dcVal} {monitoringData.direction === 'LONG' ? styles.pnlPos : styles.pnlNeg}">
                                    {monitoringData.direction}
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Avg Entry</span>
                                <span class={styles.dcVal}>{fmtUsd(monitoringData.average_entry_price)}</span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Mark Price</span>
                                <span class={styles.dcVal}>{fmtUsd(parseFloat(priceRef.toString()) || null)}</span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Total Size</span>
                                <span class={styles.dcVal}>{monitoringData.total_size?.toFixed(6) ?? '--'}</span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Unrealized PnL</span>
                                <span class="{styles.dcVal} {pnlClass(monitoringData.unrealized_pnl)}">
                                    {fmtUsd(monitoringData.unrealized_pnl)}
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>ROI</span>
                                <span class="{styles.dcVal} {pnlClass(monitoringData.unrealized_roi_pct)}">
                                    {fmtPct(monitoringData.unrealized_roi_pct)}
                                </span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Margin Used</span>
                                <span class={styles.dcVal}>{fmtUsd(monitoringData.margin_used)}</span>
                            </div>
                            <div class={styles.dcItem}>
                                <span class={styles.dcLabel}>Account Value</span>
                                <span class={styles.dcVal}>{fmtUsd(monitoringData.account_value)}</span>
                            </div>
                        </div>
                    </div>

                    <!-- Exit Signals -->
                    {#if monitoringData.exit_signals}
                        {@const es = monitoringData.exit_signals}
                        {@const oppScore = monitoringData.direction === 'LONG' ? es.opposite_score_short : es.opposite_score_long}
                        {@const thresh = es.opposite_exit_threshold}
                        {@const barPct = Math.min((oppScore / Math.max(thresh * 2, 1)) * 100, 100)}
                        {@const barClr = oppScore >= thresh ? '#ef5350' : oppScore >= thresh * 0.7 ? '#ffa726' : '#66bb6a'}
                        <div class={styles.monitorCard}>
                            <div class={styles.execCardTitle}>EXIT SIGNALS</div>
                            <div class={styles.exitInfo}>
                                <div class={styles.exitRow}>
                                    <span class={styles.exitLabel}>
                                        Opposite Score ({monitoringData.direction === 'LONG' ? 'SHORT' : 'LONG'} bias)
                                    </span>
                                    <div class={styles.gaugeTrack + ' ' + styles.exitTrack}>
                                        <span class={styles.gaugeFill} style="left:0;width:{barPct}%;background:{barClr}"></span>
                                    </div>
                                    <span class={styles.exitScore} style="color:{barClr}">{oppScore}/{thresh}</span>
                                </div>
                                {#if oppScore >= thresh}
                                    <span class={styles.exitWarning}>⚠ Exit signal triggered</span>
                                {:else if oppScore >= thresh * 0.7}
                                    <span class={styles.exitCaution}>⚡ Approaching threshold</span>
                                {:else}
                                    <span class={styles.exitSafe}>✓ Safe</span>
                                {/if}
                            </div>
                            {#if es.invalidation_level != null}
                                <div class={styles.exitInfo}>
                                    <span class={styles.exitLabel}>Invalidation Level: {fmtUsd(es.invalidation_level)}</span>
                                </div>
                            {/if}
                        </div>
                    {/if}

                    <!-- Slot Details -->
                    {#if monitoringData.slots?.length > 0}
                        <div class={styles.monitorCard}>
                            <div class={styles.execCardTitle}>SLOT DETAILS</div>
                            <div class={styles.slotsTable}>
                                <div class={styles.slotsHeader}>
                                    <span>Slot</span>
                                    <span>Entry</span>
                                    <span>Size</span>
                                    <span>PnL</span>
                                    <span>TPs</span>
                                </div>
                                {#each monitoringData.slots as slot, i}
                                    <div class={styles.slotRow}>
                                        <span>#{i + 1}</span>
                                        <span>{fmtUsd(slot.entry_price)}</span>
                                        <span>{slot.size?.toFixed(5)}</span>
                                        <span class={pnlClass(slot.unrealized_pnl)}>{fmtUsd(slot.unrealized_pnl)}</span>
                                        <span>
                                            {#if slot.take_profit_prices?.length}
                                                {slot.take_profit_prices.map((tp: number) => fmtUsd(tp)).join(' / ')}
                                            {:else}—{/if}
                                        </span>
                                    </div>
                                {/each}
                            </div>
                        </div>
                    {/if}

                    <!-- Trailing Stop + Safety -->
                    <div class={styles.monitorRow}>
                        <div class={styles.monitorCard + ' ' + styles.flexHalf}>
                            <div class={styles.execCardTitle}>TRAILING STOP</div>
                            {#if monitoringData.break_even_trail}
                                {#if monitoringData.break_even_trail.enabled && monitoringData.break_even_trail.trail_price}
                                    <span class={styles.trailActive}>● Active · Trail: {fmtUsd(monitoringData.break_even_trail.trail_price)}</span>
                                {:else}
                                    <span class={styles.trailInactive}>○ Inactive</span>
                                {/if}
                            {/if}
                        </div>
                        <div class={styles.monitorCard + ' ' + styles.flexHalf}>
                            <div class={styles.execCardTitle}>SAFETY STATE</div>
                            {#if monitoringData.safety_state}
                                {@const ss = monitoringData.safety_state}
                                <div class={styles.safetyRow}>
                                    <span class={styles.safetyItem}>
                                        Losses: <span class={ss.consecutive_losses >= ss.suspend_threshold ? styles.pnlNeg : ss.consecutive_losses >= ss.caution_threshold ? styles.exitCautionText : ''}>
                                            {ss.consecutive_losses}
                                        </span> (suspend @{ss.suspend_threshold})
                                    </span>
                                </div>
                            {/if}
                        </div>
                    </div>
                {/if}
                <button class={styles.refreshBtn} onclick={fetchMonitoring}>
                    {monitoringLoading ? '⟳' : '⟳ Refresh'}
                </button>
            </div>
        {/if}
    </div>
</div>
