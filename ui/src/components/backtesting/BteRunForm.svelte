<script lang="ts">
    // BteRunForm — the BTE Overview: depth-driven run form (v8.1).
    //
    // The archive depth (1..=365 days) is the ONLY window control: the
    // backtest window derives from it — `[now − days + burn_in, now]` —
    // and the four ladder timeframes (micro/fast/slow/macro) are fetched
    // AUTOMATICALLY on Run when coverage is short. Start/End dates and
    // the manual Backfill button are gone from this surface.
    import styles from '../../styles/engine-dashboard.module.css';
    import KpiStrip from '../KpiStrip.svelte';
    import { fmtNum, fmtSigned } from '../../lib/format';
    import type { BteResult } from './BacktestingDashboard.svelte';

    interface Props {
        bound: { pair: string; id: string; symbol: string };
        ladder: number[];
        btTimeframe: number;
        btCapital: number;
        btMode: 'recorded' | 'historical';
        depthDays: number;
        burnInSecs: number;
        coverageForTf: Record<number, {
            candle_count: number; earliest_secs: number | null; latest_secs: number | null;
            covered_span_secs: number; max_lookback_secs: number; coverage_pct: number;
        }>;
        preparing: boolean;
        prepareProgress: { pages_fetched: number; candles_stored: number } | null;
        btRunning: boolean;
        btError: string;
        btResult: BteResult | null;
        runBacktest: () => Promise<void>;
        maxDepth: number;
    }

    let {
        bound,
        ladder,
        btTimeframe = $bindable(),
        btCapital = $bindable(),
        btMode = $bindable(),
        depthDays = $bindable(),
        burnInSecs,
        coverageForTf,
        preparing,
        prepareProgress,
        btRunning,
        btError,
        btResult,
        runBacktest,
        maxDepth,
    }: Props = $props();

    const MIN_DEPTH = 1;
    const MAX_DEPTH = 365;

    let depthInput = $state('');
    $effect(() => { depthInput = String(depthDays); });

    const depthInvalid = $derived.by(() => {
        const v = Number(depthInput);
        if (!Number.isFinite(v)) return true;
        if (v < MIN_DEPTH || v > MAX_DEPTH) return true;
        return Math.floor(v) !== v;
    });

    function commitDepth() {
        const v = Number(depthInput);
        if (Number.isFinite(v) && v >= MIN_DEPTH && v <= MAX_DEPTH) {
            depthDays = Math.floor(v);
        }
        depthInput = String(depthDays);
    }

    // ── Derived window math (server burn_in_secs = warmup_bars × macro TF) ──
    const burnInDays = $derived(burnInSecs > 0 ? Math.ceil(burnInSecs / 86400) : 4);
    const scoredDays = $derived(Math.max(0, depthDays - burnInDays));
    const depthTooSmall = $derived(depthDays < burnInDays);

    // ── Per-TF readiness (all four timeframes — not just the entry TF) ──
    interface TfReadiness {
        tf: number;
        requiredSecs: number;
        coveredSecs: number;
        ready: boolean;
        count: number;
    }
    const readiness = $derived.by<TfReadiness[]>(() => {
        const required = depthDays * 86400;
        return ladder.map((tf) => {
            const row = coverageForTf[tf];
            const covered = row ? Math.max(0, (row.latest_secs ?? 0) - (row.earliest_secs ?? 0)) : 0;
            return {
                tf,
                requiredSecs: required,
                coveredSecs: covered,
                ready: row != null && covered >= required,
                count: row?.candle_count ?? 0,
            };
        });
    });
    const allReady = $derived(readiness.every((r) => r.ready));

    const busy = $derived(btRunning || preparing);
    // Run stays enabled when coverage is short — pressing Run triggers the
    // automatic data preparation (the button then reads "Preparing data…").
    const runDisabled = $derived(busy || depthInvalid || depthTooSmall);

    const kpis = $derived.by(() => {
        const s = btResult?.summary;
        if (!s) return [];
        return [
            { label: 'Total Trades', value: String(s.total_trades), sub: 'simulated' },
            { label: 'Win Rate', value: fmtNum(s.win_rate) + '%', sub: `${s.win_count}W / ${s.loss_count}L`, color: s.win_rate >= 50 ? '#22c55e' : '#ef4444' },
            { label: 'Profit Factor', value: fmtNum(s.profit_factor), sub: 'gross win / loss', color: s.profit_factor != null && s.profit_factor >= 1 ? '#22c55e' : '#ef4444' },
            { label: 'Net P&L', value: fmtSigned(s.gross_profit - s.gross_loss), sub: 'on ' + fmtSigned(btCapital) + ' capital', color: s.gross_profit - s.gross_loss >= 0 ? '#22c55e' : '#ef4444' },
            { label: 'Max Drawdown', value: '-' + fmtNum(s.max_drawdown_pct) + '%', sub: 'from peak' },
            { label: 'Expectancy', value: fmtSigned(s.expectancy), sub: 'avg per trade', color: s.expectancy >= 0 ? '#22c55e' : '#ef4444' },
        ];
    });
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle} style="margin-top:0">Run a Backtest — {bound.pair}</h3>
    <p class={styles.infoLine}>
        Bound instance <span class="{styles.badge} {styles.badgeNeutral}">{bound.id}</span> —
        exchange, base currency, TF ladder and config come from this instance (read-only).
        Historical mode fetches the <strong>four timeframe archives</strong> (micro · fast ·
        slow · macro, burn-in included) automatically, then replays the full MME pipeline —
        exactly as the live MME computes. Recorded mode replays recorded MME decisions.
    </p>

    <div class={styles.formRow}>
        <div class={styles.field}>
            <label for="bte-tf" class={styles.fieldLabel}>Timeframe</label>
            <select id="bte-tf" bind:value={btTimeframe} class={styles.fieldInput}>
                {#each ladder as tf (tf)}
                    <option value={tf}>{tf}s</option>
                {/each}
            </select>
        </div>
        <div class={styles.field}>
            <label for="bte-capital" class={styles.fieldLabel}>Capital ($)</label>
            <input id="bte-capital" type="number" bind:value={btCapital} min="100" step="100" class={styles.fieldInput} style="width:110px" />
        </div>
        <div class={styles.field}>
            <label for="bte-mode" class={styles.fieldLabel}>Mode</label>
            <select id="bte-mode" bind:value={btMode} class={styles.fieldInput}>
                <option value="historical">Historical (deep)</option>
                <option value="recorded">Recorded (replay)</option>
            </select>
        </div>
        <div class={styles.field} style="justify-content:flex-end">
            <button class="{styles.btn} {styles.btnPrimary}" onclick={runBacktest} disabled={runDisabled}>
                {preparing ? 'Preparing data…' : btRunning ? 'Running…' : 'Run Backtest'}
            </button>
        </div>
    </div>

    {#if btMode === 'historical'}
        <div class={styles.card} style="margin-top:12px">
            <div style="display:flex; align-items:center; justify-content:space-between; gap:12px; flex-wrap:wrap">
                <div>
                    <h4 class={styles.cardTitle} style="margin:0">How far back can I look</h4>
                    <p class={styles.infoLine} style="margin:4px 0 8px">
                        Backfill pages the exchange backward this many days (1–365) across all four
                        timeframes. The first {burnInDays} day(s) warm the pipeline; the rest is the
                        scored window.
                    </p>
                </div>
                <div style="display:flex; align-items:center; gap:10px">
                    <input
                        type="range"
                        min={MIN_DEPTH}
                        max={MAX_DEPTH}
                        step="1"
                        value={depthDays}
                        oninput={(e) => { depthDays = Number((e.currentTarget as HTMLInputElement).value); depthInput = String(depthDays); }}
                        aria-label="Archive depth days"
                        style="width:220px"
                    />
                    <input
                        type="number"
                        id="bte-depth"
                        min={MIN_DEPTH}
                        max={MAX_DEPTH}
                        class={styles.fieldInput}
                        style="width:86px;{depthInvalid || depthTooSmall ? 'border-color:#ef4444;color:#f87171' : ''}"
                        bind:value={depthInput}
                        onchange={commitDepth}
                        aria-label="Archive depth days (typed)"
                    />
                    <span class={styles.fieldLabel}>days</span>
                    {#if depthInvalid}
                        <span class="{styles.alertBanner} {styles.alertError}" style="margin:0; padding:2px 8px">must be 1–365</span>
                    {:else if depthTooSmall}
                        <span class="{styles.alertBanner} {styles.alertError}" style="margin:0; padding:2px 8px">needs ≥ {burnInDays}d for warmup</span>
                    {/if}
                </div>
            </div>
            <p class={styles.infoLine} style="margin-top:8px">
                Backtest window: <strong>last {scoredDays} day(s) of decisions</strong> ({depthDays}d
                fetched − {burnInDays}d warmup) → {new Date(Date.now() - scoredDays * 864e5).toLocaleDateString()}
                to {new Date().toLocaleDateString()}.
            </p>

            {#if preparing}
                <div class="{styles.alertBanner} {styles.alertWarn}" style="margin-top:10px">
                    Preparing data automatically — fetching the four timeframe archives
                    ({prepareProgress?.pages_fetched ?? 0} pages · {(prepareProgress?.candles_stored ?? 0).toLocaleString()} candles stored).
                </div>
            {/if}

            <h4 class={styles.cardTitle} style="margin-top:12px">Four-Timeframe Readiness</h4>
            <div style="display:flex; gap:10px; flex-wrap:wrap">
                {#each readiness as r (r.tf)}
                    <span
                        class="{styles.badge} {r.ready ? styles.badgeLong : styles.badgeError}"
                        title="{r.coveredSecs >= 86400 ? (r.coveredSecs / 86400).toFixed(1) + 'd' : Math.floor(r.coveredSecs / 3600) + 'h'} archived · {r.count.toLocaleString()} candles · need {r.requiredSecs >= 86400 ? (r.requiredSecs / 86400).toFixed(1) + 'd' : Math.floor(r.requiredSecs / 3600) + 'h'}"
                    >
                        {r.tf === 60 ? 'MICRO' : r.tf === 180 ? 'FAST' : r.tf === 300 ? 'SLOW' : 'MACRO'} · {r.tf}s · {r.ready ? 'READY' : 'FETCHING'}
                    </span>
                {/each}
            </div>
            <p class={styles.infoLine} style="margin-top:8px">
                {#if allReady}
                    All four timeframe archives cover the requested depth — Run will replay the full
                    MME pipeline immediately.
                {:else}
                    Missing coverage is fetched automatically when you press Run Backtest
                    (progress above); re-runs skip already-covered spans.
                {/if}
            </p>
        </div>
    {/if}

    {#if btError}
        <div class="{styles.alertBanner} {styles.alertError}" style="margin-top:12px">{btError}</div>
    {/if}

    {#if btResult}
        <h3 class={styles.cardTitle} style="margin-top:16px">Latest Run — #{btResult.backtest_id} · {btResult.mode ?? btMode}</h3>
        <KpiStrip items={kpis} />
        <p class={styles.infoLine}>
            Full analysis on the <strong>Study Report</strong> tab; per-engine breakdowns on the DIE / MME / TAE / PME / PAE tabs.
        </p>
    {:else}
        <div class={styles.empty} style="margin-top:12px">
            Choose a timeframe, capital and depth, then run the backtest. Historical mode fetches the
            four timeframe archives automatically (burn-in included) and replays the full MME pipeline;
            recorded mode replays recorded MME decisions.
        </div>
    {/if}
</div>
