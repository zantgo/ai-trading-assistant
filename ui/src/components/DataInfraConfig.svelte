<script lang="ts">
    // DataInfraConfig — DIE Settings tab. Renders the REAL platform
    // configuration served by `GET /api/system/platform-config` (+ live
    // clock runtime from `/api/system/clock` and workspace liquidity from
    // `/api/config`). Every row is a live value from config.toml — no
    // hardcoded strings, no fabricated settings.
    import type { ClockStatusResponse } from '../types';
    import { COLORS } from '../lib/statusColors';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    interface PlatformConfigPayload {
        hyperliquid?: { ws_url?: string };
        bitget?: { ws_url?: string };
        clock_monitor?: {
            enabled?: boolean;
            ntp_servers?: string[];
            poll_interval_secs?: number;
            threshold_micros?: number;
            query_timeout_secs?: number;
            jitter_window_size?: number;
            breach_action?: string;
            warn_on_breach?: boolean;
        };
        quality?: {
            median_window_size?: number;
            outlier_tolerance?: number;
            bypass_on_zero_median?: boolean;
            staleness_threshold_secs?: number;
        };
        reconnect?: {
            initial_backoff_ms?: number;
            max_backoff_ms?: number;
            jitter_pct?: number;
            connect_grace_ms?: number;
            disconnect_grace_ms?: number;
        };
        candle_buffer?: {
            size?: number;
            stale_threshold_secs?: number;
            fetch_timeout_ms?: number;
            sub_minute_skip_historical?: boolean;
        };
    }

    interface WorkspaceConfigPayload {
        liquidity?: {
            event_retention_days?: number;
            bucket_retention_days?: number;
        };
    }

    let platform: PlatformConfigPayload | null = $state(null);
    let workspace: WorkspaceConfigPayload | null = $state(null);
    let clockReport: ClockStatusResponse | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    let pollInterval: ReturnType<typeof setInterval> | null = null;

    function restUrl(wsUrl: string | undefined): string {
        if (!wsUrl) return '—';
        return wsUrl
            .replace('wss://', 'https://')
            .replace('ws://', 'http://')
            .replace('/ws', '/info');
    }

    function fmtMs(ms: number | undefined): string {
        if (ms == null) return '—';
        return ms >= 1000 ? `${(ms / 1000).toFixed(0)}s` : `${ms}ms`;
    }

    async function fetchAll() {
        try {
            const [platformRes, clockRes, configRes] = await Promise.all([
                fetch('/api/system/platform-config'),
                fetch('/api/system/clock'),
                fetch('/api/config'),
            ]);
            if (!platformRes.ok) throw new Error(`HTTP ${platformRes.status}`);
            platform = (await platformRes.json()) as PlatformConfigPayload;
            if (clockRes.ok) clockReport = (await clockRes.json()) as ClockStatusResponse;
            if (configRes.ok) workspace = (await configRes.json()) as WorkspaceConfigPayload;
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    $effect(() => {
        loading = true;
        fetchAll();
        if (pollInterval) clearInterval(pollInterval);
        pollInterval = setInterval(fetchAll, 30_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });

    function buildExport(): string {
        return buildEngineExport('data_infra', 'settings', null, {
            loading,
            error,
            platform: {
                hyperliquid: platform?.hyperliquid,
                bitget: platform?.bitget,
                clock_monitor: platform?.clock_monitor,
                quality: platform?.quality,
                reconnect: platform?.reconnect,
                candle_buffer: platform?.candle_buffer,
            },
            workspace: workspace,
            clock_runtime: clockReport ? {
                within_threshold: clockReport.within_threshold,
                drift_us: clockReport.drift_us,
                jitter_rms_us: clockReport.jitter_rms_us,
                last_poll_ms: clockReport.last_poll_ms,
                breach_count: clockReport.breach_count,
                sample_count: clockReport.sample_count,
            } : null,
        });
    }
</script>

<div class={styles.content} style="padding:0; overflow:visible">
    <div style="display:flex; align-items:center; justify-content:flex-end; margin-bottom:12px">
        <ExportDataButton onExport={buildExport} title="Copy all Settings (platform config) as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading configuration…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if platform}
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Exchange Endpoints</h3>
            <p class={styles.infoLine}>
                WebSocket feeds the daemon connects to (L1 raw ingestion), plus the derived REST
                endpoints used for bootstrap warm-up and gap-filling.
            </p>
            <table class={styles.table}>
                <thead><tr><th>Exchange</th><th>WebSocket URL</th><th>REST URL (derived)</th></tr></thead>
                <tbody>
                    <tr>
                        <td class={styles.tdMono}>Hyperliquid</td>
                        <td class={styles.tdMono}>{platform.hyperliquid?.ws_url ?? '—'}</td>
                        <td class={styles.tdMono}>{restUrl(platform.hyperliquid?.ws_url)}</td>
                    </tr>
                    <tr>
                        <td class={styles.tdMono}>Bitget</td>
                        <td class={styles.tdMono}>{platform.bitget?.ws_url ?? '—'}</td>
                        <td class={styles.tdMono}>{restUrl(platform.bitget?.ws_url)}</td>
                    </tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>NTP Clock Monitor</h3>
            <p class={styles.infoLine}>
                Config values from <code>config.toml</code>; runtime drift/samples from the live clock monitor.
            </p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Enabled</td><td>{platform.clock_monitor?.enabled ? 'Yes' : 'No'}</td></tr>
                    <tr><td>Poll interval</td><td>{platform.clock_monitor?.poll_interval_secs ?? '—'}s</td></tr>
                    <tr><td>Threshold</td><td>{clockReport?.threshold_micros ?? platform.clock_monitor?.threshold_micros ?? '—'}µs</td></tr>
                    <tr><td>Query timeout</td><td>{platform.clock_monitor?.query_timeout_secs ?? '—'}s</td></tr>
                    <tr><td>Jitter window</td><td>{platform.clock_monitor?.jitter_window_size ?? '—'} samples</td></tr>
                    <tr>
                        <td>Breach action</td>
                        <td style="color: {(platform.clock_monitor?.breach_action ?? '') === 'panic' ? COLORS.poor : COLORS.good}">
                            {(platform.clock_monitor?.breach_action ?? '—').toUpperCase()}
                        </td>
                    </tr>
                    <tr><td>Warn on breach</td><td>{platform.clock_monitor?.warn_on_breach ? 'Yes' : 'No'}</td></tr>
                    <tr><td>NTP servers</td><td>{(platform.clock_monitor?.ntp_servers ?? []).join(', ') || '—'}</td></tr>
                    <tr><td>Live drift</td><td>{clockReport?.drift_us != null ? `${clockReport.drift_us}µs` : '—'}</td></tr>
                    <tr><td>Breach count (live)</td><td style="color: {(clockReport?.breach_count ?? 0) > 0 ? COLORS.poor : COLORS.good}">{clockReport?.breach_count ?? '—'}</td></tr>
                    <tr><td>Sample count (live)</td><td>{clockReport?.sample_count ?? '—'}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Connection Resilience</h3>
            <p class={styles.infoLine}>
                Exponential backoff + grace windows for WebSocket reconnects (L1).
            </p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Initial backoff</td><td>{fmtMs(platform.reconnect?.initial_backoff_ms)}</td></tr>
                    <tr><td>Max backoff</td><td>{fmtMs(platform.reconnect?.max_backoff_ms)}</td></tr>
                    <tr><td>Jitter</td><td>±{((platform.reconnect?.jitter_pct ?? 0.2) * 100).toFixed(0)}%</td></tr>
                    <tr><td>Connect grace</td><td>{fmtMs(platform.reconnect?.connect_grace_ms)}</td></tr>
                    <tr><td>Disconnect grace</td><td>{fmtMs(platform.reconnect?.disconnect_grace_ms)}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Candle Buffer</h3>
            <p class={styles.infoLine}>
                Rolling buffer depth, staleness policy and historical warm-up behaviour (CB-01…CB-12).
            </p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Buffer size</td><td>{platform.candle_buffer?.size ?? '—'} candles</td></tr>
                    <tr><td>Stale threshold</td><td>{platform.candle_buffer?.stale_threshold_secs ?? '—'}s</td></tr>
                    <tr><td>REST fetch timeout</td><td>{fmtMs(platform.candle_buffer?.fetch_timeout_ms)}</td></tr>
                    <tr><td>Sub-minute skip historical</td><td>{platform.candle_buffer?.sub_minute_skip_historical ? 'Yes' : 'No'}</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Quality — Median Filter</h3>
            <p class={styles.infoLine}>
                L3 tick-level sanitization: median warm-up, outlier rejection, staleness.
            </p>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Median window</td><td>{platform.quality?.median_window_size ?? '—'} ticks</td></tr>
                    <tr><td>Outlier tolerance</td><td>{((platform.quality?.outlier_tolerance ?? 0.05) * 100).toFixed(1)}%</td></tr>
                    <tr><td>Bypass on zero median</td><td>{platform.quality?.bypass_on_zero_median ? 'Yes' : 'No'}</td></tr>
                    <tr><td>Staleness threshold</td><td>{platform.quality?.staleness_threshold_secs ?? '—'}s</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Quality Windows</h3>
            <p class={styles.infoLine}>
                Rolling windows for the composite connection-quality score. Fixed by the
                runtime (not configurable).
            </p>
            <table class={styles.table}>
                <thead><tr><th>Window</th><th>Duration</th><th>Score formula</th></tr></thead>
                <tbody>
                    <tr><td>1 Hour</td><td>3,600 s</td><td rowspan="3" style="color:rgba(255,255,255,0.55)">50·uptime + 30·disconnect + 20·reconnect − 5·data-loss − 5·reconstructed</td></tr>
                    <tr><td>6 Hour</td><td>21,600 s</td></tr>
                    <tr><td>24 Hour</td><td>86,400 s</td></tr>
                </tbody>
            </table>
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Persistence</h3>
            <table class={styles.table}>
                <thead><tr><th>Setting</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>Quality sample interval</td><td>60 s (runtime constant)</td></tr>
                    <tr><td>Liquidation event retention</td><td>{workspace?.liquidity?.event_retention_days ?? '—'} days</td></tr>
                    <tr><td>Liquidation bucket retention</td><td>{workspace?.liquidity?.bucket_retention_days ?? '—'} days</td></tr>
                </tbody>
            </table>
        </div>
    {:else}
        <div class={styles.empty}>No platform configuration available.</div>
    {/if}
</div>
