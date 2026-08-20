<script lang="ts">
    import type { ExchangeStatusReport, ExchangeConnectionState } from '../types';
    import { useAppStore } from '../state.svelte';
    import { formatRelativeTime } from '../lib/relTime';
    import KpiStrip from './KpiStrip.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    const app = useAppStore();

    let report: ExchangeStatusReport | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    const currentExchange = $derived(app.session?.sessionExchange ?? 'Hyperliquid');

    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchStatus() {
        try {
            const res = await fetch('/api/exchange-status');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            report = await res.json();
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    $effect(() => {
        fetchStatus();
        if (pollInterval) clearInterval(pollInterval);
        pollInterval = setInterval(fetchStatus, 30_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });

    const filteredExchanges = $derived.by(() => {
        if (!report) return [];
        const list = report.exchanges || [];
        return list.filter((e) => e.name.toLowerCase() === currentExchange.toLowerCase());
    });

    function stateBadge(state: ExchangeConnectionState): string {
        switch (state) {
            case 'Connected': return styles.badgeLong;
            case 'Connecting': return styles.badgeNeutral;
            case 'Reconnecting': return styles.badgeNeutral;
            case 'Disabled': return styles.badgeEmpty;
            default: return styles.badgeError;
        }
    }

    function stateLabel(state: ExchangeConnectionState): string {
        switch (state) {
            case 'Connected': return '● CONNECTED';
            case 'Connecting': return '◌ CONNECTING';
            case 'Reconnecting': return '↻ RECONNECTING';
            case 'Disabled': return '✕ DISABLED';
            default: return '● DISCONNECTED';
        }
    }

    function stateHint(state: ExchangeConnectionState): string {
        switch (state) {
            case 'Connected': return 'WebSocket stream active';
            case 'Connecting': return 'Establishing WebSocket handshake…';
            case 'Reconnecting': return 'Exponential backoff retry in progress';
            case 'Disabled': return 'Permanently disabled after repeated failures';
            default: return 'Not connected';
        }
    }

    function restUrl(wsUrl: string): string {
        if (!wsUrl) return '—';
        return wsUrl
            .replace('wss://', 'https://')
            .replace('ws://', 'http://')
            .replace('/ws', '/info');
    }

    function heartbeatLabel(lastHeartbeatMs: number): string {
        if (lastHeartbeatMs <= 0) return 'No heartbeat yet';
        const rel = formatRelativeTime(lastHeartbeatMs);
        return rel.label;
    }

    function heartbeatColor(lastHeartbeatMs: number): string {
        if (lastHeartbeatMs <= 0) return 'rgba(255,255,255,0.4)';
        const secs = (Date.now() - lastHeartbeatMs) / 1000;
        if (secs < 30) return '#22c55e';
        if (secs < 60) return '#f59e0b';
        return '#ef4444';
    }

    function buildExport(): string {
        return buildEngineExport('data_infra', 'exchange_status', null, {
            loading,
            error,
            scope: currentExchange,
            exchanges: filteredExchanges.map((e) => ({
                name: e.name,
                state: e.state,
                state_label: stateLabel(e.state),
                active_pairs: e.active_pairs,
                total_reconnects: e.total_reconnects,
                last_heartbeat_ms: e.last_heartbeat_ms,
                ws_url: e.ws_url,
                rest_url: restUrl(e.ws_url),
            })),
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy all Exchange Status data as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if filteredExchanges.length > 0}
        <KpiStrip items={filteredExchanges.map((e) => ({
            label: e.name,
            value: e.state === 'Connected' ? String(e.active_pairs) : stateLabel(e.state),
            sub: e.state === 'Connected' ? 'active pairs' : stateHint(e.state),
            color: e.state === 'Connected' ? '#22c55e' : e.state === 'Reconnecting' ? '#f59e0b' : undefined,
        }))} />
        <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(320px, 1fr)); gap:12px">
            {#each filteredExchanges as exchange (exchange.name)}
                <div class={styles.card}>
                    <div style="display:flex; align-items:center; gap:8px; flex-wrap:wrap; margin-bottom:10px">
                        <span class={styles.metaChip}><span class={styles.metaChipValue}>{exchange.name}</span></span>
                        <span class="{styles.badge} {stateBadge(exchange.state)}">{stateLabel(exchange.state)}</span>
                    </div>
                    <p class={styles.infoLine}>{stateHint(exchange.state)}</p>
                    <div class={styles.monoList}>
                        <span>WS: {exchange.ws_url}</span>
                        <span>REST: {restUrl(exchange.ws_url)}</span>
                    </div>
                    <table class={styles.table} style="margin-top:10px">
                        <tbody>
                            <tr><td>Pairs</td><td class={styles.tdRight}>{exchange.active_pairs}</td></tr>
                            <tr><td>Reconnects</td><td class={styles.tdRight}>{exchange.total_reconnects}</td></tr>
                            <tr><td>Last Heartbeat</td><td class={styles.tdRight} style="color:{heartbeatColor(exchange.last_heartbeat_ms)}">{heartbeatLabel(exchange.last_heartbeat_ms)}</td></tr>
                        </tbody>
                    </table>
                </div>
            {/each}
        </div>
    {:else}
        <div class={styles.empty}>No exchange data available</div>
    {/if}
</div>
