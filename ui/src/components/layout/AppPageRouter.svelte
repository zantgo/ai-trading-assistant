<script lang="ts">
    import type { CurrentView } from '../../types';
    import type { InstanceState } from '../../types';
    import type { WsState } from '../../lib/websocket.svelte';
    import styles from '../../styles/brutalist-grid.module.css';

    import LiveTerminal from '../LiveTerminal.svelte';
    import TerminalMonitor from '../TerminalMonitor.svelte';
    import AlignmentPanel from '../AlignmentPanel.svelte';
    import OpportunitiesPanel from '../OpportunitiesPanel.svelte';
    import RiskPanel from '../RiskPanel.svelte';
    import AnalysisPanel from '../AnalysisPanel.svelte';
    import AdvisoryPanel from '../AdvisoryPanel.svelte';
    import GeneralDashboard from '../GeneralDashboard.svelte';
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
    }

    let { currentEngine, middleTab, selectedInstance, activePair, activeTab, wssMap }: Props = $props();
</script>

<main class={styles.contentArea}>
    {#if currentEngine === 'profile'}
        <GeneralSettings />
    {:else if currentEngine === 'data_infra'}
        {#if middleTab === 'overview'}
            <DataInfraDashboard />
        {:else}
            <div class={styles.profileCard} style="padding:2rem">
                <h3>Data Infrastructure Settings</h3>
                <p class={styles.cardSub}>Exchange endpoints and NTP clock monitor configuration.</p>
                <p class={styles.cardSub}>Edit <code>config.toml</code> → <code>[hyperliquid]</code>, <code>[bitget]</code>, <code>[clock_monitor]</code> sections directly. Restart the daemon after changes.</p>
            </div>
        {/if}
    {:else if currentEngine === 'market_monitor'}
        {#if middleTab === 'workspace'}
            {#if selectedInstance && activePair}
                {#if activePair.currentView === 'terminal'}
                    <LiveTerminal pairKey={activeTab} />
                {:else if activePair.currentView === 'monitor'}
                    <TerminalMonitor pairKey={activeTab} />
                {:else if activePair.currentView === 'alignment'}
                    <AlignmentPanel pairKey={activeTab} />
                {:else if activePair.currentView === 'opportunity'}
                    <OpportunitiesPanel pairKey={activeTab} />
                {:else if activePair.currentView === 'risk'}
                    <RiskPanel pairKey={activeTab} />
                {:else if activePair.currentView === 'analysis'}
                    <AnalysisPanel />
                {:else if activePair.currentView === 'advisory'}
                    <AdvisoryPanel pairKey={activeTab} />
                {/if}
            {:else}
                <GeneralDashboard {wssMap} />
            {/if}
        {:else if middleTab === 'overview'}
            <GeneralDashboard {wssMap} />
        {:else}
            {#if activePair}
                <WorkspaceSettings pair={activePair} tabKey={activeTab} />
            {/if}
        {/if}
    {:else if currentEngine === 'performance'}
        {#if middleTab === 'overview'}
            <PerformanceDashboard />
        {:else}
            <div class={styles.profileCard} style="padding:2rem">
                <h3>Performance Analytics Settings</h3>
                <p class={styles.cardSub}>Configure analytics execution cadences and optimizer intervals in <code>config.toml</code> → <code>[workspace]</code> → <code>eval_interval_secs</code> and <code>optimizer_interval_secs</code>.</p>
            </div>
        {/if}
    {:else if currentEngine === 'trade_automation'}
        {#if middleTab === 'overview'}
            <TradeAutomationDashboard />
        {:else}
            <div class={styles.profileCard} style="padding:2rem">
                <h3>Trade Automation Settings</h3>
                <p class={styles.cardSub}>Configure execution policies, trigger modes, risk parameters, and paper/live trading adapter settings in <code>config.toml</code> → <code>[execution_engine]</code>. Edit policy files in <code>config/policies/</code>.</p>
            </div>
        {/if}
    {:else if currentEngine === 'portfolio'}
        {#if middleTab === 'overview'}
            <PortfolioDashboard />
        {:else}
            <div class={styles.profileCard} style="padding:2rem">
                <h3>Portfolio Management Settings</h3>
                <p class={styles.cardSub}>Configure safety thresholds, fee rates, leverage caps, concentration limits, and drawdown enforcement in <code>config.toml</code> → <code>[portfolio]</code>. Edit risk profiles in <code>config/</code>.</p>
            </div>
        {/if}
    {:else if currentEngine === 'exchange_settings'}
        <GeneralSettings />
    {/if}
</main>
