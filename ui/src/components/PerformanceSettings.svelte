<script lang="ts">
    // PerformanceSettings — the PAE Settings tab (always present).
    // Editable, config-driven: the significance treatment (α, Monte Carlo
    // runs, min trades) and the default backtest capital. Saving through
    // the validated `POST /api/config`; one header save button (shared
    // state machine). Changing α changes every verdict — warned inline.
    import { onMount } from 'svelte';
    import SettingsSaveButton, { type SettingsSaveState } from './SettingsSaveButton.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import ConfigSourceChip from './ConfigSourceChip.svelte';
    import ModeChip from './ModeChip.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import type { ExecutionMode } from '../lib/modePresentation';
    import styles from '../styles/engine-dashboard.module.css';

    let { mode }: { mode?: ExecutionMode } = $props();

    interface AnalyticsCfg {
        alpha?: number;
        monte_carlo_runs?: number;
        min_trades_for_verdict?: number;
    }
    interface InstanceEntry { id?: string; symbol?: string; portfolio_capital_usd?: number }

    let cfg: { analytics?: AnalyticsCfg; instances?: InstanceEntry[]; portfolio_capital_usd?: number } | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);
    let saveError: string | null = $state(null);
    let saveState = $state<SettingsSaveState>('idle');

    let analytics = $state<AnalyticsCfg>({ alpha: 0.05, monte_carlo_runs: 10000, min_trades_for_verdict: 30 });

    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            cfg = data;
            if (data.analytics) {
                analytics = {
                    alpha: data.analytics.alpha ?? 0.05,
                    monte_carlo_runs: data.analytics.monte_carlo_runs ?? 10000,
                    min_trades_for_verdict: data.analytics.min_trades_for_verdict ?? 30,
                };
            }
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    onMount(fetchConfig);

    const dirty = $derived.by(() => {
        const c = cfg;
        if (!c) return false;
        return (
            JSON.stringify(analytics) !== JSON.stringify({
                alpha: c.analytics?.alpha ?? 0.05,
                monte_carlo_runs: c.analytics?.monte_carlo_runs ?? 10000,
                min_trades_for_verdict: c.analytics?.min_trades_for_verdict ?? 30,
            })
        );
    });

    $effect(() => {
        if (dirty && saveState !== 'saving' && saveState !== 'error') saveState = 'dirty';
    });

    async function save() {
        if (saveState !== 'dirty' && saveState !== 'error') return;
        saveError = null;
        saveState = 'saving';
        try {
            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    analytics: {
                        alpha: Number(analytics.alpha),
                        monte_carlo_runs: Number(analytics.monte_carlo_runs),
                        min_trades_for_verdict: Number(analytics.min_trades_for_verdict),
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
        return buildEngineExport('performance', 'settings', mode ?? null, {
            analytics,
        });
    }
</script>

<div style="display:flex; flex-direction:column; height:100%; background:#000">
    <header class={styles.unifiedHeader}>
        <div class={styles.headerTop}>
            <div class={styles.titleGroup}>
                <h2 class={styles.title}>Performance Analytics Settings</h2>
            </div>
            <div class={styles.headerRight}>
                <span class={styles.tabLabel}>Settings</span>
                {#if mode}
                    <ModeChip {mode} />
                {/if}
                <SettingsSaveButton state={saveState} onsave={save} />
                <ExportDataButton onExport={buildExport} title="Copy the Performance Analytics configuration as JSON" />
            </div>
        </div>
    </header>

    <div class={styles.content}>
        {#if saveError}
            <div class="{styles.alertBanner} {styles.alertError}">{saveError}</div>
        {/if}
        {#if loading}
            <div class={styles.empty}>Loading…</div>
        {:else if error}
            <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
        {:else if cfg}
            <div class="{styles.alertBanner} {styles.alertWarn}">
                Changing the significance treatment changes every verdict. Re-run backtests after editing.
            </div>

            <div class={styles.card}>
                <div class={styles.cardHead}>
                    <h3 class={styles.cardTitle}>Significance Treatment</h3>
                    <ConfigSourceChip source="[workspace.analytics]" apply="LIVE" />
                </div>
                <p class={styles.infoLine}>The exact parameters the engine runs with for every verdict (t-test + Monte Carlo sign randomization).</p>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pae-alpha">Alpha (α)</label>
                        <input class={styles.fieldInput} id="pae-alpha" type="number" min="0.001" max="0.5" step="0.001" bind:value={analytics.alpha} />
                        <span class={styles.muted} style="font-size:10px">significance level — both p-values must fall below it</span>
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pae-mc">Monte Carlo runs</label>
                        <input class={styles.fieldInput} id="pae-mc" type="number" min="100" max="1000000" step="1000" bind:value={analytics.monte_carlo_runs} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pae-min">Min trades for verdict</label>
                        <input class={styles.fieldInput} id="pae-min" type="number" min="1" max="10000" step="1" bind:value={analytics.min_trades_for_verdict} />
                        <span class={styles.muted} style="font-size:10px">below this: InsufficientData</span>
                    </div>
                </div>
            </div>
        {:else}
            <div class={styles.empty}>No configuration available.</div>
        {/if}
    </div>
</div>
