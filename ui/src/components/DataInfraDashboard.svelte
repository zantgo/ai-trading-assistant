<script lang="ts">
    import ConnectionQualityPanel from './ConnectionQualityPanel.svelte';

    let activeSection = $state<'connectivity' | 'exchange_status' | 'clock_monitor'>('connectivity');
</script>

<div style="display:flex; height:100%; background:#000; color:#fff; font-family:monospace;">
    <!-- Left sidebar (lateral panel like Home) -->
    <div style="width:200px; border-right:1px solid #2a2e39; padding:1rem 0; flex-shrink:0;">
        <h2 style="font-size:0.75rem; text-transform:uppercase; letter-spacing:0.1em; color:#5a5f6e; padding:0 1rem; margin-bottom:0.75rem;">DATA INFRASTRUCTURE</h2>
        <button
            style="display:block; width:100%; text-align:left; padding:0.5rem 1rem; border:none; background:{activeSection === 'connectivity' ? '#1a1d26' : 'transparent'}; color:{activeSection === 'connectivity' ? '#fff' : '#888'}; cursor:pointer; font-size:0.82rem; font-family:monospace;"
            onclick={() => activeSection = 'connectivity'}
        >
            ⚡ Connectivity
        </button>
        <button
            style="display:block; width:100%; text-align:left; padding:0.5rem 1rem; border:none; background:{activeSection === 'exchange_status' ? '#1a1d26' : 'transparent'}; color:{activeSection === 'exchange_status' ? '#fff' : '#888'}; cursor:pointer; font-size:0.82rem; font-family:monospace;"
            onclick={() => activeSection = 'exchange_status'}
        >
            🏦 Exchange Status
        </button>
        <button
            style="display:block; width:100%; text-align:left; padding:0.5rem 1rem; border:none; background:{activeSection === 'clock_monitor' ? '#1a1d26' : 'transparent'}; color:{activeSection === 'clock_monitor' ? '#fff' : '#888'}; cursor:pointer; font-size:0.82rem; font-family:monospace;"
            onclick={() => activeSection = 'clock_monitor'}
        >
            🕒 NTP Clock Monitor
        </button>
    </div>

    <!-- Right content -->
    <div style="flex:1; padding:1.5rem; overflow-y:auto;">
        {#if activeSection === 'connectivity'}
            <div>
                <h3 style="font-size:1rem; margin-bottom:0.5rem;">Connection Quality</h3>
                <p style="color:#888; font-size:0.8rem; margin-bottom:1rem;">
                    Monitors WebSocket connection health for Hyperliquid and Bitget feeds.
                    Uptime, disconnect count, reconnect latency, and composite quality score
                    are tracked across rolling 1-hour, 6-hour, and 24-hour windows.
                </p>
                <ConnectionQualityPanel />
            </div>
        {:else if activeSection === 'exchange_status'}
            <div>
                <h3 style="font-size:1rem; margin-bottom:0.5rem;">Exchange Status</h3>
                <p style="color:#888; font-size:0.8rem;">Exchange health and endpoint reachability monitoring — coming soon.</p>
            </div>
        {:else if activeSection === 'clock_monitor'}
            <div>
                <h3 style="font-size:1rem; margin-bottom:0.5rem;">NTP Clock Monitor</h3>
                <p style="color:#888; font-size:0.8rem;">
                    The platform enforces a ≤50µs UTC drift budget via continuous NTP polling
                    (see <code>config.toml</code> → <code>[clock_monitor]</code>).
                    Current drift, last poll time, and breach history will be displayed here.
                </p>
                <p style="color:#ff9800; font-size:0.8rem; margin-top:1rem;">
                    Status: monitor running (check server logs for real-time drift values)
                </p>
            </div>
        {/if}
    </div>
</div>
