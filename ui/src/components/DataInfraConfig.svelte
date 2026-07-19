<script lang="ts">
    import type { ClockStatusResponse } from '../types';
    import { COLORS } from '../lib/statusColors';

    let config: Record<string, unknown> | null = $state(null);
    let clockReport: ClockStatusResponse | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            config = await res.json();
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        }
    }

    async function fetchClock() {
        try {
            const res = await fetch('/api/system/clock');
            if (res.ok) clockReport = await res.json();
        } catch (_) {}
    }

    $effect(() => {
        loading = true;
        Promise.all([fetchConfig(), fetchClock()]).finally(() => loading = false);
        if (pollInterval) clearInterval(pollInterval);
        pollInterval = setInterval(() => { fetchConfig(); fetchClock(); }, 30_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });
</script>

<div class="container">
    {#if loading}
        <div class="placeholder">Loading configuration...</div>
    {:else if error}
        <div class="error">Error: {error}</div>
    {:else if config}
        <div class="section">
            <h3 class="section-title">Exchange Endpoints</h3>
            <table class="table">
                <thead>
                    <tr><th>Exchange</th><th>WebSocket URL</th></tr>
                </thead>
                <tbody>
                    {#if config.hyperliquid}
                        <tr>
                            <td class="ex-name">Hyperliquid</td>
                            <td class="ex-url">{(config.hyperliquid as any).ws_url ?? '—'}</td>
                        </tr>
                    {/if}
                    {#if config.bitget}
                        <tr>
                            <td class="ex-name">Bitget</td>
                            <td class="ex-url">{(config.bitget as any).ws_url ?? '—'}</td>
                        </tr>
                    {/if}
                </tbody>
            </table>
        </div>

        {#if clockReport}
            <div class="section">
                <h3 class="section-title">NTP Clock Monitor</h3>
                <table class="table">
                    <thead>
                        <tr><th>Setting</th><th>Value</th></tr>
                    </thead>
                    <tbody>
                        <tr><td class="key">Threshold</td><td class="val">{clockReport.threshold_micros}µs</td></tr>
                        <tr><td class="key">Breach Action</td><td class="val" style="color: {clockReport.breach_action === 'Panic' ? COLORS.poor : COLORS.good}">{clockReport.breach_action}</td></tr>
                        <tr><td class="key">Sample Count</td><td class="val">{clockReport.sample_count}</td></tr>
                        <tr><td class="key">Breach Count</td><td class="val" style="color: {clockReport.breach_count > 0 ? COLORS.poor : COLORS.good}">{clockReport.breach_count}</td></tr>
                        <tr>
                            <td class="key">NTP Servers</td>
                            <td class="val">{clockReport.ntp_servers.join(', ')}</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        {/if}

        <div class="section">
            <h3 class="section-title">Connection Resilience</h3>
            <table class="table">
                <thead>
                    <tr><th>Setting</th><th>Value</th></tr>
                </thead>
                <tbody>
                    <tr><td class="key">Initial Backoff</td><td class="val">1s</td></tr>
                    <tr><td class="key">Max Backoff</td><td class="val">30s</td></tr>
                    <tr><td class="key">Jitter</td><td class="val">±20%</td></tr>
                    <tr><td class="key">Permanent Disable</td><td class="val">5 consecutive failures</td></tr>
                    <tr><td class="key">Session Reset</td><td class="val">After 300s stable connection</td></tr>
                </tbody>
            </table>
        </div>

        <div class="section">
            <h3 class="section-title">Quality Windows</h3>
            <table class="table">
                <thead>
                    <tr><th>Window</th><th>Duration</th></tr>
                </thead>
                <tbody>
                    <tr><td class="key">1 Hour</td><td class="val">3,600 seconds</td></tr>
                    <tr><td class="key">6 Hour</td><td class="val">21,600 seconds</td></tr>
                    <tr><td class="key">24 Hour</td><td class="val">86,400 seconds</td></tr>
                </tbody>
            </table>
        </div>

        <div class="section">
            <h3 class="section-title">Persistence</h3>
            <table class="table">
                <thead>
                    <tr><th>Setting</th><th>Value</th></tr>
                </thead>
                <tbody>
                    <tr><td class="key">Quality sample interval</td><td class="val">60 seconds</td></tr>
                    <tr><td class="key">Snapshot retention</td><td class="val">7 days</td></tr>
                    <tr><td class="key">Liquidation retention</td><td class="val">90 days</td></tr>
                </tbody>
            </table>
        </div>
    {:else}
        <div class="placeholder">No configuration available</div>
    {/if}
</div>

<style>
    .container {
        color: #e0e0e0;
        font-family: var(--mono);
    }
    .section {
        margin-bottom: 1.5rem;
    }
    .section-title {
        font-size: 0.9rem;
        color: #888;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        margin: 0 0 0.75rem 0;
        padding-bottom: 0.4rem;
        border-bottom: 1px solid #2a2e39;
    }
    .table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.82rem;
    }
    .table th {
        text-align: left;
        color: #666;
        font-weight: 600;
        text-transform: uppercase;
        font-size: 0.7rem;
        letter-spacing: 0.05em;
        padding: 0.35rem 0;
        border-bottom: 1px solid #1a1d26;
    }
    .table td {
        padding: 0.4rem 0.5rem 0.4rem 0;
    }
    .key {
        color: #666;
        width: 1%;
        white-space: nowrap;
        padding-right: 2rem !important;
    }
    .val {
        color: #e0e0e0;
    }
    .ex-name {
        color: #e0e0e0;
        font-weight: 600;
        width: 1%;
        white-space: nowrap;
        padding-right: 2rem !important;
    }
    .ex-url {
        color: #888;
        font-size: 0.75rem;
        word-break: break-all;
    }
    .placeholder {
        text-align: center;
        padding: 2rem;
        color: #666;
    }
    .error {
        padding: 1rem;
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid #ef4444;
        border-radius: 4px;
        color: #ef4444;
    }
</style>
