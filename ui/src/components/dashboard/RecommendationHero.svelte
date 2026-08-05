<script lang="ts">
    // RecommendationHero — full-width status band that answers the
    // operator's first question: "Can I trade anything right now?"
    //
    // Three states with sharply contrasting colors:
    //   TRADE       — green, at least one Actionable + READY setup
    //   WAIT        — amber, qualifying setups exist but none are READY
    //   STAND ASIDE — red, no qualifying setup or all STAND_ASIDE
    //
    // Subtext shows the actionable count, the best opportunity symbol
    // + score, and the highest confidence in the actionable set.
    import { useAppStore } from '../../state.svelte';
    import { computeHeroState, pickBestOpportunity, collectActiveSetups } from '../../lib/tradeAggregates';
    import { formatRR } from '../../lib/dashboardColors';
    import styles from './RecommendationHero.module.css';

    const app = useAppStore();

    const states = $derived.by(() => {
        const instances = Object.values(app.instancesMap);
        const hero = computeHeroState(instances);
        const setups = collectActiveSetups(instances);
        const actionable = setups.filter(
            (s) => s.viability === 'Actionable' && s.readiness === 'READY',
        );
        const best = pickBestOpportunity(instances);
        return {
            hero,
            totalCount: setups.length,
            actionableCount: actionable.length,
            bestSymbol: best?.symbol ?? null,
            bestScore: best?.opportunityScore ?? 0,
            bestDir: best?.direction ?? null,
            bestConfidence: best?.confidence ?? 0,
            bestRr: best?.rr ?? 0,
        };
    });

    function heroClass(s: string): string {
        if (s === 'TRADE') return styles.trade ?? '';
        if (s === 'WAIT') return styles.wait ?? '';
        return styles.standAside ?? '';
    }

    function headline(s: string): string {
        if (s === 'TRADE') return 'TRADE';
        if (s === 'WAIT') return 'WAIT';
        return 'STAND ASIDE';
    }

    function subtext(s: typeof states): string {
        if (s.hero === 'TRADE') {
            const symbol = s.bestSymbol ?? '—';
            const dir = s.bestDir === 'LONG' ? 'LONG' : s.bestDir === 'SHORT' ? 'SHORT' : '—';
            const rr = formatRR(s.bestRr);
            return `${s.actionableCount} actionable setup${s.actionableCount === 1 ? '' : 's'} · best ${symbol} ${dir} · R:R ${rr} · confidence ${s.bestConfidence.toFixed(0)}%`;
        }
        if (s.hero === 'WAIT') {
            return `${s.totalCount} candidate setup${s.totalCount === 1 ? '' : 's'} forming — no READY trade yet.`;
        }
        return 'No high-quality opportunities detected — stand aside.';
    }
</script>

<div class="{styles.hero} {heroClass(states.hero)}">
    <div class={styles.left}>
        <div class={styles.label}>MARKET STATUS</div>
        <div class={styles.headline}>{headline(states.hero)}</div>
    </div>
    <div class={styles.right}>
        <div class={styles.subtext}>{subtext(states)}</div>
    </div>
</div>
