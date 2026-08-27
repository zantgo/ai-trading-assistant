<script lang="ts">
    // BteCoverageTab (DIE) — the archived-data surface: per-TF candle
    // coverage, span, theoretical max lookback, and the backfill trigger.
    import styles from '../../styles/engine-dashboard.module.css';
    import { fmtSpan } from '../../lib/studyCharts';

    interface Props {
        coverage: {
            symbol: string; timeframe_secs: number; candle_count: number;
            earliest_secs: number | null; latest_secs: number | null;
            covered_span_secs: number; max_lookback_secs: number; coverage_pct: number;
        }[];
        ladder: number[];
        depthDays: number;
        backfillJob: { status: string; pages_fetched: number; candles_stored: number } | null;
        backfillError: string;
        startBackfill: () => Promise<void>;
        backfilling: boolean;
    }

    let { coverage, ladder, depthDays, backfillJob, backfillError, startBackfill, backfilling }: Props = $props();

    const rows = $derived.by(() => {
        return ladder.map((tf) => {
            const row = coverage.find((c) => c.timeframe_secs === tf);
            return {
                tf,
                count: row?.candle_count ?? 0,
                earliest: row?.earliest_secs ?? null,
                latest: row?.latest_secs ?? null,
                span: row?.covered_span_secs ?? 0,
                pct: row?.coverage_pct ?? 0,
            };
        });
    });

    function date(secs: number | null): string {
        return secs ? new Date(secs * 1000).toLocaleDateString() : '—';
    }
</script>

<div class={styles.card}>
    <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:8px">
        <div>
            <h3 class={styles.cardTitle} style="margin:0">DIE · Archived Data Coverage</h3>
            <p class={styles.infoLine}>
                The candle archive feeds historical backtests. Live completed candles write into
                it continuously; the backfill job pages the exchange backward up to {depthDays} days.
            </p>
        </div>
        <button class={styles.btn} onclick={startBackfill} disabled={backfilling}>
            {backfilling ? 'Starting…' : 'Backfill Archive'}
        </button>
    </div>

    {#if backfillJob}
        <div class="{styles.alertBanner} {styles.alertWarn}" style="margin-top:10px">
            Backfill {backfillJob.status.toUpperCase()} — {backfillJob.pages_fetched} pages ·
            {backfillJob.candles_stored.toLocaleString()} candles stored.
        </div>
    {/if}
    {#if backfillError}
        <div class="{styles.alertBanner} {styles.alertError}" style="margin-top:10px">{backfillError}</div>
    {/if}

    {#if rows.length === 0}
        <div class={styles.empty} style="margin-top:12px">No ladder known yet (fetching config…).</div>
    {:else}
        <table class={styles.table} style="margin-top:12px">
            <thead>
                <tr>
                    <th>Timeframe</th>
                    <th class={styles.tdRight}>Candles</th>
                    <th class={styles.tdRight}>Earliest</th>
                    <th class={styles.tdRight}>Latest</th>
                    <th class={styles.tdRight}>Covered Span</th>
                    <th class={styles.tdRight}>vs {depthDays}d</th>
                    <th style="width:30%">Coverage</th>
                </tr>
            </thead>
            <tbody>
                {#each rows as r (r.tf)}
                    <tr>
                        <td class={styles.tdMono}>{r.tf}s</td>
                        <td class={styles.tdRight}>{r.count > 0 ? r.count.toLocaleString() : '—'}</td>
                        <td class={styles.tdRight}>{date(r.earliest)}</td>
                        <td class={styles.tdRight}>{date(r.latest)}</td>
                        <td class={styles.tdRight}>{r.span > 0 ? fmtSpan(r.span) : '—'}</td>
                        <td class={styles.tdRight}>{r.pct > 0 ? r.pct.toFixed(1) + '%' : '—'}</td>
                        <td>
                            <div style="background:rgba(255,255,255,0.06); border-radius:3px; height:8px; overflow:hidden">
                                <div style="width:{Math.min(100, r.pct)}%; height:100%; background:{r.pct >= 80 ? '#22c55e' : r.pct >= 30 ? '#f59e0b' : '#ef4444'}; transition:width 0.3s"></div>
                            </div>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
        <p class={styles.infoLine} style="margin-top:10px">
            Theoretical max lookback = archive depth ({depthDays}d). Sub-minute TFs have no
            exchange history (HFP-03) — their coverage comes from the live write path only.
        </p>
    {/if}
</div>
