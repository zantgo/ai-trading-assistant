<script lang="ts">
    import type { ExchangeStatusReport, ExchangeConnectionState } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './ExchangeStatusPanel.module.css';

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

    function stateClass(state: ExchangeConnectionState): string {
        switch (state) {
            case 'Connected': return styles.stateConnected;
            case 'Connecting': return styles.stateConnecting;
            case 'Reconnecting': return styles.stateReconnecting;
            case 'Disabled': return styles.stateDisabled;
            default: return styles.stateDisconnected;
        }
    }

    function stateLabel(state: ExchangeConnectionState): string {
        switch (state) {
            case 'Connected': return '● Connected';
            case 'Connecting': return '◌ Connecting';
            case 'Reconnecting': return '↻ Reconnecting';
            case 'Disabled': return '✕ Disabled';
            default: return '● Disconnected';
        }
    }

    function stateHint(state: ExchangeConnectionState): string {
        switch (state) {
            case 'Connected': return 'WebSocket stream active';
            case 'Connecting': return 'Establishing WebSocket handshake…';
            case 'Reconnecting': return 'Exponential backoff retry in progress';
            case 'Disabled': return 'Permanently disabled after 5 consecutive failures';
            default: return 'Not connected';
        }
    }

    function heartbeatText(lastHeartbeatMs: number): string {
        if (lastHeartbeatMs <= 0) return 'No heartbeat yet';
        const secs = Math.floor((Date.now() - lastHeartbeatMs) / 1000);
        return `${secs}s ago`;
    }

    function heartbeatClass(lastHeartbeatMs: number): string {
        if (lastHeartbeatMs <= 0) return styles.hbNone;
        const secs = (Date.now() - lastHeartbeatMs) / 1000;
        if (secs < 30) return styles.hbGood;
        if (secs < 60) return styles.hbWarn;
        return styles.hbBad;
    }
</script>

<div class={styles.container}>
    <div class={styles.header}>
        <h2 class={styles.title}>Exchange Status</h2>
    </div>

    {#if loading}
        <div class={styles.placeholder}>Loading...</div>
    {:else if error}
        <div class={styles.error}>Error: {error}</div>
    {:else if filteredExchanges.length > 0}
        <div class={styles.exchanges}>
            {#each filteredExchanges as exchange}
                <div class={styles.exchangeCard}>
                    <div class={styles.exchangeName}>{exchange.name}</div>
                    <div class="{styles.exchangeStatus} {stateClass(exchange.state)}">
                        {stateLabel(exchange.state)}
                    </div>
                    <div class={styles.stateHint}>{stateHint(exchange.state)}</div>
                    <div class={styles.wsUrl}>{exchange.ws_url}</div>
                    <div class={styles.exchangeDetails}>
                        <div class={styles.detailRow}>
                            <span class={styles.detailLabel}>Pairs</span>
                            <span class={styles.detailValue}>{exchange.active_pairs}</span>
                        </div>
                        <div class={styles.detailRow}>
                            <span class={styles.detailLabel}>Reconnects</span>
                            <span class={styles.detailValue}>{exchange.total_reconnects}</span>
                        </div>
                        <div class={styles.detailRow}>
                            <span class={styles.detailLabel}>Last Heartbeat</span>
                            <span class="{styles.detailValue} {heartbeatClass(exchange.last_heartbeat_ms)}">
                                {heartbeatText(exchange.last_heartbeat_ms)}
                            </span>
                        </div>
                    </div>
                </div>
            {/each}
        </div>
    {:else}
        <div class={styles.placeholder}>No exchange data available</div>
    {/if}
</div>
