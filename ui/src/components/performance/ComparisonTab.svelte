<script lang="ts">
    // ComparisonTab — v10: the data → information → learning table.
    // Rows = persisted sessions + backtest runs; verdict badges carry the
    // NHST classification. A session picker drills into the session-scoped
    // PAE payloads.
    import { onMount } from 'svelte';
    import styles from './ComparisonTab.module.css';

    interface Row {
        kind: 'session' | 'backtest';
        id: number;
        label: string;
        mode: string;
        trades: number;
        win_rate: number;
        profit_factor: number | null;
        expectancy: number;
        sharpe: number | null;
        max_drawdown_pct: number;
        verdict: string | null;
    }

    let rows = $state<Row[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let sessions = $state<{ id: number; label: string }[]>([]);
    let selectedSession = $state<number | null>(null);
    let sessionReport = $state<any>(null);
    let reportLoading = $state(false);

    onMount(() => {
        void load();
    });

    async function load() {
        loading = true; error = null;
        try {
            const [cmpRes, sesRes] = await Promise.all([
                fetch('/api/analytics/comparison'),
                fetch('/api/sessions'),
            ]);
            if (!cmpRes.ok || !sesRes.ok) throw new Error('comparison fetch failed');
            const cmp = await cmpRes.json();
            rows = cmp.rows ?? [];
            const ses = await sesRes.json();
            sessions = (ses.sessions ?? []).map((s: any) => ({ id: s.id, label: `SESSION #${String(s.id).padStart(4, '0')} (${s.mode})` }));
        } catch (e: any) {
            error = e?.message ?? 'failed to load comparison';
        } finally { loading = false; }
    }

    async function loadSession(id: number | null) {
        selectedSession = id;
        if (id == null) { sessionReport = null; return; }
        reportLoading = true;
        try {
            const res = await fetch(`/api/sessions/${id}/analytics`);
            if (res.ok) sessionReport = await res.json();
        } finally { reportLoading = false; }
    }

    function fmt(v: number | null | undefined, digits = 2): string {
        if (v == null || !Number.isFinite(v)) return '—';
        return v.toFixed(digits);
    }

    function verdictClass(v: string | null): string {
        if (!v) return styles.verdictNone;
        if (v.includes('StrongEdge')) return styles.verdictStrong;
        if (v.includes('ModerateEdge')) return styles.verdictModerate;
        if (v.includes('Weak')) return styles.verdictWeak;
        if (v.includes('Insufficient')) return styles.verdictNone;
        return styles.verdictNegative;
    }
</script>

<div class={styles.wrap}>
    <div class={styles.controls}>
        <label class={styles.pickerLabel} for="sessionPicker">Session scope</label>
        <select
            id="sessionPicker"
            class={styles.picker}
            value={selectedSession ?? ''}
            onchange={(e) => loadSession(Number((e.target as HTMLSelectElement).value) || null)}
        >
            <option value="">— all sessions —</option>
            {#each sessions as s (s.id)}
                <option value={s.id}>{s.label}</option>
            {/each}
        </select>
        {#if reportLoading}<span class={styles.hint}>loading…</span>{/if}
    </div>

    {#if sessionReport}
        <div class={styles.report}>
            <span class={styles.reportTitle}>SESSION #{String(sessionReport.session_id).padStart(4, '0')}</span>
            <span class={styles.hint}>
                {sessionReport.counts.market_snapshots} market snapshots · {sessionReport.counts.trades} trades
            </span>
        </div>
    {/if}

    {#if loading}
        <div class={styles.state}>Loading comparison…</div>
    {:else if error}
        <div class={styles.state}>{error}</div>
    {:else}
        <table class={styles.table}>
            <thead>
                <tr>
                    <th>Run</th>
                    <th>Mode</th>
                    <th class={styles.num}>Trades</th>
                    <th class={styles.num}>WR %</th>
                    <th class={styles.num}>PF</th>
                    <th class={styles.num}>Expectancy</th>
                    <th class={styles.num}>Sharpe</th>
                    <th class={styles.num}>maxDD %</th>
                    <th>Verdict</th>
                </tr>
            </thead>
            <tbody>
                {#each rows as r (r.kind + '-' + r.id)}
                    <tr>
                        <td class={styles.label}>{r.label}</td>
                        <td><span class={styles.modeChip}>{r.mode}</span></td>
                        <td class={styles.num}>{r.trades}</td>
                        <td class={styles.num}>{fmt(r.win_rate, 1)}</td>
                        <td class={styles.num}>{fmt(r.profit_factor)}</td>
                        <td class={styles.num}>{fmt(r.expectancy, 3)}</td>
                        <td class={styles.num}>{fmt(r.sharpe)}</td>
                        <td class={styles.num}>{fmt(r.max_drawdown_pct, 1)}</td>
                        <td><span class="{styles.verdict} {verdictClass(r.verdict)}">{r.verdict ?? '—'}</span></td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
