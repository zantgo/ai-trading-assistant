<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import type { CurrentView, Level2Mode, InstanceState } from './types';

    import LiveTerminal from './components/LiveTerminal.svelte';
    import StatePanel from './components/StatePanel.svelte';
    import PerformanceDashboard from './components/PerformanceDashboard.svelte';
    import DecisionTrading from './components/DecisionTrading.svelte';
    import RiskCalculator from './components/RiskCalculator.svelte';
    import AnalyticsDashboard from './components/AnalyticsDashboard.svelte';
    import TradeListLedger from './components/TradeListLedger.svelte';
    import CommissionCalculator from './components/CommissionCalculator.svelte';
    import ObservabilityHub from './components/ObservabilityHub.svelte';
    import InstanceList from './components/InstanceList.svelte';
    import GeneralDashboard from './components/GeneralDashboard.svelte';
    import GeneralSettings from './components/GeneralSettings.svelte';
    import WorkspaceSettings from './components/WorkspaceSettings.svelte';
    import CopilotModal from './components/CopilotModal.svelte';
    import AiAssistantPanel from './components/AiAssistantPanel.svelte';
    import PaperTradingPanel from './components/PaperTradingPanel.svelte';
    import CostDashboardPanel from './components/CostDashboardPanel.svelte';
    import RiskProfilePanel from './components/RiskProfilePanel.svelte';
    import TimeframeSettings from './components/TimeframeSettings.svelte';
    import ExchangeSettings from './components/ExchangeSettings.svelte';
    import MonitoringPanel from './components/MonitoringPanel.svelte';
    import RiskManagementPanel from './components/RiskManagementPanel.svelte';
    import WelcomeGate from './WelcomeGate.svelte';
    import QuitDialog from './QuitDialog.svelte';
    import UserProfile from './components/UserProfile.svelte';
    import EdgeBuilder from './components/EdgeBuilder.svelte';
    import EdgeAnalyzer from './components/EdgeAnalyzer.svelte';
    import StatisticalPanel from './components/StatisticalPanel.svelte';
    import TradingWizard from './components/TradingWizard.svelte';
    import TradingWorkflow from './components/TradingWorkflow.svelte';
    import RiskOverview from './components/RiskOverview.svelte';
    import styles from './App.module.css';
    import Icon from './lib/Icon.svelte';
    import type { IconName } from './lib/icons';

    // ─── Lib imports ─────────────────────────────────────────────────────────
    import { fetchConfigFromServer, applyConfigToStore } from './lib/api.svelte';
    import {
        createWsState, disconnectAllWs,
        connectWebsocket as connectWs, shouldReconnect,
        type WsState,
    } from './lib/websocket.svelte';
    import { fetchAssistantHistory } from './lib/analysis.svelte';

    // ─── App store & component-local state ──────────────────────────────────
    const app = useAppStore();
    const wsState: WsState = createWsState();
    let showProfileMenu = $state(false);

    // ─── 3-Tier navigation config ───────────────────────────────────────────
    const MODE_DEFS: { key: Level2Mode; label: string }[] = [
        { key: 'general', label: 'GENERAL' },
        { key: 'wizard', label: 'TRADING WORKFLOW' },
        { key: 'risk', label: 'RISK' },
        { key: 'user', label: 'USER-CONTROLLED' },
        { key: 'rule', label: 'RULE-BASED' },
        { key: 'ai', label: 'AI-DRIVEN' },
    ];

    const MODE_TABS: Record<Level2Mode, { view: CurrentView; label: string; icon: IconName }[]> = {
        general: [
            { view: 'terminal', label: 'Live Workspace', icon: 'trending-up' },
            { view: 'timeframe_settings', label: 'Timeframe Settings', icon: 'clock' },
            { view: 'commission', label: 'Fee Projection', icon: 'percent' },
            { view: 'costs', label: 'Token Costs', icon: 'dollar' },
            { view: 'settings', label: 'Workspace Settings', icon: 'monitor' },
        ],
        wizard: [
            { view: 'workflow', label: 'Workflow', icon: 'target' },
        ],
        risk: [
            { view: 'risk_overview', label: 'Overview', icon: 'shield' },
            { view: 'risk_profile', label: 'Risk Profile', icon: 'shield' },
            { view: 'risk_management', label: 'Risk Manager', icon: 'shield' },
            { view: 'commission', label: 'Fees', icon: 'percent' },
        ],
        user: [
            { view: 'monitor', label: 'State Panel', icon: 'monitor' },
            { view: 'monitoring', label: 'Monitoring', icon: 'eye' },
            { view: 'positions', label: 'Positions', icon: 'dollar' },
            { view: 'risk_management', label: 'Risk Management', icon: 'shield' },
        ],
        rule: [
            { view: 'decision', label: 'Decision Trading', icon: 'target' },
            { view: 'edge_builder', label: 'Edge Builder', icon: 'tool' },
            { view: 'edge_analyzer', label: 'Edge Analyzer', icon: 'compass' },
        ],
        ai: [
            { view: 'assistant', label: 'AI Assistant', icon: 'bot' },
            { view: 'statistics', label: 'Statistical Intel', icon: 'bar-chart' },
            { view: 'monitoring', label: 'Monitoring', icon: 'eye' },
            { view: 'observability', label: 'Decision HUD', icon: 'target' },
            { view: 'performance', label: 'Performance Metrics', icon: 'bar-chart' },
            { view: 'analytics', label: 'Trade Audit', icon: 'bar-chart' },
            { view: 'ledger', label: 'Trade Ledger', icon: 'book' },
            { view: 'edge_builder', label: 'Edge Builder', icon: 'tool' },
            { view: 'edge_analyzer', label: 'Edge Analyzer', icon: 'compass' },
        ],
    };

    function selectView(pair: InstanceState, view: CurrentView) {
        pair.currentView = view;
        pair.modeViews[pair.currentLevel2Mode] = view;
        if (view === 'positions') app.fetchPaperStatus();
        else if (view === 'costs') app.fetchCostEstimate();
        else if (view === 'risk_profile') app.fetchRiskProfile();
        else if (view === 'monitoring') app.fetchActiveTrades();
        else if (view === 'risk_management') app.fetchRiskProfile();
    }

    function selectMode(pair: InstanceState, mode: Level2Mode) {
        pair.currentLevel2Mode = mode;
        selectView(pair, pair.modeViews[mode]);
    }

    // ─── Config & lifecycle ─────────────────────────────────────────────
    let configReady = false;

    async function fetchConfig() {
        try {
            const config = await fetchConfigFromServer();
            const { firstSymbol } = applyConfigToStore(app, config);
            if (firstSymbol) app.activeTab = app.pairKeyFor(firstSymbol);
            configReady = true;
            connectWs(app, wsState);
        } catch (e) {
            console.error('Failed to fetch config from server:', e);
            configReady = true;
        }
    }

    onMount(async () => {
        app.fetchSessionStatus();
        await fetchConfig();
        await fetchAssistantHistory(app);
    });

    onDestroy(() => {
        disconnectAllWs(wsState);
    });

    $effect(() => {
        const tab = app.activeTab;
        if (configReady && tab && shouldReconnect(app, wsState)) {
            connectWs(app, wsState);
        }
    });
</script>

{#if !app.sessionChecked}
    <div class={styles.sessionLoading}>
        <div class={styles.loadingSpinner}></div>
        <p>Connecting to AI Trading Assistant...</p>
    </div>
{:else if !app.sessionActive}
    <WelcomeGate />
{:else}
<div class={styles.terminalBody}>
    <!-- Unified Global Header -->
    <div class={styles.navHeaderStack}>
        <div class={styles.globalNavCard}>
            <!-- Left: brand + session badges -->
            <div class={styles.brandLeftGroup}>
                <span class={styles.brandHeaderTitle}>AI TRADING ASSISTANT</span>
                <span class={styles.navbarPillBadge}>
                    <span class={styles.exchangeBadgeCurrency}>{app.sessionCurrency}</span>
                    <span class={styles.exchangeBadgeOn}> on </span>
                    <span class={styles.exchangeBadgeExchange}>{app.sessionExchange}</span>
                </span>
                <span class="{styles.navbarPillBadge} {app.sessionMode === 'live' ? styles.navbarPillBadgeActiveLive : styles.navbarPillBadgeActivePaper}">{app.sessionMode?.toUpperCase()}</span>
            </div>
            <!-- Right: view switcher + profile -->
            <div class={styles.navRightGroup}>
                <div class={styles.navbarTabs}>
                    <button class={styles.navbarTab} class:active={app.currentGlobalView === 'dashboard'} onclick={() => { app.currentGlobalView = 'dashboard'; }}>
                        <Icon name="dashboard" size={14} /> Dashboard
                    </button>
                    <button class={styles.navbarTab} class:active={app.currentGlobalView === 'instances' || app.currentGlobalView === 'workspace'} onclick={() => { app.currentGlobalView = 'instances'; }}>
                        <Icon name="instances" size={14} /> Instances
                    </button>
                    <button class={styles.navbarTab} class:active={app.currentGlobalView === 'settings'} onclick={() => { app.currentGlobalView = 'settings'; }}>
                        <Icon name="settings" size={14} /> Settings
                    </button>
                </div>
                <div class={styles.profileMenuWrapper}>
                    <button class="{styles.navbarTab} {styles.navbarProfileTab}" class:active={showProfileMenu || app.currentGlobalView === 'user_profile'} onclick={() => showProfileMenu = !showProfileMenu} title="Profile">
                        <Icon name="user" size={16} /> {app.sessionUserName || 'Profile'}
                    </button>
                    {#if showProfileMenu}
                        <div class={styles.profileDropdown} role="menu">
                            <div class={styles.profileDropdownUser}>
                                <span class={styles.profileDropdownName}>{app.sessionUserName || 'Trader'}</span>
                            </div>
                            <div class={styles.profileDropdownDivider}></div>
                            <div class={styles.profileDropdownInfo}>
                                <div class={styles.profileDropdownInfoRow}>
                                    <span class={styles.profileDropdownInfoLabel}>Exchange</span>
                                    <span class={styles.profileDropdownInfoValue}>{app.sessionExchange}</span>
                                </div>
                                <div class={styles.profileDropdownInfoRow}>
                                    <span class={styles.profileDropdownInfoLabel}>Portfolio</span>
                                    <span class={styles.profileDropdownInfoValue}>{app.sessionCurrency} {app.sessionCapital?.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) || '0.00'}</span>
                                </div>
                                <div class={styles.profileDropdownInfoRow}>
                                    <span class={styles.profileDropdownInfoLabel}>Per Instance</span>
                                    <span class={styles.profileDropdownInfoValue}>{app.sessionCurrency} {app.perInstanceCapital.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
                                </div>
                            </div>
                            <div class={styles.profileDropdownDivider}></div>
                            <button class={styles.profileDropdownItem} onclick={() => { showProfileMenu = false; app.currentGlobalView = 'user_profile'; }}>
                                <Icon name="user" /> Profile
                            </button>
                            <div class={styles.profileDropdownDivider}></div>
                            <button class={styles.profileDropdownItem + " " + styles.danger} onclick={() => { showProfileMenu = false; app.showQuitDialog = true; }}>
                                <Icon name="quit" /> Quit
                            </button>
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    </div>
    <!-- Click-outside handler for profile menu -->
    {#if showProfileMenu}
        <div class={styles.profileBackdrop} role="presentation" onclick={() => showProfileMenu = false} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') showProfileMenu = false; }}></div>
    {/if}

    {#if app.currentGlobalView === 'dashboard'}
        <GeneralDashboard />
    {:else if app.currentGlobalView === 'instances'}
        <InstanceList />
    {:else if app.currentGlobalView === 'settings'}
        <GeneralSettings />
    {:else if app.currentGlobalView === 'user_profile'}
        <UserProfile />
    {:else}
    {#if !app.apiKeyConfigured}
        <div class={styles.apiKeyBanner}>
            <Icon name="alert" size={14} /> DeepSeek AI API Key is not configured. Falling back to local heuristic mode.
        </div>
    {/if}

    <div class={styles.appLayout}>
        <div class={styles.workspaceViewport}>
        {#each Object.keys(app.instancesMap) as tabKey (tabKey)}
            {@const pair = app.instancesMap[tabKey]}
            <div class="{styles.workspaceWindow} {tabKey !== app.activeTab ? styles.hiddenPane : ''}">

                <!-- Level 2: Operational Mode navbar -->
                <div class={styles.modeNavbar}>
                    <div class={styles.modeTabsContainer}>
                        {#each MODE_DEFS as mode (mode.key)}
                            <button
                                class="{styles.modeBtn} {pair.currentLevel2Mode === mode.key ? styles.modeActive : ''}"
                                onclick={() => selectMode(pair, mode.key)}
                            >
                                {mode.label}
                            </button>
                        {/each}
                    </div>
                </div>

                <!-- Level 3: Feature Panel navbar (subset for the active mode) -->
                <div class={styles.workspaceSubHeader}>
                    <div class={styles.subTabsContainer}>
                         {#each MODE_TABS[pair.currentLevel2Mode] as tab (tab.view)}
                            <button
                                class="{styles.subTabBtn} {pair.currentView === tab.view ? styles.subTabActive : ''}"
                                onclick={() => selectView(pair, tab.view)}
                            >
                                <Icon name={tab.icon} size={14} /> {tab.label}
                            </button>
                        {/each}
                    </div>
                </div>

                <div class={styles.instancePairBanner}>
                    <span class={styles.pairBannerTitle}>{pair.symbol} / {app.quote}</span>
                    <div class={styles.pairBannerPriceRow}>
                        <span class={styles.pairBannerPrice}>
                            {app.priceText !== '--' ? '$' + app.priceText : '--'}
                        </span>
                        {#if app.dayChangePct !== null}
                            <span class="{styles.pairBannerChange} {app.dayChangePct >= 0 ? styles.up : styles.down}">
                                {app.dayChangePct >= 0 ? '+' : ''}{app.dayChangePct.toFixed(2)}%
                            </span>
                        {/if}
                    </div>
                </div>

                <!-- 1. Live Terminal Inner View -->
                {#if pair.currentView === 'terminal'}
                    <div class={styles.mainLayout + " " + 'animate-fade'}>
                        <LiveTerminal pairKey={tabKey} />
                    </div>

                <!-- 1.2 State Panel (decision cockpit) View -->
                {:else if pair.currentView === 'monitor'}
                    <StatePanel pairKey={tabKey} />

                <!-- 1.3 Trading Workflow -->
                {:else if pair.currentView === 'workflow'}
                    <TradingWorkflow pairKey={tabKey} />

                <!-- 1.4 Trading Wizard (legacy) -->
                {:else if pair.currentView === 'wizard_flow'}
                    <TradingWizard />

                <!-- 1.5 AI Assistant View -->
                {:else if pair.currentView === 'assistant'}
                    <AiAssistantPanel />

                <!-- 1.6 Positions Inner View -->
                {:else if pair.currentView === 'positions'}
                    <PaperTradingPanel />

                <!-- 2. Performance Metrics Inner View -->
                {:else if pair.currentView === 'performance'}
                    <div class={styles.workspaceInnerContent}>
                        <PerformanceDashboard />
                    </div>

                <!-- 3. Local Workspace Settings Tab View -->
                {:else if pair.currentView === 'settings'}
                    <WorkspaceSettings {pair} {tabKey} />

                <!-- 4. Decision Trading View -->
                {:else if pair.currentView === 'decision'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <DecisionTrading />
                    </div>

                <!-- 5. Risk Calculator View -->
                {:else if pair.currentView === 'risk'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <RiskCalculator />
                    </div>

                <!-- 5b. Commission Fee Projection View -->
                {:else if pair.currentView === 'commission'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <CommissionCalculator />
                    </div>

                <!-- 7. Analytics Dashboard View -->
                {:else if pair.currentView === 'analytics'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <AnalyticsDashboard />
                    </div>

                <!-- 8. Trade Ledger View -->
                {:else if pair.currentView === 'ledger'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <TradeListLedger />
                    </div>

                <!-- 9. Token Cost Dashboard -->
                {:else if pair.currentView === 'costs'}
                    <CostDashboardPanel {pair} />

                <!-- 9a. Risk Overview -->
                {:else if pair.currentView === 'risk_overview'}
                    <RiskOverview />

                <!-- 9b. Institutional Risk Management Layer -->
                {:else if pair.currentView === 'risk_profile'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <RiskProfilePanel {pair} />
                    </div>

                <!-- 9d. Monitoring Panel -->
                {:else if pair.currentView === 'monitoring'}
                    <MonitoringPanel pairKey={tabKey} />

                <!-- 9e. Unified Risk Management Panel -->
                {:else if pair.currentView === 'risk_management'}
                    <RiskManagementPanel pairKey={tabKey} />

                <!-- 9b. Edge Builder -->
                {:else if pair.currentView === 'edge_builder'}
                    <EdgeBuilder paradigm={pair.currentLevel2Mode === 'ai' ? 'ai' : 'rule'} />

                <!-- 9c. Edge Analyzer -->
                {:else if pair.currentView === 'edge_analyzer'}
                    <EdgeAnalyzer paradigm={pair.currentLevel2Mode === 'ai' ? 'ai' : 'rule'} />

                {:else if pair.currentView === 'observability'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <ObservabilityHub />
                    </div>
                {:else if pair.currentView === 'statistics'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <StatisticalPanel />
                    </div>
                {:else if pair.currentView === 'exchange'}
                    <ExchangeSettings />
                {:else if pair.currentView === 'timeframe_settings'}
                    <TimeframeSettings {pair} {tabKey} onApplied={() => connectWs(app, wsState)} />
                {/if}

            </div>
        {/each}
    </div>
</div>

    <!-- Modals -->
    <CopilotModal />
{/if}
</div>

{#if app.showQuitDialog}
    <QuitDialog onclose={() => app.showQuitDialog = false} />
{/if}
{/if}

