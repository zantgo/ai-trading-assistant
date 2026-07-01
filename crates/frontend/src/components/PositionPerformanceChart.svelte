<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, LineSeries, type IChartApi, type ISeriesApi, type IPriceLine, type Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import styles from './PositionPerformanceChart.module.css';

    const app = useAppStore();

    let chartContainer: HTMLDivElement;
    let chart: IChartApi | null = null;
    let series: ISeriesApi<'Line'> | null = null;
    let isFullscreen = $state(false);
    let ro: ResizeObserver;

    let selectedTimeframe = $state<'1H' | '1D' | '1W' | '1M' | '1Y' | 'ALL'>('ALL');
    let lastTimeframe = $state('');
    let marginLine: IPriceLine | null = null;

    // Derived chart data combining historical database snapshots and live position valuation
    const chartData = $derived.by(() => {
        const nowMs = Date.now();
        let cutoffMs = 0;
        if (selectedTimeframe === '1H') cutoffMs = nowMs - 60 * 60 * 1000;
        else if (selectedTimeframe === '1D') cutoffMs = nowMs - 24 * 60 * 60 * 1000;
        else if (selectedTimeframe === '1W') cutoffMs = nowMs - 7 * 24 * 60 * 60 * 1000;
        else if (selectedTimeframe === '1M') cutoffMs = nowMs - 30 * 24 * 60 * 60 * 1000;
        else if (selectedTimeframe === '1Y') cutoffMs = nowMs - 365 * 24 * 60 * 60 * 1000;

        let filteredSnapshots = app.equitySnapshots;
        if (selectedTimeframe !== 'ALL') {
            filteredSnapshots = app.equitySnapshots.filter(s => s.timestamp >= cutoffMs);
        }

        let data = filteredSnapshots.map((s: { timestamp: number; equity_value: number }) => ({
            time: Math.floor(s.timestamp / 1000) as Time,
            value: s.equity_value,
        }));

        // Sort chronologically to prevent rendering errors
        data.sort((a, b) => (a.time as number) - (b.time as number));

        const pos = app.activePaperPosition;
        if (pos) {
            const entryTimeSec = Math.floor((pos.entry_timestamp as number || Date.now()) / 1000);
            const initialMargin = (pos.initial_allocated_margin as number) || (pos.allocated_usd as number) || 0;
            const currentVal = (pos.allocated_usd as number || 0) + app.paperUnrealizedPnl;

            // Synthesize the starting point if no historical database entries exist yet
            if (data.length === 0) {
                data.push({
                    time: entryTimeSec as Time,
                    value: initialMargin,
                });
            }

            // Append the current live valuation point
            const nowSec = Math.floor(Date.now() / 1000);
            const lastTime = data[data.length - 1].time as number;
            const liveTime = Math.max(nowSec, lastTime + 1);

            data.push({
                time: liveTime as Time,
                value: currentVal,
            });
        }

        // Pad to at least two points to guarantee that the line is rendered
        if (data.length === 1) {
            const single = data[0];
            data.push({
                time: ((single.time as number) + 1) as Time,
                value: single.value,
            });
        }

        return data;
    });

    // Derive current active position equity
    const latestEquity = $derived.by(() => {
        const pos = app.activePaperPosition;
        if (pos) {
            return (pos.allocated_usd as number || 0) + app.paperUnrealizedPnl;
        }
        return chartData.length > 0 ? chartData[chartData.length - 1].value : 0;
    });

    const isPositive = $derived(latestEquity >= app.paperMarginUsed || app.paperUnrealizedPnl >= 0);

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
        const w = chartContainer.clientWidth || 300;
        const h = chartContainer.clientHeight || 200;

        chart = createChart(chartContainer, {
            autoSize: true,
            layout: {
                background: { color: 'transparent' },
                textColor: '#8f929d',
                fontSize: 10
            },
            grid: {
                vertLines: { color: '#1e1e3a' },
                horzLines: { color: '#1e1e3a' },
            },
            timeScale: {
                borderColor: '#2a2a4a',
                visible: true,
                timeVisible: true,
                secondsVisible: false,
            },
            rightPriceScale: {
                borderColor: '#2a2a4a',
                scaleMargins: { top: 0.15, bottom: 0.15 },
            },
            crosshair: {
                vertLine: { color: '#3b82f6', labelBackgroundColor: '#3b82f6' },
                horzLine: { color: '#3b82f6', labelBackgroundColor: '#3b82f6' },
            },
            handleScale: true,
            handleScroll: true,
            width: w,
            height: isFullscreen ? window.innerHeight - 80 : h,
        });

        series = chart.addSeries(LineSeries, {
            color: '#3b82f6',
            lineWidth: 3,
            priceLineVisible: false,
            crosshairMarkerVisible: true,
        });

        if (chartData.length > 0) {
            series.setData(chartData);
        }

        chart.timeScale().fitContent();
        return chart;
    }

    function destroyChart() {
        if (chart) { chart.remove(); chart = null; }
    }

    onMount(() => {
        buildChart();
        ro = new ResizeObserver(() => {
            const w = chartContainer.clientWidth;
            const h = chartContainer.clientHeight;
            if (chart && w > 0 && h > 0) {
                chart.resize(w, isFullscreen ? window.innerHeight - 80 : h);
            }
        });
        if (chartContainer) {
            ro.observe(chartContainer);
        }

        const handleResize = () => {
            if (chart && chartContainer) {
                chart.applyOptions({
                    width: isFullscreen ? window.innerWidth - 80 : chartContainer.clientWidth,
                    height: isFullscreen ? window.innerHeight - 80 : chartContainer.clientHeight || 200,
                });
            }
        };
        window.addEventListener('resize', handleResize);
        return () => {
            window.removeEventListener('resize', handleResize);
            destroyChart();
            ro?.disconnect();
        };
    });

    // Handle timeframe switching and fit content updates
    $effect(() => {
        if (!chart && chartContainer) buildChart();
        if (chart && series && chartData.length > 0) {
            series.setData(chartData);
            const timeframeChanged = lastTimeframe !== selectedTimeframe;
            lastTimeframe = selectedTimeframe;
            if (timeframeChanged) {
                chart.timeScale().fitContent();
            }
        }
    });

    // Render a constant horizontal baseline showing Margin allocation
    $effect(() => {
        if (!series) return;
        if (marginLine) {
            series.removePriceLine(marginLine);
            marginLine = null;
        }

        const pos = app.activePaperPosition;
        const margin = app.paperMarginUsed || (pos?.initial_allocated_margin as number) || (pos?.allocated_usd as number) || 0;

        if (margin > 0) {
            marginLine = series.createPriceLine({
                price: margin,
                color: '#3b82f6',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: 'Margin Baseline',
            });
        }
    });

    function toggleFullscreen() {
        isFullscreen = !isFullscreen;
        if (chart) {
            chart.applyOptions({
                height: isFullscreen ? window.innerHeight - 80 : chartContainer?.clientHeight ?? 200,
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
        <div class={styles.headerLeft}>
            <span class={styles.perfTitle}>Position Value (USDT)</span>
            <span class="{styles.perfValue} {isPositive ? styles.green : styles.red}">
                {fmt(latestEquity)}
            </span>
        </div>

        <!-- Interactive Timeframe Filter Tabs -->
        <div class={styles.timeframeTabs}>
            <button class="{styles.timeframeBtn} {selectedTimeframe === '1H' ? styles.active : ''}" onclick={() => selectedTimeframe = '1H'}>1H</button>
            <button class="{styles.timeframeBtn} {selectedTimeframe === '1D' ? styles.active : ''}" onclick={() => selectedTimeframe = '1D'}>1D</button>
            <button class="{styles.timeframeBtn} {selectedTimeframe === '1W' ? styles.active : ''}" onclick={() => selectedTimeframe = '1W'}>1W</button>
            <button class="{styles.timeframeBtn} {selectedTimeframe === '1M' ? styles.active : ''}" onclick={() => selectedTimeframe = '1M'}>1M</button>
            <button class="{styles.timeframeBtn} {selectedTimeframe === '1Y' ? styles.active : ''}" onclick={() => selectedTimeframe = '1Y'}>1Y</button>
            <button class="{styles.timeframeBtn} {selectedTimeframe === 'ALL' ? styles.active : ''}" onclick={() => selectedTimeframe = 'ALL'}>ALL</button>
        </div>
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
