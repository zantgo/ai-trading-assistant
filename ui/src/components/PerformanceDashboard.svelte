<script lang="ts">
    // PerformanceDashboard — v7.3 mode-aware shell on the shared MME
    // design vocabulary. Personality by the launch session mode:
    //   observe → "Edge Validator" (Overview + Backtesting + History +
    //             Methodology — the recorded-decision backtest surfaces)
    //   paper   → "Backtest + Forward Test" (drift vs the paper record)
    //   live    → "Performance Truth" (drift vs the live record)
    // Tab order follows the PAE layers: Overview · Trades (L1) · Strategy
    // (L2) · Risk (L3) · Performance (L4) · Backtesting (L5) · History ·
    // Methodology (cross-cutting last).
    import styles from '../styles/engine-dashboard.module.css';
    import { useAppStore } from '../state.svelte';
    import DashboardHeader from './DashboardHeader.svelte';
    import ModeChip from './ModeChip.svelte';
    import ModeBanner from './ModeBanner.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import { isExecutionMode, type ExecutionMode } from '../lib/modePresentation';
    import type {
        StrategyAnalyticsRow, RiskAnalyticsRow, PerformanceMatrixRow,
        OptimizationReport, TradeAnalyticsRecord,
    } from '../types/analytics';
    import OverviewTab from './performance/OverviewTab.svelte';
    import TradesTab from './performance/TradesTab.svelte';
    import StrategyTab from './performance/StrategyTab.svelte';
    import RiskTab from './performance/RiskTab.svelte';
    import PerformanceTab from './performance/PerformanceTab.svelte';
    import BacktestTab from './performance/BacktestTab.svelte';
    import HistoryTab from './performance/HistoryTab.svelte';
    import MethodologyTab from './performance/MethodologyTab.svelte';
    import NoInstanceState from './NoInstanceState.svelte';
    import PerformanceSettings from './PerformanceSettings.svelte';

    const app = useAppStore();

    let { section = 'overview' }: { section?: string } = $props();
    let loading = $state(false);
    let errorMsg = $state<string | null>(null);

    let dashboardStats = $state<any>(null);
    let strategyRows = $state<StrategyAnalyticsRow[]>([]);
    let riskData = $state<RiskAnalyticsRow | null>(null);
    let performanceRows = $state<PerformanceMatrixRow[]>([]);
    let optimizationReport = $state<OptimizationReport | null>(null);
    let tradeRecords = $state<TradeAnalyticsRecord[]>([]);

    // ── Backtesting state (v7: live /api/backtest/run) — lifted here so
    // the Overview drift card can reference the latest run.
    // v7.3: NO symbol fallback — with no instance active the dashboard
    // renders the no-instance empty state instead of a default symbol.
    const btSymbols = $derived(Object.keys(app.instancesMap));
    let btSymbol = $state('BTC-USDC');
    let btTimeframe = $state(60);
    let btStartDate = $state(new Date(Date.now() - 30 * 864e5).toISOString().slice(0, 10));
    let btEndDate = $state(new Date().toISOString().slice(0, 10));
    let btCapital = $state(1000);
    let btRunning = $state(false);
    let btError = $state('');
    let btResult = $state<{
        backtest_id: number;
        summary: { total_trades: number; win_count: number; loss_count: number; win_rate: number; gross_profit: number; gross_loss: number; profit_factor: number | null; expectancy: number; max_drawdown_pct: number };
        stats: StrategyAnalyticsRow;
        trades: { timestamp: number; direction: string; entry_price: number; exit_price: number; size: number; pnl: number; exit_reason: string }[];
        equity_curve: [number, number][];
    } | null>(null);

    // v7.2: the system-wide launch mode drives PAE framing.
    const mode = $derived<ExecutionMode | undefined>(
        app.sessionMode && isExecutionMode(app.sessionMode) ? app.sessionMode : undefined,
    );
    const observe = $derived(mode === 'observe');

    // v7.3: observe keeps the data-bearing tab set (Overview + Backtesting +
    // History + Methodology); Settings is always present in every mode.
    // Any other section falls back to Overview.
    const OBSERVE_SECTIONS = ['overview', 'backtesting', 'history', 'methodology', 'settings'];
    const safeSection = $derived(observe && !OBSERVE_SECTIONS.includes(section) ? 'overview' : section);

    const status = $derived<'live' | 'stale' | 'error' | 'loading'>(
        loading ? 'loading' : errorMsg ? 'error' : 'live',
    );

    async function runBacktest() {
        btRunning = true;
        btError = '';
        btResult = null;
        try {
            const fromMs = Date.parse(btStartDate);
            const toMs = Date.parse(btEndDate) + 864e5 - 1;
            if (!isFinite(fromMs) || !isFinite(toMs)) throw new Error('Invalid date range');
            const res = await fetch('/api/backtest/run', {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({
                    symbol: btSymbol,
                    timeframe_secs: Number(btTimeframe),
                    from_ms: fromMs,
                    to_ms: toMs,
                    initial_capital: Number(btCapital),
                }),
            });
            if (!res.ok) throw new Error('Backtest failed: HTTP ' + res.status);
            btResult = await res.json();
        } catch (e: any) {
            btError = e?.message ?? 'Backtest failed';
        } finally {
            btRunning = false;
        }
    }

    const sessionCapital = $derived(app.sessionCapital ?? 10000);

    async function fetchPanelData() {
        loading = true; errorMsg = null;
        try {
            const [statsRes, strategyRes, riskRes, perfRes, optRes, tradesRes] = await Promise.all([
                fetch(`/api/dashboard/stats?initial_capital=${sessionCapital}`),
                fetch('/api/analytics/strategy'),
                fetch('/api/analytics/risk'),
                fetch('/api/analytics/performance'),
                fetch('/api/analytics/optimization'),
                fetch('/api/analytics/trades?limit=200'),
            ]);
            if (statsRes.ok) dashboardStats = await statsRes.json();
            if (strategyRes.ok) strategyRows = await strategyRes.json();
            if (riskRes.ok) riskData = await riskRes.json();
            if (perfRes.ok) performanceRows = await perfRes.json();
            if (optRes.ok) optimizationReport = await optRes.json();
            if (tradesRes.ok) tradeRecords = await tradesRes.json();
        } catch (e: any) {
            errorMsg = e?.message ?? 'Failed to fetch analytics data';
        } finally {
            loading = false;
        }
    }

    $effect(() => { fetchPanelData(); });

    function headerTitle(s: string): string {
        const m: Record<string, string> = {
            overview: 'Performance Overview',
            trades: 'Trade Analytics',
            strategy: 'Strategy Analytics',
            risk: 'Risk Metrics',
            performance: 'Performance',
            backtesting: 'Backtesting',
            history: 'History',
            methodology: 'Methodology',
        };
        return m[s] ?? 'Performance';
    }

    function tabLabel(s: string): string {
        const m: Record<string, string> = {
            overview: 'Overview',
            trades: 'Trades',
            strategy: 'Strategy',
            risk: 'Risk',
            performance: 'Performance',
            backtesting: 'Backtesting',
            history: 'History',
            methodology: 'Methodology',
        };
        return m[s] ?? 'Overview';
    }

    // ── Export JSON: the current tab's shell-owned visible state ────────
    function buildExport(): string {
        let data: Record<string, unknown>;
        switch (safeSection) {
            case 'settings':
                data = { mode, note: 'settings payload is exported by the PerformanceSettings tab itself' };
                break;
            case 'trades':
                data = { mode, trade_records: tradeRecords };
                break;
            case 'strategy':
                data = { mode, strategy_rows: strategyRows };
                break;
            case 'risk':
                data = { mode, risk_data: riskData };
                break;
            case 'performance':
                data = { mode, performance_rows: performanceRows, optimization_report: optimizationReport };
                break;
            case 'methodology':
                data = { mode, analytics_config: null };
                break;
            default:
                data = {
                    mode,
                    observe,
                    dashboard_stats: dashboardStats,
                    risk_data: riskData,
                    last_backtest: btResult ? {
                        backtest_id: btResult.backtest_id,
                        summary: btResult.summary,
                        stats: btResult.stats,
                    } : null,
                };
        }
        return buildEngineExport('performance', safeSection, mode ?? null, data);
    }
</script>

<div class={styles.dashboard}>
    <div class={styles.content}>
        <DashboardHeader
            title={headerTitle(safeSection)}
            tabLabel={tabLabel(safeSection)}
            {status}
        >
            {#snippet trailing()}
                {#if mode}
                    <ModeChip {mode} />
                {/if}
                <ExportDataButton onExport={buildExport} title="Copy all data on this tab as JSON" />
            {/snippet}
        </DashboardHeader>

        <ModeBanner engine="performance" {mode} />

        {#if safeSection === 'settings'}
            <PerformanceSettings />
        {:else if Object.keys(app.instancesMap).length === 0 && !loading}
            <!-- v7.3: no active instance → SVG empty state. No fallback
                 symbol, no data, no loading message. -->
            <NoInstanceState engine="performance" />
        {:else if loading}
            <div class={styles.empty}>Loading analytics data…</div>
        {:else if safeSection === 'overview'}
            <OverviewTab
                {mode}
                {observe}
                {dashboardStats}
                {riskData}
                {btResult}
            />
        {:else if safeSection === 'trades'}
            <TradesTab {tradeRecords} />
        {:else if safeSection === 'strategy'}
            <StrategyTab {strategyRows} />
        {:else if safeSection === 'risk'}
            <RiskTab {riskData} />
        {:else if safeSection === 'performance'}
            <PerformanceTab {performanceRows} {optimizationReport} />
        {:else if safeSection === 'backtesting'}
            <BacktestTab
                {btSymbols}
                bind:btSymbol
                bind:btTimeframe
                bind:btStartDate
                bind:btEndDate
                bind:btCapital
                {btRunning}
                {btError}
                {btResult}
                {runBacktest}
            />
        {:else if safeSection === 'history'}
            <HistoryTab />
        {:else if safeSection === 'methodology'}
            <MethodologyTab />
        {/if}
    </div>
</div>
