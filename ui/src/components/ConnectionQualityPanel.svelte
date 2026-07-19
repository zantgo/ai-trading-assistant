  <script lang="ts">
      import type { ConnectionQualityReport, QualityWindow } from '../types';
      import styles from './ConnectionQualityPanel.module.css';

      let activeWindow: QualityWindow = $state('one_hour');
      let report: ConnectionQualityReport | null = $state(null);
      let loading = $state(true);
      let error: string | null = $state(null);
      let lastFetchMs = $state(0);

      let pollInterval: ReturnType<typeof setInterval> | null = null;

      async function fetchQuality() {
          try {
              const res = await fetch(`/api/connection-quality?window=${activeWindow}`);
              if (!res.ok) throw new Error(`HTTP ${res.status}`);
              report = await res.json();
              lastFetchMs = Date.now();
              error = null;
          } catch (e) {
              error = e instanceof Error ? e.message : String(e);
          } finally {
              loading = false;
          }
      }

      $effect(() => {
          fetchQuality();
          if (pollInterval) clearInterval(pollInterval);
          pollInterval = setInterval(fetchQuality, 30_000);
          return () => {
              if (pollInterval) clearInterval(pollInterval);
          };
      });

      const isIdle = $derived(
          report &&
          report.uptime_pct === 0 &&
          report.disconnect_count === 0 &&
          report.score === 100
      );

      function scoreClass(score: number): string {
          if (score >= 90) return styles.scoreExcellent;
          if (score >= 75) return styles.scoreGood;
          if (score >= 50) return styles.scoreModerate;
          return styles.scorePoor;
      }

      function uptimeClass(pct: number): string {
          if (pct >= 99) return styles.uptimeExcellent;
          if (pct >= 95) return styles.uptimeGood;
          return styles.uptimePoor;
      }
  </script>

  <div class={styles.container}>
      <div class={styles.header}>
          <h2 class={styles.title}>Connection Quality</h2>
          <div class={styles.tabs}>
              <button class="{styles.tab} {activeWindow === 'one_hour' ? styles.tabActive : ''}" onclick={() => activeWindow = 'one_hour'}>1h</button>
              <button class="{styles.tab} {activeWindow === 'six_hour' ? styles.tabActive : ''}" onclick={() => activeWindow = 'six_hour'}>6h</button>
              <button class="{styles.tab} {activeWindow === 'twenty_four_hour' ? styles.tabActive : ''}" onclick={() => activeWindow = 'twenty_four_hour'}>24h</button>
          </div>
      </div>

      {#if loading}
          <div class={styles.placeholder}>Loading...</div>
      {:else if error}
          <div class={styles.error}>Error: {error}</div>
      {:else if report}
          {#if isIdle}
              <div class={styles.placeholder}>Waiting for events — no connections recorded yet</div>
          {:else}
              <div class={styles.metrics}>
                  <div class={styles.metric}>
                      <div class={styles.metricLabel}>Score</div>
                      <div class="{styles.metricValue} {scoreClass(report.score)}">{report.score.toFixed(1)}</div>
                  </div>
                  <div class={styles.metric}>
                      <div class={styles.metricLabel}>Uptime</div>
                      <div class="{styles.metricValue} {uptimeClass(report.uptime_pct)}">{report.uptime_pct.toFixed(2)}%</div>
                  </div>
                  <div class={styles.metric}>
                      <div class={styles.metricLabel}>Disconnects</div>
                      <div class={styles.metricValue}>{report.disconnect_count}</div>
                  </div>
                  <div class={styles.metric}>
                      <div class={styles.metricLabel}>Avg Reconnect</div>
                      <div class={styles.metricValue}>{report.avg_reconnect_ms.toFixed(0)}ms</div>
                  </div>
                  <div class={styles.metric}>
                      <div class={styles.metricLabel}>Data Loss</div>
                      <div class={styles.metricValue}>{report.total_data_loss_secs}s</div>
                  </div>
                  <div class={styles.metric}>
                      <div class={styles.metricLabel}>Reconstructed Candles</div>
                      <div class={styles.metricValue}>{report.reconstructed_candles}</div>
                  </div>
              </div>
          {/if}
      {:else}
          <div class={styles.placeholder}>No data</div>
      {/if}
  </div>
