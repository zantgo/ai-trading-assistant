  <script lang="ts">
      import ConnectionQualityPanel from './ConnectionQualityPanel.svelte';
      import ClockMonitorPanel from './ClockMonitorPanel.svelte';
      import ExchangeStatusPanel from './ExchangeStatusPanel.svelte';
      import DataQualityPanel from './DataQualityPanel.svelte';
      import DataInfraConfig from './DataInfraConfig.svelte';
      import styles from './DataInfraDashboard.module.css';

      let { section = 'connectivity' }: { section?: string } = $props();
  </script>

  <div class={styles.dashboard}>
    <div class={styles.content}>
        {#if section === 'connectivity'}
            <h3 class={styles.sectionTitle}>Connection Quality</h3>
            <p class={styles.sectionDesc}>
                Monitors WebSocket connection health for Hyperliquid and Bitget feeds.
                Uptime, disconnect count, reconnect latency, and composite quality score
                are tracked across rolling 1-hour, 6-hour, and 24-hour windows.
            </p>
            <ConnectionQualityPanel />
        {:else if section === 'exchange_status'}
            <h3 class={styles.sectionTitle}>Exchange Status</h3>
            <p class={styles.sectionDesc}>
                Live per-exchange connectivity status, active pairs, and reconnect counters.
            </p>
            <ExchangeStatusPanel />
        {:else if section === 'clock_monitor'}
            <h3 class={styles.sectionTitle}>NTP Clock Monitor</h3>
          <p class={styles.sectionDesc}>
              The platform enforces a ≤100µs UTC drift budget via continuous NTP polling
              (see <code>config.toml</code> → <code>[clock_monitor]</code>).
            </p>
            <ClockMonitorPanel />
        {:else if section === 'data_quality'}
            <h3 class={styles.sectionTitle}>Data Quality</h3>
            <p class={styles.sectionDesc}>
                Per-session pipeline reliability metrics: coverage, gaps, outlier rejection,
                out-of-order drops, and reconstructed candle counts.
            </p>
          <DataQualityPanel />
          {:else if section === 'settings'}
              <h3 class={styles.sectionTitle}>Settings</h3>
              <p class={styles.sectionDesc}>
                  Data Infrastructure Engine settings: exchange endpoints, NTP clock monitor
                  parameters, connection resilience, quality windows, and persistence intervals.
              </p>
              <DataInfraConfig />
          {/if}
    </div>
</div>
