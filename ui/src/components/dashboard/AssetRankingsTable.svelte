<script lang="ts">
    // AssetRankingsTable — 9-column leaderboard with sortable column
    // headers. Default sort is by `opportunityScore` descending — the
    // operator's first question after a glance at the hero is "which
    // pair is best?"
    //
    // Columns: Symbol, Price, Bias, Signal, Direction, R:R, Score,
    //          Confidence, Risk, Updated.
    import { useAppStore } from '../../state.svelte';
    import { formatRelativeTime } from '../../lib/relTime';
    import {
        biasColor,
        directionColor,
        directionLabel,
        formatRR,
        rrColor,
        scoreColor,
        signalLabel,
        asciiBar,
    } from '../../lib/dashboardColors';
    import styles from './AssetRankingsTable.module.css';

    const app = useAppStore();

    type SortKey = 'symbol' | 'price' | 'bias' | 'signal' | 'direction' | 'rr' | 'score' | 'confidence' | 'risk' | 'updated';
    type SortDir = 'asc' | 'desc';
    let sortKey = $state<SortKey>('score');
    let sortDir = $state<SortDir>('desc');

    let tick = $state(0);
    $effect(() => {
        const id = setInterval(() => { tick = tick + 1; }, 1000);
        return () => clearInterval(id);
    });

    interface Row {
        symbol: string;
        price: string;
        bias: string;
        signal: 'BUY' | 'SELL' | 'WAIT';
        direction: 'LONG' | 'SHORT' | 'NEUTRAL';
        rr: number;
        score: number;
        confidence: number;
        risk: number;
        updatedMs: number | null;
        connected: boolean;
    }

    const rows = $derived.by((): Row[] => {
        const out: Row[] = [];
        for (const [key, inst] of Object.entries(app.instancesMap)) {
            const opp = inst.opportunity;
            const adv = inst.advisory;
            const analysis = inst.analysis;
            const risk = inst.risk;
            const guidance = adv?.directional_guidance ?? null;
            const direction = directionLabel(guidance);
            const signal = signalLabel(guidance);
            // Aggregate opportunity score: prefer highest per-profile score,
            // fall back to opportunity_score from the matrix.
            let score = 0;
            if (opp?.profiles && opp.profiles.length > 0) {
                score = Math.max(...opp.profiles.map((p) => p.score ?? 0));
            } else if (opp?.opportunity_score != null) {
                score = opp.opportunity_score;
            }
            // Per-side R:R resolved by bias.
            const bias = analysis?.bias ?? null;
            const isBearish = bias === 'Bearish' || bias === 'StrongBearish';
            const rr = isBearish
                ? (opp?.short_expected_rr_internal ?? 0)
                : (opp?.long_expected_rr_internal ?? 0);
            const confidence = adv?.confidence_assessment ?? 0;
            const riskScore = risk?.overall_risk?.score ?? 0;
            const snap = inst.microTerm?.latestSnapshot as { timestamp?: number } | null;
            const ts = snap?.timestamp ?? null;
            out.push({
                symbol: inst.symbol,
                price: inst.microTerm?.priceText ?? '--',
                bias: analysis?.bias ?? 'Neutral',
                signal,
                direction,
                rr,
                score,
                confidence,
                risk: riskScore,
                updatedMs: ts,
                connected: inst.isConnected,
            });
        }
        // Sort
        const dir = sortDir === 'asc' ? 1 : -1;
        out.sort((a, b) => {
            const av = (a as any)[sortKey];
            const bv = (b as any)[sortKey];
            if (typeof av === 'string' && typeof bv === 'string') {
                return av.localeCompare(bv) * dir;
            }
            if (av == null && bv != null) return 1;
            if (av != null && bv == null) return -1;
            if (av == null && bv == null) return 0;
            return (Number(av) - Number(bv)) * dir;
        });
        return out;
    });

    function toggleSort(k: SortKey) {
        if (sortKey === k) {
            sortDir = sortDir === 'asc' ? 'desc' : 'asc';
        } else {
            sortKey = k;
            sortDir = k === 'symbol' || k === 'bias' ? 'asc' : 'desc';
        }
    }

    function arrow(k: SortKey): string {
        if (sortKey !== k) return '';
        return sortDir === 'asc' ? ' ↑' : ' ↓';
    }

    function rel(ms: number | null): string {
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        tick;
        return formatRelativeTime(ms).label;
    }
</script>

<div class={styles.tableSection}>
    <div class={styles.tableHeader}>
        <h3 class={styles.sectionTitle}>ASSET RANKINGS</h3>
        <span class={styles.sortHint}>click column to sort</span>
    </div>
    <div class={styles.tableWrap}>
        <table class={styles.table}>
            <thead>
                <tr>
                    <th class={styles.th} onclick={() => toggleSort('symbol')}>Symbol{arrow('symbol')}</th>
                    <th class={styles.th} onclick={() => toggleSort('price')}>Price{arrow('price')}</th>
                    <th class={styles.th} onclick={() => toggleSort('bias')}>Bias{arrow('bias')}</th>
                    <th class={styles.th} onclick={() => toggleSort('signal')}>Signal{arrow('signal')}</th>
                    <th class={styles.th} onclick={() => toggleSort('direction')}>Direction{arrow('direction')}</th>
                    <th class={styles.th} onclick={() => toggleSort('rr')}>R:R{arrow('rr')}</th>
                    <th class={styles.th} onclick={() => toggleSort('score')}>Score{arrow('score')}</th>
                    <th class={styles.th} onclick={() => toggleSort('confidence')}>Confidence{arrow('confidence')}</th>
                    <th class={styles.th} onclick={() => toggleSort('risk')}>Risk{arrow('risk')}</th>
                    <th class={styles.th} onclick={() => toggleSort('updated')}>Updated{arrow('updated')}</th>
                </tr>
            </thead>
            <tbody>
                {#each rows as r (r.symbol)}
                    <tr class={styles.tr}>
                        <td class={styles.tdSymbol}>
                            <span class={styles.statusDot} class:active={r.connected}></span>
                            {r.symbol}
                        </td>
                        <td class={styles.tdMono}>{r.price}</td>
                        <td class={styles.td} style="color: {biasColor(r.bias)}">{r.bias}</td>
                        <td class={styles.td}>
                            <span class={styles.signal} style="color: {directionColor(r.direction)}">
                                {r.signal}
                            </span>
                        </td>
                        <td class={styles.td} style="color: {directionColor(r.direction)}">{r.direction}</td>
                        <td class={styles.td} style="color: {rrColor(r.rr)}">{formatRR(r.rr)}</td>
                        <td class={styles.td}>
                            <span class={styles.scoreCell}>
                                <span class={styles.scoreVal} style="color: {scoreColor(r.score)}">{r.score.toFixed(0)}</span>
                                <span class={styles.scoreBar}>{asciiBar(r.score, 8)}</span>
                            </span>
                        </td>
                        <td class={styles.td}>{r.confidence.toFixed(0)}%</td>
                        <td class={styles.td} style="color: {r.risk >= 60 ? '#ef4444' : r.risk >= 40 ? '#f59e0b' : '#22c55e'}">
                            {r.risk.toFixed(0)}
                        </td>
                        <td class={styles.tdUpdated}>{rel(r.updatedMs)}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
</div>
