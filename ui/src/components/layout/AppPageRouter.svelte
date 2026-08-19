<script lang="ts">
    import type { CurrentView } from '../../types';
    import type { InstanceState } from '../../types';
    import type { WsState } from '../../lib/websocket.svelte';
    import { resolveEngineTab, type EngineKey } from '../../lib/engineTabs';
    import styles from '../../styles/brutalist-grid.module.css';

    import LiveTerminal from '../LiveTerminal.svelte';
    import TerminalMonitor from '../TerminalMonitor.svelte';
    import AlignmentPanel from '../AlignmentPanel.svelte';
    import OpportunitiesPanel from '../OpportunitiesPanel.svelte';
    import RiskPanel from '../RiskPanel.svelte';
    import AnalysisPanel from '../AnalysisPanel.svelte';
    import RecommendationPanel from '../RecommendationPanel.svelte';
    import GeneralDashboard from '../GeneralDashboard.svelte';
    import InstancePicker from '../InstancePicker.svelte';
    import GeneralSettings from '../GeneralSettings.svelte';
    import DataInfraDashboard from '../DataInfraDashboard.svelte';
    import PerformanceDashboard from '../PerformanceDashboard.svelte';
    import TradeAutomationDashboard from '../TradeAutomationDashboard.svelte';
    import PortfolioDashboard from '../PortfolioDashboard.svelte';
    import WorkspaceSettings from '../WorkspaceSettings.svelte';

    interface Props {
        currentEngine: string;
        middleTab: string;
        selectedInstance: string | null;
        activePair: InstanceState | undefined;
        activeTab: string;
        wssMap: Record<string, WsState>;
        /** Delegated to InstancePicker so delete flows through the same
         *  App-level confirm modal + `executeDelete` path as the right
         *  Instances panel. */
        onrequestConfirm: (id: string, action: 'delete', pair?: string) => void;
        errorMessage: string | null;
    }

    let { currentEngine, middleTab, selectedInstance, activePair, activeTab, wssMap, onrequestConfirm, errorMessage }: Props = $props();

    // Diagnostic: uncomment to confirm props remain reactive after the fix
    // $inspect('router.middleTab', middleTab);
    // $inspect('router.selectedInstance', selectedInstance);
    // $inspect('router.activePair', activePair);

    // Per-symbol WS state, derived once per render. The instance tabs
    // (`TerminalMonitor`, `AlignmentPanel`, `OpportunitiesPanel`,
    // `RiskPanel`, `AnalysisPanel`, `RecommendationPanel`) all read this
    // to feed the `LayerHeader` status pill (live / stale / error).
    const activeWss = $derived<WsState | undefined>(wssMap[activeTab]);

    // Section for the section-driven engine dashboards. Stale or legacy
    // middleTab values (e.g. `#/engine/data_infra/overview`) resolve to
    // the engine's default tab so the navbar always has an active item.
    const section = $derived(resolveEngineTab(currentEngine as EngineKey, middleTab));
</script>

<main class={styles.contentArea}>
    {#if currentEngine === 'profile' || currentEngine === 'exchange_settings'}
        <GeneralSettings />
    {:else if currentEngine === 'data_infra'}
        <DataInfraDashboard section={section} />
    {:else if currentEngine === 'market_monitor'}
        {#if middleTab === 'workspace'}
            {#if selectedInstance && activePair}
                {#if activePair.currentView === 'terminal'}
                    <LiveTerminal pairKey={activeTab} />
                {:else if activePair.currentView === 'monitor'}
                    <TerminalMonitor pairKey={activeTab} wssState={activeWss} />
                {:else if activePair.currentView === 'alignment'}
                    <AlignmentPanel pairKey={activeTab} wssState={activeWss} />
                {:else if activePair.currentView === 'opportunity'}
                    <OpportunitiesPanel pairKey={activeTab} wssState={activeWss} />
                {:else if activePair.currentView === 'risk'}
                    <RiskPanel pairKey={activeTab} wssState={activeWss} />
                {:else if activePair.currentView === 'analysis'}
                    <AnalysisPanel wssState={activeWss} />
                {:else if activePair.currentView === 'recommendation'}
                    <RecommendationPanel pairKey={activeTab} wssState={activeWss} />
                {/if}
            {:else}
                <InstancePicker {onrequestConfirm} {errorMessage} />
            {/if}
        {:else if middleTab === 'overview'}
            <GeneralDashboard {wssMap} />
        {:else}
            {#if activePair}
                <WorkspaceSettings pair={activePair} tabKey={activeTab} />
            {:else}
                <div class={styles.profileCard} style="padding:2rem">
                    <h3>Settings</h3>
                    <p class={styles.cardSub}>Select a workspace instance from the top-right panel to configure timeframes, indicators, and visual overlays.</p>
                </div>
            {/if}
        {/if}
    {:else if currentEngine === 'performance'}
        <PerformanceDashboard section={section} />
    {:else if currentEngine === 'trade_automation'}
        <TradeAutomationDashboard section={section} />
    {:else if currentEngine === 'portfolio'}
        <PortfolioDashboard section={section} />
    {/if}
</main>
