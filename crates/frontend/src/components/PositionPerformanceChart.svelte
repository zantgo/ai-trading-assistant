<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, ColorType, LineSeries, type IChartApi } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import styles from './PositionPerformanceChart.module.css';

    const app = useAppStore();

    let chartContainer: HTMLDivElement;
    let chart: IChartApi | null = null;
    let isFullscreen = $state(false);

    const equityData = $derived(
        app.equitySnapshots
            .map((s: { timestamp: number; equity_value: number }) => ({
                time: Math.floor(s.timestamp / 1000) as import('lightweight-charts').UTCTimestamp,
                value: s.equity_value,
            }))
            .sort((a, b) => a.time - b.time)
    );

    const latestEquity = $derived(equityData.length > 0 ? equityData[equityData.length - 1].value : 0);
    const isPositive = $derived(latestEquity > app.paperMarginUsed || app.paperUnrealizedPnl >= 0);

    const marginUsed = $derived(app.paperMarginUsed);
    const unrealizedPnl = $derived(app.paperUnrealizedPnl);
    const realizedPnl = $derived(app.paperRealizedPnlAccumulator);

    const donutTotal = $derived(Math.abs(marginUsed) + Math.abs(unrealizedPnl) + Math.abs(realizedPnl) || 1);
    const marginDeg = $derived((Math.abs(marginUsed) / donutTotal) * 360);
    const unrealizedDeg = $derived((Math.abs(unrealizedPnl) / donutTotal) * 360);

    function fmt(n: number): string {
        if (!isFinite(n)) return '$0.00';
        return (n >= 0 ? '+' : '') + '$' + n.toFixed(2);
    }

    function buildChart() {
        if (!chartContainer) return;
        chart = createChart(chartContainer, {
            layout: {
                background: { type: ColorType.Solid, color: '#131722' },
                textColor: '#64748b',
            },
            grid: {
                vertLines: { color: '#1a1f2e' },
                horzLines: { color: '#1a1f2e' },
            },
            timeScale: {
                borderColor: '#2a2e39',
                timeVisible: true,
            },
            rightPriceScale: {
                borderColor: '#2a2e39',
            },
            crosshair: {
                vertLine: { color: '#3b82f6', labelBackgroundColor: '#3b82f6' },
                horzLine: { color: '#3b82f6', labelBackgroundColor: '#3b82f6' },
            },
            height: isFullscreen ? window.innerHeight - 80 : 200,
            width: isFullscreen ? window.innerWidth - 80 : chartContainer.clientWidth,
        });

        const series = chart.addSeries(LineSeries, {
            color: isPositive ? '#10b981' : '#ef4444',
            lineWidth: 2,
            priceLineVisible: false,
        });

        if (equityData.length > 0) {
            series.setData(equityData);
        }

        chart.timeScale().fitContent();
        return chart;
    }

    function destroyChart() {
        if (chart) { chart.remove(); chart = null; }
    }

    onMount(() => {
        buildChart();
        const handleResize = () => {
            if (chart && chartContainer) {
                chart.applyOptions({
                    width: isFullscreen ? window.innerWidth - 80 : chartContainer.clientWidth,
                    height: isFullscreen ? window.innerHeight - 80 : 200,
                });
            }
        };
        window.addEventListener('resize', handleResize);
        return () => {
            window.removeEventListener('resize', handleResize);
            destroyChart();
        };
    });

    $effect(() => {
        if (!chart && chartContainer) buildChart();
        if (chart && equityData.length > 0) {
            const series = chart.series()[0] as ReturnType<typeof chart.addSeries>;
            series.setData(equityData);
            chart.timeScale().fitContent();
        }
    });

    function toggleFullscreen() {
        isFullscreen = !isFullscreen;
        if (chart) {
            chart.applyOptions({
                height: isFullscreen ? window.innerHeight - 80 : 200,
                width: isFullscreen ? window.innerWidth - 80 : chartContainer?.clientWidth ?? 600,
            });
        }
    }

    function handleWindowKeydown(e: KeyboardEvent) {
        if (isFullscreen && e.key === 'Escape') {
            isFullscreen = false;
        }
    }

    onDestroy(destroyChart);
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if isFullscreen}
    <div class={styles.fullscreenBackdrop} onclick={toggleFullscreen} role="presentation">
        <div class={styles.fullscreenContent} onclick={(e) => e.stopPropagation()} role="dialog">
            <div class={styles.fullscreenHeader}>
                <span>Position Performance — {app.activeTab}</span>
                <button class={styles.closeBtn} onclick={toggleFullscreen}>✕</button>
            </div>
            <div bind:this={chartContainer} class={styles.fullscreenChart}></div>
        </div>
    </div>
{/if}

<div class={styles.perfContainer} ondblclick={toggleFullscreen}>
    <div class={styles.perfHeader}>
        <span class={styles.perfTitle}>Position Value (USDT)</span>
        <span class="{styles.perfValue} {isPositive ? styles.green : styles.red}">
            {fmt(latestEquity)}
        </span>
    </div>

    <div class={styles.chartRow}>
        <div bind:this={chartContainer} class={styles.chartArea}></div>

        <!-- Donut Chart SVG -->
        <div class={styles.donutWrap}>
            <svg viewBox="0 0 100 100" class={styles.donutSvg}>
                <!-- Margin Used (blue) -->
                <circle cx="50" cy="50" r="38" fill="none" stroke="#1e293b" stroke-width="12" />
                <circle cx="50" cy="50" r="38" fill="none" stroke="#3b82f6" stroke-width="12"
                    stroke-dasharray="{marginDeg * 0.664} {360 * 0.664 - marginDeg * 0.664}"
                    stroke-dashoffset="{0}" transform="rotate(-90 50 50)" />
                <!-- Unrealized PnL (green/red) -->
                <circle cx="50" cy="50" r="38" fill="none"
                    stroke={unrealizedPnl >= 0 ? '#10b981' : '#ef4444'} stroke-width="12"
                    stroke-dasharray="{unrealizedDeg * 0.664} {360 * 0.664 - unrealizedDeg * 0.664}"
                    stroke-dashoffset="{-marginDeg * 0.664}" transform="rotate(-90 50 50)" />
                <!-- Inner circle -->
                <circle cx="50" cy="50" r="24" fill="#131722" />
                <text x="50" y="46" text-anchor="middle" fill="#f1f5f9" font-size="9" font-weight="700">ROI</text>
                <text x="50" y="58" text-anchor="middle" fill={unrealizedPnl >= 0 ? '#10b981' : '#ef4444'}
                    font-size="8" font-weight="800">{app.paperUnrealizedRoi.toFixed(1)}%</text>
            </svg>

            <div class={styles.donutLegend}>
                <div class={styles.legendItem}>
                    <span class={styles.legendDot + ' ' + styles.dotBlue}></span>
                    <span class={styles.legendLabel}>Margin ${marginUsed.toFixed(2)}</span>
                </div>
                <div class={styles.legendItem}>
                    <span class={styles.legendDot} style="background:{unrealizedPnl >= 0 ? '#10b981' : '#ef4444'};"></span>
                    <span class={styles.legendLabel}>Unreal. {fmt(unrealizedPnl)}</span>
                </div>
                <div class={styles.legendItem}>
                    <span class={styles.legendDot + ' ' + styles.dotGray}></span>
                    <span class={styles.legendLabel}>Realized {fmt(realizedPnl)}</span>
                </div>
            </div>
        </div>
    </div>
</div>
