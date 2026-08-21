<script lang="ts">
    // BacktestingDashboard — the Backtesting Engine shell (v8).
    //
    // Binds to ONE running instance via the shared app-store selection
    // (right-side Instances panel / Market Monitor Workspace tab — the
    // same mechanism the other engines use; no new picker).
    //
    // Navbar contract: with no running instance selected the navbar
    // collapses to Overview + History + Settings and re-charges
    // reactively the moment an instance is selected (safeSection clamps
    // to a visible tab, mirroring PerformanceDashboard).
    import { onMount } from 'svelte';
    import { useAppStore } from '../../state.svelte';
    import { isExecutionMode, type ExecutionMode } from '../../lib/modePresentation';
    import { BTE_TABS_NO_INSTANCE, ENGINE_TABS, type EngineKey } from '../../lib/engineTabs';
    import { buildEngineExport } from '../../lib/engineExport';
    import { fmtNum } from '../../lib/format';
    import styles from '../../styles/engine-dashboard.module.css';
    import DashboardHeader from '../DashboardHeader.svelte';
    import ModeChip from '../ModeChip.svelte';
    import ExportDataButton from '../ExportDataButton.svelte';
    import NoInstanceState from '../NoInstanceState.svelte';
    import BteRunForm from './BteRunForm.svelte';
    import BteCoverageTab from './BteCoverageTab.svelte';
    import BteStudyTab from './BteStudyTab.svelte';
    import BteExecutionsTab from './BteExecutionsTab.svelte';
    import BtePortfolioTab from './BtePortfolioTab.svelte';
    import BteStatsTab from './BteStatsTab.svelte';
    import BteSignalsTab from './BteSignalsTab.svelte';
    import BteHistoryTab from './BteHistoryTab.svelte';
    import BacktestSettings from './BacktestSettings.svelte';

    const app = useAppStore();
    let { section = 'overview' }: { section?: string } = $props();

    interface InstanceRow {
        id: string;
        pair: string;
        symbol: string;
        status: string;
        mode?: 'observe' | 'paper' | 'live';
    }
    interface CoverageRow {
        symbol: string;
        timeframe_secs: number;
        candle_count: number;
        earliest_secs: number | null;
        latest_secs: number | null;
        covered_span_secs: number;
        max_lookback_secs: number;
        coverage_pct: number;
    }
    export interface BteResult {
        backtest_id: number;
        mode?: string;
        params: { symbol: string; timeframe_secs: number; from_secs: number; to_secs: number; initial_capital: number };
        summary: {
            total_trades: number; win_count: number; loss_count: number; win_rate: number;
            gross_profit: number; gross_loss: number; profit_factor: number | null;
            expectancy: number; max_drawdown_pct: number;
        };
        stats: any;
        trades: { timestamp: number; direction: string; entry_price: number; exit_price: number; size: number; pnl: number; exit_reason: string }[];
        equity_curve: [number, number][];
    }

    let instances = $state<InstanceRow[]>([]);
    let loadingInstances = $state(true);
    let coverage = $state<CoverageRow[]>([]);
    let coverageDepth = $state<number>(180);
    let coverageError = $state<string>('');
    let ladder = $state<number[]>([60, 180, 300, 900]);
    let burnInSecs = $state<number>(0);
    let cfg = $state<{ backtest?: any; workspace?: any } | null>(null);

    // ── Backfill state (manual on the DIE tab + automatic on Run) ──
    let backfillJob = $state<{ job_id: number; status: string; pages_fetched: number; candles_stored: number; cursor_ts_secs: number | null; error: string | null; depth_days: number } | null>(null);
    let backfilling = $state(false);
    let backfillError = $state('');
    // Auto-prepare: the Run flow fetches missing archives before running.
    let preparing = $state(false);

    // ── Run form state (lifted for the study + export) ──
    let btTimeframe = $state(900);
    let btCapital = $state(1000);
    let btMode = $state<'recorded' | 'historical'>('historical');
    let depthDays = $state(180);
    let btRunning = $state(false);
    let btError = $state('');
    let btResult = $state<BteResult | null>(null);
    // DS payloads for the loaded study (portfolio/signals).
    let dsPortfolio = $state<{ run_id: number; portfolio: any[] } | null>(null);
    let dsSignals = $state<{ run_id: number; count: number; signals: any[] } | null>(null);

    // Session mode (BTE lives in observe sessions).
    const mode = $derived<ExecutionMode | undefined>(
        app.sessionMode && isExecutionMode(app.sessionMode) ? app.sessionMode : undefined,
    );

    // ── Instance binding (the shared selection, "like always") ──
    const boundInstance = $derived.by<InstanceRow | null>(() => {
        const sel = app.selectedInstance;
        if (!sel) return null;
        return instances.find((i) => i.pair === sel && i.status === 'running') ?? null;
    });

    const visibleTabs = $derived(boundInstance ? ENGINE_TABS.backtesting : BTE_TABS_NO_INSTANCE);
    const safeSection = $derived(
        visibleTabs.some((t) => t.key === section) ? section : 'overview',
    );

    async function fetchInstances() {
        try {
            const res = await fetch('/api/instances');
            if (res.ok) {
                const data = await res.json();
                instances = data.instances ?? [];
            }
        } catch (_) {}
        finally { loadingInstances = false; }
    }

    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (res.ok) {
                const data = await res.json();
                cfg = data;
                const ws = data?.workspace;
                if (data?.backtest?.archive_depth_days) depthDays = data.backtest.archive_depth_days;
                const slow = ws?.slow_timeframe?.duration_seconds ?? 300;
                const macro = ws?.macro_timeframe?.duration_seconds ?? 900;
                ladder = [60, 180, slow, macro];
                if (!ladder.includes(btTimeframe)) btTimeframe = macro;
            }
        } catch (_) {}
    }

    async function fetchCoverage() {
        const inst = boundInstance;
        if (!inst) return;
        try {
            const url = `/api/backtest/coverage?instance_id=${encodeURIComponent(inst.id)}`;
            const res = await fetch(url);
            if (res.ok) {
                const data = await res.json();
                coverage = data.archive ?? [];
                coverageDepth = data.archive_depth_days ?? coverageDepth;
                if (typeof data.burn_in_secs === 'number') burnInSecs = data.burn_in_secs;
                if (Array.isArray(data.ladder) && (data.ladder as number[]).length === 4) {
                    ladder = data.ladder;
                    if (!ladder.includes(btTimeframe)) btTimeframe = ladder[3];
                }
                coverageError = '';
            } else {
                coverageError = 'Coverage fetch failed: HTTP ' + res.status;
            }
        } catch (e: any) {
            coverageError = e?.message ?? 'Coverage fetch failed';
        }
    }

    // Recharge on instance selection changes + periodic backstop.
    $effect(() => {
        const _ = app.selectedInstance;
        const _l = ladder.length;
        void fetchCoverage();
    });

    onMount(() => {
        void fetchInstances();
        void fetchConfig();
        const timer = setInterval(() => { void fetchInstances(); }, 3000);
        return () => clearInterval(timer);
    });

    // ── Backfill (manual on the DIE tab; also used by auto-prepare) ──
    async function startBackfill() {
        const inst = boundInstance;
        if (!inst) return;
        backfilling = true;
        backfillError = '';
        backfillJob = null;
        try {
            const res = await fetch('/api/backtest/archive/backfill', {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({ instance_id: inst.id, depth_days: depthDays }),
            });
            const data = await res.json().catch(() => ({}));
            if (!res.ok) {
                let msg = data?.error ?? 'Backfill failed: HTTP ' + res.status;
                if (data?.hint) msg += ' — ' + data.hint;
                throw new Error(msg);
            }
            backfillJob = { job_id: data.job_id, status: 'running', pages_fetched: 0, candles_stored: 0, cursor_ts_secs: null, error: null, depth_days: depthDays };
            pollBackfill();
        } catch (e: any) {
            backfillError = e?.message ?? 'Backfill failed';
        } finally {
            backfilling = false;
        }
    }

    function pollBackfill() {
        const jobId = backfillJob?.job_id;
        if (!jobId) return;
        const timer = setInterval(async () => {
            try {
                const res = await fetch(`/api/backtest/archive/progress/${jobId}`);
                if (!res.ok) { clearInterval(timer); backfillError = 'Backfill progress lost (HTTP ' + res.status + ')'; return; }
                const data = await res.json();
                backfillJob = {
                    job_id: data.job_id,
                    status: data.status,
                    pages_fetched: data.pages_fetched,
                    candles_stored: data.candles_stored,
                    cursor_ts_secs: data.cursor_ts_secs,
                    error: data.error ?? null,
                    depth_days: data.depth_days,
                };
                if (data.status !== 'running') {
                    clearInterval(timer);
                    if (data.status === 'failed') backfillError = data.error ?? 'Backfill failed';
                    await fetchCoverage();
                    // Auto-prepare continuation: when the Run flow started
                    // the backfill, fire the actual run once coverage is
                    // sufficient.
                    if (preparing) {
                        preparing = false;
                        if (!backfillError && historicalCoverageReady()) {
                            await runNow();
                        } else {
                            btError = backfillError || 'Data preparation did not produce full four-timeframe coverage.';
                        }
                    }
                }
            } catch (_) {}
        }, 1000);
        void timer;
    }

    // Per-TF coverage vs the requested depth (all four ladder timeframes).
    function historicalCoverageReady(): boolean {
        const required = depthDays * 86400;
        return ladder.every((tf) => {
            const row = coverage.find((c) => c.timeframe_secs === tf);
            return row != null && (row.covered_span_secs ?? 0) >= required;
        });
    }

    // ── Run (depth-driven; auto-prepares missing archives) ──
    async function runBacktest() {
        const inst = boundInstance;
        if (!inst) return;
        if (btMode === 'historical' && !historicalCoverageReady()) {
            // Auto data preparation: fetch the four timeframe archives,
            // then run automatically when coverage is sufficient.
            preparing = true;
            btError = '';
            backfillError = '';
            await startBackfill();
            if (backfillError) {
                preparing = false;
                btError = backfillError;
            }
            return;
        }
        await runNow();
    }

    async function runNow() {
        const inst = boundInstance;
        if (!inst) return;
        btRunning = true;
        btError = '';
        btResult = null;
        dsPortfolio = null;
        dsSignals = null;
        try {
            // Depth-driven window: fetch `depthDays` of data; the first
            // burn-in portion warms the pipeline, the rest is scored.
            const toMs = Date.now();
            const fromMs = toMs - (depthDays * 864e5 - burnInSecs * 1000);
            if (fromMs >= toMs) throw new Error('Depth too small for the warmup window');
            const res = await fetch('/api/backtest/run', {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({
                    symbol: inst.symbol,
                    timeframe_secs: Number(btTimeframe),
                    from_ms: fromMs,
                    to_ms: toMs,
                    initial_capital: Number(btCapital),
                    instance_id: inst.id,
                    mode: btMode,
                }),
            });
            if (!res.ok) {
                let msg = 'Backtest failed: HTTP ' + res.status;
                try {
                    const err = await res.json();
                    if (err?.error) msg = String(err.error);
                    if (err?.hint) msg += ' — ' + String(err.hint);
                    if (err?.code) msg += ` (${err.code})`;
                } catch (_) {}
                throw new Error(msg);
            }
            const result = await res.json() as BteResult;
            btResult = result;
            await loadDsFor(result.backtest_id);
        } catch (e: any) {
            btError = e?.message ?? 'Backtest failed';
        } finally {
            btRunning = false;
        }
    }

    async function loadDsFor(runId: number) {
        try {
            const [pRes, sRes] = await Promise.all([
                fetch(`/api/backtest/${runId}/portfolio`),
                fetch(`/api/backtest/${runId}/signals`),
            ]);
            if (pRes.ok) dsPortfolio = await pRes.json();
            if (sRes.ok) dsSignals = await sRes.json();
        } catch (_) {}
    }

    async function loadRun(run: { id: number }) {
        try {
            const res = await fetch(`/api/backtest/${run.id}`);
            if (res.ok) {
                btResult = await res.json();
                await loadDsFor(run.id);
            }
        } catch (_) {}
    }

    function headerTitle(s: string): string {
        const m: Record<string, string> = {
            overview: 'Backtesting Overview',
            die: 'DIE · Archived Data',
            mme: 'MME · Simulated Signals',
            tae: 'TAE · Simulated Executions',
            pme: 'PME · Simulated Portfolio',
            pae: 'PAE · Statistical Treatment',
            study: 'Study Report',
            history: 'Backtest History',
            settings: 'Backtesting Settings',
        };
        return m[s] ?? 'Backtesting';
    }

    function tabLabel(s: string): string {
        const m: Record<string, string> = {
            overview: 'Overview', die: 'DIE · Data', mme: 'MME · Signals',
            tae: 'TAE · Executions', pme: 'PME · Portfolio', pae: 'PAE · Statistics',
            study: 'Study Report', history: 'History', settings: 'Settings',
        };
        return m[s] ?? 'Overview';
    }

    function buildExport(): string {
        return buildEngineExport('backtesting', safeSection, mode ?? null, {
            bound_instance: boundInstance ? { id: boundInstance.id, pair: boundInstance.pair, symbol: boundInstance.symbol } : null,
            coverage,
            depth_days: depthDays,
            burn_in_secs: burnInSecs,
            preparing,
            backfill: backfillJob,
            result: btResult ?? null,
        });
    }

    const coverageForTf = $derived.by(() => {
        const sym = boundInstance?.symbol;
        const map: Record<number, CoverageRow> = {};
        for (const row of coverage) {
            if (!sym || row.symbol === sym) map[row.timeframe_secs] = row;
        }
        return map;
    });
</script>

<div class={styles.dashboard}>
    <div class={styles.content}>
        <DashboardHeader
            title={headerTitle(safeSection)}
            tabLabel={tabLabel(safeSection)}
            status={loadingInstances ? 'loading' : 'live'}
        >
            {#snippet trailing()}
                {#if mode}
                    <ModeChip {mode} />
                {/if}
                {#if boundInstance}
                    <span class="{styles.badge} {styles.badgeNeutral}" title="Bound instance — select another from the right-side Instances panel">
                        {boundInstance.pair} · {boundInstance.id}
                    </span>
                {:else}
                    <span class="{styles.badge} {styles.badgeEmpty}">NO INSTANCE</span>
                {/if}
                {#if safeSection !== 'settings'}
                    <ExportDataButton onExport={buildExport} title="Copy the Backtesting Engine data as JSON" />
                {/if}
            {/snippet}
        </DashboardHeader>

        {#if safeSection === 'settings'}
            <BacktestSettings />
        {:else if !boundInstance}
            <NoInstanceState engine="backtesting" />
        {:else if safeSection === 'overview'}
            <BteRunForm
                bound={{ pair: boundInstance.pair, id: boundInstance.id, symbol: boundInstance.symbol }}
                {ladder}
                bind:btTimeframe
                bind:btCapital
                bind:btMode
                bind:depthDays
                {burnInSecs}
                {coverageForTf}
                {preparing}
                prepareProgress={backfillJob}
                {btRunning}
                {btError}
                {btResult}
                {runBacktest}
                maxDepth={coverageDepth}
            />
        {:else if safeSection === 'die'}
            <BteCoverageTab {coverage} {ladder} depthDays={coverageDepth} {backfillJob} {backfillError} {startBackfill} {backfilling} />
        {:else if safeSection === 'mme'}
            <BteSignalsTab signals={dsSignals?.signals ?? []} runId={btResult?.backtest_id ?? null} />
        {:else if safeSection === 'tae'}
            <BteExecutionsTab trades={btResult?.trades ?? []} result={btResult} />
        {:else if safeSection === 'pme'}
            <BtePortfolioTab portfolio={dsPortfolio?.portfolio ?? []} equity={btResult?.equity_curve ?? []} capital={btResult?.params?.initial_capital ?? 1000} />
        {:else if safeSection === 'pae'}
            <BteStatsTab stats={btResult?.stats ?? null} summary={btResult?.summary ?? null} />
        {:else if safeSection === 'study'}
            <BteStudyTab result={btResult} portfolio={dsPortfolio?.portfolio ?? []} signals={dsSignals?.signals ?? []} />
        {:else if safeSection === 'history'}
            <BteHistoryTab {loadRun} activeRunId={btResult?.backtest_id ?? null} />
        {/if}
    </div>
</div>
