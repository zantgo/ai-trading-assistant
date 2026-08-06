<script lang="ts">
    // GeneralDashboard — system-wide Market Overview for the Market
    // Monitoring engine. Composed of 11 sub-components in
    // `./dashboard/`. The dashboard's role is to answer the operator's
    // first question: "Can I trade anything right now?" — within ~3 s
    // of opening the workspace.
    //
    // Layout (top-down) — v7.0-prod:
    //   1. LayerHeader (L7 MARKET OVERVIEW) — canonical badge + meta
    //      chips (health, systemic risk, sync) + status pill; trailing
    //      slot hosts the UTC clock + scan strip + panel title.
    //   2. RecommendationHero (TRADE / WAIT / STAND ASIDE)
    //   3. Header KPI strip (6 cards)
    //   4. 4-up card row: Trade Opportunities, Risk Distribution,
    //      Signal Quality, Direction
    //   5. Market Health card (4 sub-dim bars)
    //   6. Regime Distribution (ASCII bars)
    //   7. Asset Rankings table (9-column leaderboard)
    //   8. Watchlist runner button (CTA)
    import type { WsState } from '../lib/websocket.svelte';
    import { useAppStore } from '../state.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL7OverviewHeader, type LayerHeaderSpec } from '../lib/layerHeader';
    import styles from './GeneralDashboard.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import WatchlistRunnerButton from './WatchlistRunnerButton.svelte';
    import UtcClockBadge from './dashboard/UtcClockBadge.svelte';
    import ScanStatusStrip from './dashboard/ScanStatusStrip.svelte';
    import RecommendationHero from './dashboard/RecommendationHero.svelte';
    import HeaderKpiStrip from './dashboard/HeaderKpiStrip.svelte';
    import TradeOpportunitiesCard from './dashboard/TradeOpportunitiesCard.svelte';
    import RiskDistributionCard from './dashboard/RiskDistributionCard.svelte';
    import SignalQualityCard from './dashboard/SignalQualityCard.svelte';
    import DirectionDistributionCard from './dashboard/DirectionDistributionCard.svelte';
    import MarketHealthCard from './dashboard/MarketHealthCard.svelte';
    import RegimeDistributionCard from './dashboard/RegimeDistributionCard.svelte';
    import AssetRankingsTable from './dashboard/AssetRankingsTable.svelte';

    interface Props {
        wssMap: Record<string, WsState>;
    }

    let { wssMap }: Props = $props();

    const app = useAppStore();
    const totalCount = $derived(Object.keys(app.instancesMap).length);

    // L7 LayerHeader — sourced from the system-wide Overview Matrix. The
    // status pill mirrors the L7 fetch state (live/stale/error) so
    // the operator can see at a glance whether the synthesis is fresh.
    const now = $derived(Date.now());
    const headerSpec = $derived<LayerHeaderSpec>(buildL7OverviewHeader(
        app.overviewMatrix,
        {
            lastSuccessMs: app.lastOverviewFetchMs,
            lastErrorMs: app.lastOverviewErrorMs,
            now,
            pollIntervalMs: 3000,
        },
    ));
</script>

<div class={styles.dashboardView}>
    <div class={styles.content}>
        {#if totalCount === 0}
            <div class={styles.featurePlaceholder}>
                <SvgIcon name="layoutDashboard" size={64} />
                <h2 class={styles.featurePlaceholderTitle}>Market Overview</h2>
                <p class={styles.featurePlaceholderMsg}>
                    Add workspaces to see system-wide market intelligence across all monitored pairs.
                </p>
            </div>
        {:else}
            <!-- L7 HEADER (v7.0-prod — shared chrome across all MME tabs) -->
            <LayerHeader spec={headerSpec}>
                {#snippet trailing()}
                    <div class={styles.header}>
                        <div class={styles.headerLeft}>
                            <h2 class={styles.title}>MARKET OVERVIEW</h2>
                            <UtcClockBadge />
                        </div>
                        <ScanStatusStrip />
                    </div>
                {/snippet}
            </LayerHeader>

            <RecommendationHero />

            <HeaderKpiStrip />

            <div class={styles.grid4}>
                <TradeOpportunitiesCard />
                <RiskDistributionCard />
                <SignalQualityCard />
                <DirectionDistributionCard />
            </div>

            <MarketHealthCard />

            <RegimeDistributionCard />

            <AssetRankingsTable />
        {/if}

        <WatchlistRunnerButton {wssMap} />
    </div>
</div>
