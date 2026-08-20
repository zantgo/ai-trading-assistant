<script lang="ts">
    // PortfolioSettings — the PME Settings tab (always present).
    // Read-only, config-driven values from /api/config — the safety ladder
    // thresholds previously fell back to hardcoded defaults because
    // ConfigResponse did not carry `safety`; v7.3 fixes that.
    import { onMount } from 'svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    interface Cfg {
        safety?: {
            consecutive_loss_caution?: number;
            consecutive_loss_dropout?: number;
            dropout_duration_hours?: number;
            drawdown_limit_pct?: number;
            max_daily_drawdown_pct?: number;
            systemic_risk_threshold?: number;
        };
        risk_limits?: {
            max_single_pair_exposure_pct?: number;
            max_portfolio_exposure_pct?: number;
            max_correlation?: number;
        };
        fees?: { maker_fee_pct?: number; taker_fee_pct?: number; funding_rate_8h?: number };
        leverage?: { cross_leverage?: number };
    }

    let cfg: Cfg | null = $state(null);
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

    function buildExport(): string {
        return buildEngineExport('portfolio', 'settings', null, {
            loading,
            error,
            safety: cfg?.safety ?? null,
            risk_limits: cfg?.risk_limits ?? null,
            fees: cfg?.fees ?? null,
            leverage: cfg?.leverage ?? null,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy the Portfolio Management configuration as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if cfg}
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Safety Ladder</h3>
            <p class={styles.infoLine}>Config from <code>config.toml → [workspace.safety]</code> — the protection ladder thresholds the PME Safety tab renders.</p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Consecutive losses → CAUTIOUS</td><td>{cfg.safety?.consecutive_loss_caution ?? '—'}</td></tr>
                    <tr><td>Consecutive losses → SUSPENDED</td><td>{cfg.safety?.consecutive_loss_dropout ?? '—'}</td></tr>
                    <tr><td>Dropout cooldown</td><td>{cfg.safety?.dropout_duration_hours ?? '—'}h</td></tr>
                    <tr><td>Drawdown limit</td><td>{fmtPct(cfg.safety?.drawdown_limit_pct)}</td></tr>
                    <tr><td>Max daily drawdown</td><td>{fmtPct(cfg.safety?.max_daily_drawdown_pct)}</td></tr>
                    <tr><td>Systemic risk threshold</td><td>{cfg.safety?.systemic_risk_threshold ?? '—'}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Risk Limits</h3>
            <p class={styles.infoLine}>Concentration / exposure / correlation caps from <code>[workspace.risk_limits]</code> — the same numbers the Exposure tab renders and the backend enforces.</p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Max single-pair exposure</td><td>{fmtPct(cfg.risk_limits?.max_single_pair_exposure_pct)}</td></tr>
                    <tr><td>Max portfolio exposure</td><td>{fmtPct(cfg.risk_limits?.max_portfolio_exposure_pct)}</td></tr>
                    <tr><td>Max correlation</td><td>{cfg.risk_limits?.max_correlation ?? '—'}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Fees &amp; Leverage</h3>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Maker fee</td><td>{fmtPct(cfg.fees?.maker_fee_pct)}</td></tr>
                    <tr><td>Taker fee</td><td>{fmtPct(cfg.fees?.taker_fee_pct)}</td></tr>
                    <tr><td>Funding rate (8h)</td><td>{fmtPct(cfg.fees?.funding_rate_8h)}</td></tr>
                    <tr><td>Cross leverage</td><td>{cfg.leverage?.cross_leverage ?? '—'}×</td></tr>
                </tbody>
            </table>
        </div>
    {:else}
        <div class={styles.empty}>No configuration available.</div>
    {/if}
</div>
