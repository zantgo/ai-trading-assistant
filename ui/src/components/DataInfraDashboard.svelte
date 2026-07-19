  <script lang="ts">
      import ConnectionQualityPanel from './ConnectionQualityPanel.svelte';
      import ClockMonitorPanel from './ClockMonitorPanel.svelte';
      import ExchangeStatusPanel from './ExchangeStatusPanel.svelte';
      import DataQualityPanel from './DataQualityPanel.svelte';
      import DataInfraConfig from './DataInfraConfig.svelte';
      import styles from './DataInfraDashboard.module.css';

      type Section = 'connectivity' | 'exchange_status' | 'clock_monitor' | 'data_quality' | 'config';
      let activeSection: Section = $state('connectivity');
  </script>

  <div class={styles.dashboard}>
      <div class={styles.sidebar}>
          <h2 class={styles.sidebarTitle}>DATA INFRASTRUCTURE</h2>
          <button class="{styles.sidebarBtn} {activeSection === 'connectivity' ? styles.sidebarBtnActive : ''}" onclick={() => activeSection = 'connectivity'}>Connectivity</button>
          <button class="{styles.sidebarBtn} {activeSection === 'exchange_status' ? styles.sidebarBtnActive : ''}" onclick={() => activeSection = 'exchange_status'}>Exchange Status</button>
          <button class="{styles.sidebarBtn} {activeSection === 'clock_monitor' ? styles.sidebarBtnActive : ''}" onclick={() => activeSection = 'clock_monitor'}>NTP Clock Monitor</button>
          <button class="{styles.sidebarBtn} {activeSection === 'data_quality' ? styles.sidebarBtnActive : ''}" onclick={() => activeSection = 'data_quality'}>Data Quality</button>
          <button class="{styles.sidebarBtn} {activeSection === 'config' ? styles.sidebarBtnActive : ''}" onclick={() => activeSection = 'config'}>Configuration</button>
    </div>

    <div class={styles.content}>
        {#if activeSection === 'connectivity'}
            <h3 class={styles.sectionTitle}>Connection Quality</h3>
            <p class={styles.sectionDesc}>
                Monitors WebSocket connection health for Hyperliquid and Bitget feeds.
                Uptime, disconnect count, reconnect latency, and composite quality score
                are tracked across rolling 1-hour, 6-hour, and 24-hour windows.
            </p>
            <ConnectionQualityPanel />
        {:else if activeSection === 'exchange_status'}
            <h3 class={styles.sectionTitle}>Exchange Status</h3>
            <p class={styles.sectionDesc}>
                Live per-exchange connectivity status, active pairs, and reconnect counters.
            </p>
            <ExchangeStatusPanel />
        {:else if activeSection === 'clock_monitor'}
            <h3 class={styles.sectionTitle}>NTP Clock Monitor</h3>
          <p class={styles.sectionDesc}>
              The platform enforces a ≤100µs UTC drift budget via continuous NTP polling
              (see <code>config.toml</code> → <code>[clock_monitor]</code>).
            </p>
            <ClockMonitorPanel />
        {:else if activeSection === 'data_quality'}
            <h3 class={styles.sectionTitle}>Data Quality</h3>
            <p class={styles.sectionDesc}>
                Per-session pipeline reliability metrics: coverage, gaps, outlier rejection,
                out-of-order drops, and reconstructed candle counts.
            </p>
          <DataQualityPanel />
          {:else if activeSection === 'config'}
              <h3 class={styles.sectionTitle}>Configuration</h3>
              <p class={styles.sectionDesc}>
                  Data Infrastructure Engine settings: exchange endpoints, NTP clock monitor
                  parameters, connection resilience, quality windows, and persistence intervals.
              </p>
              <DataInfraConfig />
          {/if}
    </div>
</div>
