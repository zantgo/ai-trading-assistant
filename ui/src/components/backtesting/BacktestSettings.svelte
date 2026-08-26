<script lang="ts">
    // BacktestSettings — the BTE Settings tab (always present, even with
    // no instance). Edits [workspace.backtest] through the validated
    // POST /api/config; one header save button with the shared state
    // machine. The archive depth carries the 1..=365 contract with the
    // same slider + typed-input validation as the run form.
    import { onMount } from 'svelte';
    import SettingsSaveButton, { type SettingsSaveState } from '../SettingsSaveButton.svelte';
    import ExportDataButton from '../ExportDataButton.svelte';
    import ConfigSourceChip from '../ConfigSourceChip.svelte';
    import { buildEngineExport } from '../../lib/engineExport';
    import styles from '../../styles/engine-dashboard.module.css';

    interface BacktestCfg {
        archive_depth_days?: number;
        warmup_bars?: number;
        max_equity_points?: number;
        max_snapshots?: number;
        store_input_bars?: boolean;
        hyperliquid?: { page_cap?: number; rate_limit_delay_ms?: number; max_pages_per_run?: number };
        bitget?: { page_cap?: number; rate_limit_delay_ms?: number; max_pages_per_run?: number };
    }

    let cfg: BacktestCfg | null = $state(null);
    let loading = $state(true);
    let saveError: string | null = $state(null);
    let saveState = $state<SettingsSaveState>('idle');

    let depth = $state(180);
    let depthInput = $state('180');
    let warmupBars = $state(300);
    let storeInputBars = $state(true);

    const MIN_DEPTH = 1;
    const MAX_DEPTH = 365;
    const depthInvalid = $derived.by(() => {
        const v = Number(depthInput);
        return !Number.isFinite(v) || v < MIN_DEPTH || v > MAX_DEPTH || Math.floor(v) !== v;
    });

    function commitDepth() {
        const v = Number(depthInput);
        if (Number.isFinite(v) && v >= MIN_DEPTH && v <= MAX_DEPTH) depth = Math.floor(v);
        depthInput = String(depth);
    }

    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            const b = data.backtest ?? {};
            cfg = b;
            depth = b.archive_depth_days ?? 180;
            depthInput = String(depth);
            warmupBars = b.warmup_bars ?? 300;
            storeInputBars = b.store_input_bars ?? true;
        } catch (_) {} finally {
            loading = false;
        }
    }

    onMount(fetchConfig);

    const dirty = $derived.by(() => {
        if (!cfg) return false;
        return depth !== (cfg.archive_depth_days ?? 180)
            || warmupBars !== (cfg.warmup_bars ?? 300)
            || storeInputBars !== (cfg.store_input_bars ?? true);
    });

    $effect(() => {
        if (dirty && saveState !== 'saving' && saveState !== 'error') saveState = 'dirty';
    });

    async function save() {
        if (saveState !== 'dirty' && saveState !== 'error') return;
        if (depthInvalid || warmupBars < 30 || warmupBars > 10_000) {
            saveError = 'Invalid values: archive depth must be 1–365, warmup bars 30–10,000.';
            saveState = 'error';
            return;
        }
        saveError = null;
        saveState = 'saving';
        try {
            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    backtest: {
                        ...cfg,
                        archive_depth_days: depth,
                        warmup_bars: warmupBars,
                        store_input_bars: storeInputBars,
                    },
                }),
            });
            if (res.ok) {
                await fetchConfig();
                saveState = 'saved';
                setTimeout(() => { saveState = 'idle'; }, 2000);
            } else {
                saveError = (await res.text()) || 'Save failed';
                saveState = 'error';
            }
        } catch (e) {
            saveError = e instanceof Error ? e.message : 'Save failed';
            saveState = 'error';
        }
    }

    function buildExport(): string {
        return buildEngineExport('backtesting', 'settings', null, {
            warmup_bars: warmupBars,
            store_input_bars: storeInputBars,
        });
    }
</script>

<div style="display:flex; flex-direction:column; height:100%; background:#000">
    <header class={styles.unifiedHeader}>
        <div class={styles.headerTop}>
            <div class={styles.titleGroup}>
                <h2 class={styles.title}>Backtesting Settings</h2>
            </div>
            <div class={styles.headerRight}>
                <span class={styles.tabLabel}>Settings</span>
                <SettingsSaveButton state={saveState} onsave={save} />
                <ExportDataButton onExport={buildExport} title="Copy the Backtesting configuration as JSON" />
            </div>
        </div>
    </header>

    <div class={styles.content}>
        {#if saveError}
            <div class="{styles.alertBanner} {styles.alertError}">{saveError}</div>
        {/if}
        {#if loading}
            <div class={styles.empty}>Loading backtesting configuration…</div>
        {:else}
            <div class={styles.card}>
                <h3 class={styles.cardTitle} style="margin:0">
                    Backtest Execution
                    <ConfigSourceChip source="config.toml → [workspace.backtest]" apply="LIVE" />
                </h3>
                <p class={styles.infoLine}>
                    Archive depth (1–365 days) is chosen per-run in the Backtest Launcher wizard
                    (Historical Data step) — no global depth setting here. Warmup bars and
                    input-bar persistence below still apply to every run.
                </p>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label for="bts-warmup" class={styles.fieldLabel}>Warmup Bars</label>
                        <input id="bts-warmup" type="number" bind:value={warmupBars} min="30" max="10000" class={styles.fieldInput} />
                    </div>
                    <div class={styles.field}>
                        <label for="bts-store" class={styles.fieldLabel}>Store Input Bars</label>
                        <select id="bts-store" bind:value={storeInputBars} class={styles.fieldInput}>
                            <option value={true}>Yes (reproducible)</option>
                            <option value={false}>No (lighter DB)</option>
                        </select>
                    </div>
                </div>
            </div>

            <div class={styles.card}>
                <h3 class={styles.cardTitle} style="margin:0">
                    Exchange Paging Limits
                    <ConfigSourceChip source="config.toml → [workspace.backtest.<exchange>]" />
                </h3>
                <p class={styles.infoLine}>
                    Read-only summary — Hyperliquid pages conservatively at
                    {cfg?.hyperliquid?.page_cap ?? 1000} candles/request (window-bounded, no limit
                    parameter); Bitget accepts limit 1–1000 and pages at
                    {cfg?.bitget?.page_cap ?? 200}/request. These caps determine the backfill
                    page budget and the theoretical max lookback.
                </p>
            </div>
        {/if}
    </div>
</div>
