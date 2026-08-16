<script lang="ts">
    // GeneralDashboard — system-wide Market Overview for the Market
    // Monitoring engine. Composed of 11 sub-components in
    // `./dashboard/`. The dashboard's role is to answer the operator's
    // first question: "Can I trade anything right now?" — within ~3 s
    // of opening the workspace.
    //
    // Layout (top-down) — v7.0-prod:
    //   1. LayerHeader (L7 MARKET OVERVIEW) — canonical badge + meta
    //      chips (health, systemic risk, sync) + status pill; the
    //      stackRight variant pins the LIVE pill over the ExportDataButton
    //      in a right-edge column.
    //   2. Scan bar (MARKET OVERVIEW title + UTC clock + scan strip)
    //   3. RecommendationHero (TRADE / WAIT / STAND ASIDE)
    //   4. Header KPI strip (6 cards)
    //   5. 5-up card row: Trade Opportunities, Risk Distribution,
    //      Signal Quality, Direction, Market Alignment
    //   6. Market Health card (4 sub-dim bars)
    //   7. Regime Distribution (ASCII bars)
    //   8. Asset Rankings table (11-column leaderboard incl. MTF cols)
    //   9. Bottom toolbar: [SCHEDULE SNAPSHOTS] [SCAN WATCHLIST] buttons
    //      grouped and centered; instructional copy lives inside the
    //      watchlist scanner modal
    import type { WsState } from '../lib/websocket.svelte';
    import { useAppStore } from '../state.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL7OverviewHeader, type LayerHeaderSpec } from '../lib/layerHeader';
    import styles from './GeneralDashboard.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import WatchlistRunnerButton from './WatchlistRunnerButton.svelte';
    import SnapshotSchedulerButton from './SnapshotSchedulerButton.svelte';
    import UtcClockBadge from './dashboard/UtcClockBadge.svelte';
    import ScanStatusStrip from './dashboard/ScanStatusStrip.svelte';
    import RecommendationHero from './dashboard/RecommendationHero.svelte';
    import HeaderKpiStrip from './dashboard/HeaderKpiStrip.svelte';
    import TradeOpportunitiesCard from './dashboard/TradeOpportunitiesCard.svelte';
    import RiskDistributionCard from './dashboard/RiskDistributionCard.svelte';
    import SignalQualityCard from './dashboard/SignalQualityCard.svelte';
    import DirectionDistributionCard from './dashboard/DirectionDistributionCard.svelte';
    import MarketAlignmentCard from './dashboard/MarketAlignmentCard.svelte';
    import MarketHealthCard from './dashboard/MarketHealthCard.svelte';
    import RegimeDistributionCard from './dashboard/RegimeDistributionCard.svelte';
    import AssetRankingsTable from './dashboard/AssetRankingsTable.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildOverviewTabExport } from '../lib/exportBuilders/overviewTab';

    interface Props {
        wssMap: Record<string, WsState>;
    }

    let { wssMap }: Props = $props();

    const app = useAppStore();
    const totalCount = $derived(Object.values(app.instancesMap).filter(i => i.instanceId).length);

    // L7 LayerHeader — sourced from the system-wide Overview Matrix. The
    // status pill mirrors the L7 fetch state (live/stale/error) so
    // the operator can see at a glance whether the synthesis is fresh.
    const headerSpec = $derived<LayerHeaderSpec>(buildL7OverviewHeader(
        app.overviewMatrix,
        {
            lastSuccessMs: app.lastOverviewFetchMs,
            lastErrorMs: app.lastOverviewErrorMs,
            now: Date.now(),
            pollIntervalMs: 3000,
        },
    ));

    // EXPORT DATA — mirrors the panel 1:1 via the
    // `lib/exportBuilders/overviewTab.ts` builder (see that file for
    // the block ↔ sub-component mapping).
    const buildExport = $derived(() => {
        const instances = Object.values(app.instancesMap);
        return buildOverviewTabExport({
            overviewMatrix: app.overviewMatrix,
            instances,
            headerSpec,
            nowMs: Date.now(),
        });
    });
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
            <!-- L7 HEADER (v7.0-prod — shared chrome across all MME tabs).
                 stackRight stacks the LIVE pill over the EXPORT DATA button
                 in a right-edge column; the scan bar below keeps the
                 MARKET OVERVIEW title + UTC clock + scan strip. -->
            <LayerHeader spec={headerSpec} stackRight>
                {#snippet trailing()}
                    <ExportDataButton onExport={buildExport} title="Copy all Overview data as JSON" />
                {/snippet}
            </LayerHeader>

            <div class={styles.scanBar}>
                <div class={styles.headerLeft}>
                    <h2 class={styles.title}>MARKET OVERVIEW</h2>
                    <UtcClockBadge />
                </div>
                <ScanStatusStrip />
            </div>

            <RecommendationHero />

            <HeaderKpiStrip />

            <div class={styles.grid5}>
                <TradeOpportunitiesCard />
                <RiskDistributionCard />
                <SignalQualityCard />
                <DirectionDistributionCard />
                <MarketAlignmentCard />
            </div>

            <MarketHealthCard />

            <RegimeDistributionCard />

            <AssetRankingsTable />
        {/if}

        <div class={styles.runnerBar}>
            <div class={styles.actions}>
                <SnapshotSchedulerButton />
                <WatchlistRunnerButton {wssMap} />
            </div>
        </div>
    </div>
</div>
