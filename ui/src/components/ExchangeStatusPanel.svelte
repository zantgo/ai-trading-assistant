<script lang="ts">
    import type { ExchangeStatusReport, ExchangeConnectionState } from '../types';
    import styles from './ExchangeStatusPanel.module.css';

    let report: ExchangeStatusReport | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

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
</script>

<div class={styles.container}>
    <div class={styles.header}>
        <h2 class={styles.title}>Exchange Status</h2>
    </div>

    {#if loading}
        <div class={styles.placeholder}>Loading...</div>
    {:else if error}
        <div class={styles.error}>Error: {error}</div>
    {:else if report}
        <div class={styles.exchanges}>
            {#each report.exchanges as exchange}
                <div class={styles.exchangeCard}>
                    <div class={styles.exchangeName}>{exchange.name}</div>
                    <div class="{styles.exchangeStatus} {stateClass(exchange.state)}">
                        {stateLabel(exchange.state)}
                    </div>
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
                            <span class={styles.detailValue}>
                                {exchange.last_heartbeat_ms > 0
                                    ? `${((Date.now() - exchange.last_heartbeat_ms) / 1000).toFixed(0)}s ago`
                                    : '—'}
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
