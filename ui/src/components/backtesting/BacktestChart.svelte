<script lang="ts">
    // BacktestChart — v10: the backtest's input candles with the simulated
    // entries (arrows) and exits (markers colored by PnL sign) overlaid.
    // MICRO/FAST/SLOW/MACRO slot pills + symbol selector for multi-symbol runs.
    import { onMount } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, createSeriesMarkers } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time, UTCTimestamp, SeriesMarker } from 'lightweight-charts';
    import styles from './BacktestChart.module.css';

    interface Props {
        runId: number | null;
        defaultSymbol?: string | null;
    }
    let { runId, defaultSymbol = null }: Props = $props();

    let container = $state<HTMLDivElement | null>(null);
    let bars = $state<Bar[]>([]);
    let trades = $state<Trade[]>([]);
    let loading = $state(false);
    let error = $state<string | null>(null);

    const slots: { key: string; label: string }[] = [
        { key: 'micro', label: 'MICRO' },
        { key: 'fast', label: 'FAST' },
        { key: 'slow', label: 'SLOW' },
        { key: 'macro', label: 'MACRO' },
    ];
    let slot = $state('micro');
    let symbol = $state<string | null>(null);
    $effect(() => {
        if (defaultSymbol != null) symbol = defaultSymbol;
    });
    let symbols = $state<string[]>([]);

    interface Bar {
        symbol: string;
        timeframe_secs: number;
        ts_secs: number;
        open: number;
        high: number;
        low: number;
        close: number;
        volume: number;
    }
    interface Trade {
        ts_close_secs: number;
        ts_entry_secs: number;
        direction: string;
        entry_price: number;
        exit_price: number;
        pnl: number;
        exit_reason: string;
    }

    // Slot → typical timeframe mapping (config-driven ladders map via the
    // distinct timeframes found in the run's bars, largest = macro).
    const slotSeconds = $derived.by(() => {
        const tfs = [...new Set(bars.map((b) => b.timeframe_secs))].sort((a, b) => a - b);
        const out: Record<string, number> = {};
        if (tfs.length === 1) { out.micro = tfs[0]; out.fast = tfs[0]; out.slow = tfs[0]; out.macro = tfs[0]; }
        else if (tfs.length === 2) { out.micro = tfs[0]; out.fast = tfs[0]; out.slow = tfs[1]; out.macro = tfs[1]; }
        else if (tfs.length === 3) { out.micro = tfs[0]; out.fast = tfs[1]; out.slow = tfs[2]; out.macro = tfs[2]; }
        else { out.micro = tfs[0]; out.fast = tfs[1]; out.slow = tfs[2]; out.macro = tfs[3]; }
        return out;
    });

    const visibleBars = $derived(bars.filter((b) => b.timeframe_secs === slotSeconds[slot]));

    let chart: IChartApi | null = null;
    let candleSeries: ISeriesApi<'Candlestick'> | null = null;

    onMount(() => {
        if (!container || !runId) return;
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal },
            rightPriceScale: { borderColor: '#2a2e39' },
            timeScale: { borderColor: '#2a2e39', timeVisible: true, secondsVisible: false },
            handleScale: true,
            handleScroll: true,
        });
        candleSeries = chart.addSeries(CandlestickSeries, {
            upColor: '#26a69a', downColor: '#ef5350', borderVisible: false,
            wickUpColor: '#26a69a', wickDownColor: '#ef5350',
        });
        void fetchData();
    });

    async function fetchData() {
        if (!runId) return;
        loading = true; error = null;
        try {
            const [barsRes, tradesRes] = await Promise.all([
                fetch(`/api/backtest/${runId}/input_bars`),
                fetch(`/api/backtest/${runId}/trades?limit=5000`),
            ]);
            if (!barsRes.ok || !tradesRes.ok) throw new Error('backtest data fetch failed');
            const barsData = await barsRes.json();
            const tradesData = await tradesRes.json();
            bars = barsData.bars ?? [];
            trades = tradesData.trades ?? [];
            const syms = [...new Set(bars.map((b: Bar) => b.symbol))] as string[];
            symbols = syms;
            if (syms.length > 0 && (!symbol || !syms.includes(symbol))) symbol = syms[0];
        } catch (e: any) {
            error = e?.message ?? 'failed to load chart data';
        } finally {
            loading = false;
        }
    }

    $effect(() => {
        const c = candleSeries;
        if (!c || visibleBars.length === 0) return;
        const selectedSymbol = symbol;
        c.setData(visibleBars
            .filter((b) => !selectedSymbol || b.symbol === selectedSymbol)
            .map((b) => ({
                time: b.ts_secs as UTCTimestamp,
                open: b.open, high: b.high, low: b.low, close: b.close,
            })));
        const entryMarkers = trades
            .filter((t) => t.ts_entry_secs > 0)
            .map((t) => ({
                time: t.ts_entry_secs as UTCTimestamp,
                position: t.direction === 'LONG' ? 'belowBar' as const : 'aboveBar' as const,
                color: '#fdd835',
                shape: t.direction === 'LONG' ? 'arrowUp' as const : 'arrowDown' as const,
                text: `${t.direction} @ ${t.entry_price.toFixed(2)}`,
            }));
        const exitMarkers: SeriesMarker<Time>[] = trades.map((t) => ({
            time: t.ts_close_secs as UTCTimestamp,
            position: 'aboveBar' as const,
            color: t.pnl >= 0 ? '#26a69a' : '#ef5350',
            shape: 'circle' as const,
            text: `${t.exit_reason} · ${t.pnl >= 0 ? '+' : ''}${t.pnl.toFixed(2)}`,
        }));
        createSeriesMarkers(c, [...entryMarkers, ...exitMarkers]);
        if (chart) chart.timeScale().fitContent();
    });
</script>

<div class={styles.wrap}>
    <div class={styles.controls}>
        <div class={styles.pills}>
            {#each slots as s (s.key)}
                <button
                    class="{styles.pill} {slot === s.key ? styles.pillActive : ''}"
                    onclick={() => (slot = s.key)}
                    disabled={!slotSeconds[s.key]}
                    title={`${slotSeconds[s.key] ?? '—'}s candles`}
                >
                    {s.label}
                    {#if slotSeconds[s.key]}
                        <span class={styles.pillTf}>{slotSeconds[s.key]}s</span>
                    {/if}
                </button>
            {/each}
        </div>
        {#if symbols.length > 1}
            <select class={styles.symbolSelect} bind:value={symbol}>
                {#each symbols as s (s)}
                    <option value={s}>{s}</option>
                {/each}
            </select>
        {/if}
    </div>

    {#if loading}
        <div class={styles.state}>Loading chart data…</div>
    {:else if error}
        <div class={styles.state}>{error}</div>
    {:else if !runId}
        <div class={styles.state}>Run a backtest to render its chart.</div>
    {/if}

    <div class={styles.chart} bind:this={container}></div>
</div>
