<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type {
        TimeframeTelemetry, IndicatorMeta, IndicatorSignal, SignalKind
    } from '../types';
    import TelemetryTable from './TelemetryTable.svelte';
    import LiquidityPanel from './LiquidityPanel.svelte';
    import styles from './TerminalMonitor.module.css';
    import { getIcon } from '../lib/icons';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);

    type TfLabel = 'Micro' | 'Fast' | 'Slow' | 'Macro';
    let activeTf: TfLabel = $state('Micro');

    const TIMEFRAMES: { key: TfLabel; label: string; tfKey: string; secs: number }[] = [
        { key: 'Micro', label: 'Micro', tfKey: 'microTerm', secs: 60 },
        { key: 'Fast',  label: 'Fast',  tfKey: 'fastTerm',  secs: 180 },
        { key: 'Slow',  label: 'Slow',  tfKey: 'slowTerm',  secs: 300 },
        { key: 'Macro', label: 'Macro', tfKey: 'macroTerm', secs: 900 },
    ];

    const activeTfEntry = $derived(TIMEFRAMES.find(t => t.key === activeTf)!);
    const activeTfObj = $derived<TimeframeTelemetry | undefined>(
        (pair as any)?.[activeTfEntry.tfKey] as TimeframeTelemetry | undefined
    );

    const hasLiquidity = $derived(!!activeTfObj?.liquidity || !!activeTfObj?.cluster);

    const SIGNAL_KIND_ORDER: SignalKind[] = [
        'Divergence', 'Crossover', 'Threshold', 'Breakout', 'BandTouch',
        'ZeroLineCross', 'CompressionRelease', 'LevelTest', 'TrendFlip',
        'VolumeClimax', 'StackChange', 'PatternForming',
    ];

    const SIGNAL_ABBR: Record<string, string> = {
        Divergence: 'DIV', Crossover: 'CRO', Threshold: 'TH', Breakout: 'BO',
        BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
        LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
        StackChange: 'STK', PatternForming: 'PAT',
    };

    function indicatorName(key: string): string {
        const meta = registry.find(m => m.key === key);
        return meta?.display_name ?? key;
    }

    function confidence(tf: TimeframeTelemetry, key: string): number {
        return Math.round((tf.indicators?.[key]?.confidence ?? 0) * 100);
    }

    function signalStyle(s: IndicatorSignal): string {
        if (s.direction === 'Bullish') return 'bull';
        if (s.direction === 'Bearish') return 'bear';
        return 'neutral';
    }

    function signalAge(s: IndicatorSignal): string {
        const a = s.age_bars ?? 0;
        return a === 0 ? 'now' : `${a}b`;
    }

    function getSignalsForKind(kind: SignalKind): Array<{ indicatorKey: string; displayName: string; signal: IndicatorSignal }> {
        if (!activeTfObj) return [];
        const results: Array<{ indicatorKey: string; displayName: string; signal: IndicatorSignal }> = [];
        for (const meta of registry) {
            const signals = activeTfObj.indicators?.[meta.key]?.signals ?? [];
            for (const s of signals) {
                if (s.kind === kind) {
                    results.push({
                        indicatorKey: meta.key,
                        displayName: meta.display_name,
                        signal: s,
                    });
                }
            }
        }
        return results.sort((a, b) => b.signal.strength - a.signal.strength);
    }

    let expandedKinds = $state<Record<string, boolean>>({});

    function toggleKind(kind: string) {
        expandedKinds = { ...expandedKinds, [kind]: !expandedKinds[kind] };
    }

    let liquidityOpen = $state(false);
</script>

<div class={styles.monitor}>
    <div class={styles.tfSidebar}>
        <h3 class={styles.tfSidebarTitle}>TIMEFRAMES</h3>
        {#each TIMEFRAMES as tf (tf.key)}
            <button
                class={styles.tfSidebarItem}
                class:active={activeTf === tf.key}
                onclick={() => activeTf = tf.key}
            >
                <span class={styles.tfLabel}>{tf.label}</span>
                <span class={styles.tfSecs}>{tf.secs}s</span>
            </button>
        {/each}
    </div>

    <div class={styles.contentArea}>
        {#if pair && registry.length > 0}
            <div class={styles.header}>
                <span class={styles.title}>METRICS</span>
                <span class={styles.symbol}>{app.pairDisplayFor(pair.symbol)}</span>
                <span class={styles.tfBadge}>{activeTfEntry.label} · {activeTfEntry.secs}s</span>
            </div>

            {#each SIGNAL_KIND_ORDER as kind}
                {@const sigs = getSignalsForKind(kind)}
                {#if sigs.length > 0}
                    <div class={styles.signalKindCard}>
                        <button class={styles.kindHeader} onclick={() => toggleKind(kind)}>
                            <span class={styles.kindCaret}>{expandedKinds[kind] !== false ? '▼' : '▶'}</span>
                            <span class={styles.kindName}>{kind}</span>
                            <span class={styles.kindCount}>{sigs.length} signal{sigs.length > 1 ? 's' : ''}</span>
                            <span class={styles.kindAbbr}>{SIGNAL_ABBR[kind]}</span>
                        </button>
                        {#if expandedKinds[kind] !== false}
                            <div class={styles.kindSignals}>
                                {#each sigs as entry (entry.indicatorKey)}
                                    <div class={styles.signalRow}>
                                        <span class={styles.signalIndicator}>{entry.displayName}</span>
                                        <span class="{styles.signalDir} {styles[signalStyle(entry.signal)]}">
                                            {entry.signal.direction}
                                        </span>
                                        <span class={styles.signalStatus}>{entry.signal.status}</span>
                                        <span class={styles.signalMeta}>
                                            str {(entry.signal.strength * 100).toFixed(0)} ·
                                            conf {confidence(activeTfObj!, entry.indicatorKey)}% ·
                                            age {signalAge(entry.signal)}
                                        </span>
                                        <span class={styles.signalLabel}>{entry.signal.label}</span>
                                    </div>
                                {/each}
                            </div>
                        {/if}
                    </div>
                {/if}
            {/each}

            <div class={styles.sectionDivider}></div>

            <TelemetryTable {pairKey} tfKey={activeTfEntry.tfKey} tfSecs={activeTfEntry.secs} />

            {#if hasLiquidity}
                <div class={styles.sectionDivider}></div>
                <div class={styles.liquiditySection}>
                    <button class={styles.liquidityHeader} onclick={() => liquidityOpen = !liquidityOpen}>
                        <span class={styles.kindCaret}>{liquidityOpen ? '▼' : '▶'}</span>
                        <span>Liquidity Data</span>
                        <span class={styles.tfBadge}>cascade · cluster · flow</span>
                    </button>
                    {#if liquidityOpen}
                        <div class={styles.liquidityBody}>
                            <LiquidityPanel {pairKey} />
                        </div>
                    {/if}
                </div>
            {/if}
        {:else}
            <div class={styles.featurePlaceholder}>
                {@html getIcon('tableChart', 64)}
                <h2 class={styles.featurePlaceholderTitle}>Market Metrics</h2>
                <p class={styles.featurePlaceholderMsg}>
                    Awaiting indicator registry and market data...
                </p>
            </div>
        {/if}
    </div>
</div>
