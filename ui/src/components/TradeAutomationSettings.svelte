<script lang="ts">
    // TradeAutomationSettings — the TAE Settings tab (always present).
    // Read-only, config-driven values from /api/config — no hardcoded
    // numbers, no fabricated settings.
    import { onMount } from 'svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    interface Cfg {
        minimal_tae?: {
            enabled?: boolean;
            risk_per_trade_pct?: number;
            min_net_rr?: number;
            max_position_size_usd?: number | null;
            max_open_positions?: number;
            entry_mode?: string;
            invalidate_on?: string;
        };
        fees?: { maker_fee_pct?: number; taker_fee_pct?: number; funding_rate_8h?: number };
        leverage?: { cross_leverage?: number };
        execution?: { slippage_ceiling_pct?: number };
        scoring?: {
            base_allocation_pct?: number;
            micro_allocation_pct?: number;
            max_allocation_pct?: number;
            base_score_threshold?: number;
            micro_score_threshold?: number;
        };
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
        return buildEngineExport('trade_automation', 'settings', null, {
            loading,
            error,
            minimal_tae: cfg?.minimal_tae ?? null,
            fees: cfg?.fees ?? null,
            leverage: cfg?.leverage ?? null,
            execution: cfg?.execution ?? null,
            scoring: cfg?.scoring ?? null,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy the Trade Automation configuration as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if cfg}
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Setup Executor</h3>
            <p class={styles.infoLine}>Config from <code>config.toml → [workspace.minimal_tae]</code> — how the executor accepts, sizes and manages setups.</p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Enabled</td><td>{cfg.minimal_tae?.enabled ? 'Yes' : 'No'}</td></tr>
                    <tr><td>Risk per trade</td><td>{fmtPct(cfg.minimal_tae?.risk_per_trade_pct)}</td></tr>
                    <tr><td>Min net R:R</td><td>{cfg.minimal_tae?.min_net_rr ?? '—'}</td></tr>
                    <tr><td>Max position size</td><td>{cfg.minimal_tae?.max_position_size_usd != null ? `$${cfg.minimal_tae.max_position_size_usd.toLocaleString()}` : 'No cap'}</td></tr>
                    <tr><td>Max open positions</td><td>{cfg.minimal_tae?.max_open_positions ?? '—'}</td></tr>
                    <tr><td>Entry mode</td><td class={styles.tdMono}>{cfg.minimal_tae?.entry_mode ?? '—'}</td></tr>
                    <tr><td>Invalidate on</td><td class={styles.tdMono}>{cfg.minimal_tae?.invalidate_on ?? '—'}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Fees &amp; Economics</h3>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Maker fee</td><td>{fmtPct(cfg.fees?.maker_fee_pct)}</td></tr>
                    <tr><td>Taker fee</td><td>{fmtPct(cfg.fees?.taker_fee_pct)}</td></tr>
                    <tr><td>Funding rate (8h)</td><td>{fmtPct(cfg.fees?.funding_rate_8h)}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Leverage &amp; Execution</h3>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Cross leverage</td><td>{cfg.leverage?.cross_leverage ?? '—'}×</td></tr>
                    <tr><td>Slippage ceiling</td><td>{fmtPct(cfg.execution?.slippage_ceiling_pct)}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Allocation Scoring</h3>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Base allocation</td><td>{fmtPct(cfg.scoring?.base_allocation_pct)}</td></tr>
                    <tr><td>Micro allocation</td><td>{fmtPct(cfg.scoring?.micro_allocation_pct)}</td></tr>
                    <tr><td>Max allocation</td><td>{fmtPct(cfg.scoring?.max_allocation_pct)}</td></tr>
                    <tr><td>Base score threshold</td><td>{cfg.scoring?.base_score_threshold ?? '—'}</td></tr>
                    <tr><td>Micro score threshold</td><td>{cfg.scoring?.micro_score_threshold ?? '—'}</td></tr>
                </tbody>
            </table>
        </div>
    {:else}
        <div class={styles.empty}>No configuration available.</div>
    {/if}
</div>
