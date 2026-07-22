<script lang="ts">
    // Liquidity cluster-refresh status pill.
    //
    // Polls `/api/liquidity/cluster-status?symbol=<pair>` every 3 seconds
    // and renders a small colored dot + label indicating whether the
    // per-TF cluster refresh is alive. The colored dot semantics:
    //
    //   - green  (Ok)      — most recent refresh produced a valid matrix
    //   - yellow (Pending) — no refresh attempted yet (cold boot)
    //   - red    (Skipped) — refresh failed; hover for the skip reason
    //
    // The pill is rendered inline in the ChartToggles bar next to the
    // LIQ HEATMAP toggle so the operator sees a status signal whenever
    // they look at the heatmap pill. Without this, a failing refresh is
    // invisible: the heatmap just stays empty.

    import { onDestroy, onMount } from 'svelte';
    import styles from './LiquidityStatusPanel.module.css';

    /// `null` while a fetch is in flight; `undefined` on the very first
    /// load (before any data has arrived).
    type Status = 'Ok' | 'Pending' | 'Skipped' | 'Stale' | 'error' | 'fetching';

    let { symbol }: { symbol: string } = $props();

    let status = $state<Status>('fetching');
    let skipReason = $state<string | null>(null);
    let lastUpdateMs = $state<number>(0);
    let timer: ReturnType<typeof setInterval> | null = null;
    let abortCtrl: AbortController | null = null;

    async function fetchStatus(): Promise<void> {
        if (!symbol) return;
        abortCtrl?.abort();
        abortCtrl = new AbortController();
        try {
            const res = await fetch(
                `/api/liquidity/cluster-status?symbol=${encodeURIComponent(symbol)}`,
                { signal: abortCtrl.signal },
            );
            if (!res.ok) {
                status = 'error';
                skipReason = `HTTP ${res.status}`;
                return;
            }
            const body = await res.json() as {
                slots?: Record<string, { status: string; last_skip_reason?: string | null }>;
            };
            if (!body?.slots || typeof body.slots !== 'object') {
                status = 'error';
                skipReason = 'malformed response (no `slots` field)';
                return;
            }
            const slotArr = Object.entries(body.slots).map(([, v]) => v as { status: string; last_skip_reason?: string | null });
            // Normalize wire-format SCREAMING_SNAKE_CASE to our canonical
            // PascalCase union. Unknown values fall through to 'fetching'
            // (neutral) so a future server-side enum addition doesn't
            // crash the UI — operators see the literal value in the
            // tooltip rather than the pill going red.
            const normalize = (s: string): Status => {
                const upper = s.toUpperCase();
                if (upper === 'OK') return 'Ok';
                if (upper === 'PENDING') return 'Pending';
                if (upper === 'STALE') return 'Stale';
                if (upper === 'SKIPPED') return 'Skipped';
                return 'fetching';
            };
            const normalizedSlots = slotArr.map(s => ({ ...s, _status: normalize(s.status) }));
            const pickRank = (s: Status) => {
                switch (s) {
                    case 'Ok': return 0;
                    case 'Pending': return 1;
                    case 'Stale': return 2;
                    case 'Skipped': return 3;
                    default: return 0;
                }
            };
            let worst: Status = 'Ok';
            let worstRank = -1;
            for (const s of normalizedSlots) {
                const rank = pickRank(s._status);
                if (rank > worstRank) {
                    worstRank = rank;
                    worst = s._status;
                }
            }
            status = worst;
            // Capture the most informative skip reason across slots.
            const skips = slotArr
                .map(s => s.last_skip_reason)
                .filter((r): r is string => typeof r === 'string' && r.length > 0);
            skipReason = skips.length > 0 ? skips[0] : null;
            lastUpdateMs = Date.now();
        } catch (err) {
            if ((err as Error)?.name === 'AbortError') return;
            status = 'error';
            skipReason = (err as Error)?.message ?? 'unknown fetch error';
        }
    }

    onMount(() => {
        fetchStatus();
        timer = setInterval(fetchStatus, 3000);
    });

    onDestroy(() => {
        if (timer != null) clearInterval(timer);
        abortCtrl?.abort();
    });

    function pillLabel(s: Status): string {
        switch (s) {
            case 'Ok': return 'LIQ OK';
            case 'Pending': return 'LIQ BOOT';
            case 'Stale': return 'LIQ STALE';
            case 'Skipped': return 'LIQ ERR';
            case 'error': return 'LIQ DOWN';
            case 'fetching': return 'LIQ ...';
            default: return 'LIQ ?';
        }
    }

    const tooltip = $derived(
        status === 'Ok'
            ? `Cluster refresh OK for ${symbol} (last update ${new Date(lastUpdateMs).toLocaleTimeString()})`
            : status === 'Skipped'
                ? `Cluster refresh failing for ${symbol}: ${skipReason ?? '(no reason returned)'}`
                : status === 'Pending'
                    ? `Cluster refresh not yet started for ${symbol}`
                    : status === 'Stale'
                        ? `Cluster matrix expired for ${symbol} (TTL elapsed)`
                        : status === 'error'
                            ? `Cluster status endpoint unreachable: ${skipReason ?? 'unknown'}`
                            : `Cluster status: ${status}`,
    );
</script>

<div class="{styles.pill} {styles[`pill_${status.toLowerCase()}`]}" title={tooltip}>
    <span class={styles.dot}></span>
    <span class={styles.label}>{pillLabel(status)}</span>
</div>
