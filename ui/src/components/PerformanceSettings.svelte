<script lang="ts">
    // PerformanceSettings — the PAE Settings tab (always present).
    // Read-only, config-driven values from /api/config — the significance
    // treatment the Methodology tab explains, plus fee defaults.
    import { onMount } from 'svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    interface Cfg {
        analytics?: {
            alpha?: number;
            monte_carlo_runs?: number;
            min_trades_for_verdict?: number;
        };
        fees?: { maker_fee_pct?: number; taker_fee_pct?: number; funding_rate_8h?: number };
        instances?: { initial_capital_usd?: number }[];
    }

    let cfg = $state<Cfg | null>(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            cfg = (await res.json()) as Cfg;
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    onMount(fetchConfig);

    function fmtPct(v: number | undefined): string {
        return v != null ? `${v.toFixed(2)}%` : '—';
    }

    const defaultCapital = $derived(cfg?.instances?.[0]?.initial_capital_usd ?? 1000);

    function buildExport(): string {
        return buildEngineExport('performance', 'settings', null, {
            loading,
            error,
            analytics: cfg?.analytics ?? null,
            fees: cfg?.fees ?? null,
            default_initial_capital_usd: defaultCapital,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy the Performance Analytics configuration as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if cfg}
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Significance Treatment</h3>
            <p class={styles.infoLine}>Config from <code>config.toml → [workspace.analytics]</code> — the exact parameters the engine runs with for every verdict (t-test + Monte Carlo).</p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Alpha (α)</td><td>{cfg.analytics?.alpha ?? 0.05}</td></tr>
                    <tr><td>Monte Carlo runs</td><td>{(cfg.analytics?.monte_carlo_runs ?? 10000).toLocaleString()}</td></tr>
                    <tr><td>Min trades for verdict</td><td>{cfg.analytics?.min_trades_for_verdict ?? 30}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Fees &amp; Capital Defaults</h3>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Maker fee</td><td>{fmtPct(cfg.fees?.maker_fee_pct)}</td></tr>
                    <tr><td>Taker fee</td><td>{fmtPct(cfg.fees?.taker_fee_pct)}</td></tr>
                    <tr><td>Funding rate (8h)</td><td>{fmtPct(cfg.fees?.funding_rate_8h)}</td></tr>
                    <tr><td>Default backtest capital</td><td>${defaultCapital.toLocaleString()}</td></tr>
                </tbody>
            </table>
        </div>
    {:else}
        <div class={styles.empty}>No configuration available.</div>
    {/if}
</div>
