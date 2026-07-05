<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import type { CurrentView, Level2Mode, OperationalMode, InstanceState } from './types';

    import LiveTerminal from './components/LiveTerminal.svelte';
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
    import TimeframeSettings from './components/TimeframeSettings.svelte';
    import WelcomeGate from './WelcomeGate.svelte';
    import QuitDialog from './QuitDialog.svelte';
    import EdgeBuilder from './components/EdgeBuilder.svelte';
    import EdgeAnalyzer from './components/EdgeAnalyzer.svelte';
    import styles from './App.module.css';

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
    let showQuitDialog = $state(false);
    let showProfileMenu = $state(false);
    let applyBusy = $state(false);

    // ─── 3-Tier navigation config ───────────────────────────────────────────
    const MODE_DEFS: { key: Level2Mode; label: string }[] = [
        { key: 'general', label: 'GENERAL' },
        { key: 'user', label: 'USER-CONTROLLED' },
        { key: 'rule', label: 'RULE-BASED' },
        { key: 'ai', label: 'AI-DRIVEN' },
    ];

    const MODE_TABS: Record<Level2Mode, { view: CurrentView; label: string }[]> = {
        general: [
            { view: 'timeframe_settings', label: '🕐 Timeframe Settings' },
            { view: 'settings', label: '⚙️ Workspace Settings' },
            { view: 'risk', label: '🛡️ Risk Management' },
        ],
        user: [
            { view: 'terminal', label: '📈 Live Terminal' },
            { view: 'positions', label: '💰 Positions' },
            { view: 'commission', label: '💸 Fee Projection' },
            { view: 'costs', label: '💰 Token Costs' },
        ],
        rule: [
            { view: 'decision', label: '🎯 Decision Trading' },
            { view: 'edge_builder', label: '🔧 Edge Builder' },
            { view: 'edge_analyzer', label: '📐 Edge Analyzer' },
        ],
        ai: [
            { view: 'assistant', label: '🤖 AI Assistant' },
            { view: 'observability', label: '🎯 DECISION HUD' },
            { view: 'performance', label: '📊 Performance Metrics' },
            { view: 'analytics', label: '📊 Trade Audit' },
            { view: 'ledger', label: '📋 Trade Ledger' },
            { view: 'edge_builder', label: '🔧 Edge Builder' },
            { view: 'edge_analyzer', label: '📐 Edge Analyzer' },
        ],
    };

    const MODE_TO_OP: Record<Level2Mode, OperationalMode | null> = {
        general: null, user: 'ManualOnly', rule: 'DeterministicHeuristics', ai: 'HybridAiCopilot',
    };

    function execLabel(mode: OperationalMode): string {
        if (mode === 'ManualOnly') return 'User-Controlled';
        if (mode === 'DeterministicHeuristics') return 'Rule-Based';
        return 'AI-Driven';
    }

    function selectView(pair: InstanceState, view: CurrentView) {
        pair.currentView = view;
        pair.modeViews[pair.currentLevel2Mode] = view;
        if (view === 'positions') app.fetchPaperStatus();
        else if (view === 'costs') app.fetchCostEstimate();
    }

    function selectMode(pair: InstanceState, mode: Level2Mode) {
        pair.currentLevel2Mode = mode;
        selectView(pair, pair.modeViews[mode]);
    }

    async function applyMode() {
        applyBusy = true;
        await app.applyMode();
        applyBusy = false;
    }

    // ─── Config & lifecycle ─────────────────────────────────────────────
    let configReady = false;

    async function fetchConfig() {
        try {
            const config = await fetchConfigFromServer();
            const { firstSymbol } = applyConfigToStore(app, config);
            if (firstSymbol) app.activeTab = `${firstSymbol}-USDT`;
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
    <!-- Global Top Navbar -->
    <nav class={styles.globalNavbar}>
        <div class={styles.navbarBrand}>
            <span class={styles.navbarLogo}>AI Trading Assistant</span>
            <span class={styles.navbarSessionBadge}>{app.sessionMode?.toUpperCase()} — {app.sessionCurrency} on {app.sessionExchange}</span>
        </div>
        <div class={styles.navbarTabs}>
            <button class={styles.navbarTab} class:active={app.currentGlobalView === 'dashboard'} onclick={() => { app.currentGlobalView = 'dashboard'; }}>
                <span>📊</span> Dashboard
            </button>
            <button class={styles.navbarTab} class:active={app.currentGlobalView === 'instances'} onclick={() => { app.currentGlobalView = 'instances'; }}>
                <span>📋</span> Instances
            </button>
            <button class={styles.navbarTab} class:active={app.currentGlobalView === 'settings'} onclick={() => { app.currentGlobalView = 'settings'; }}>
                <span>⚙️</span> Settings
            </button>
        </div>
        <div class={styles.navbarActions}>
            <div class={styles.profileMenuWrapper}>
                <button class={styles.navbarProfileBtn} onclick={() => showProfileMenu = !showProfileMenu} title="Profile">
                    <span>👤</span>
                </button>
                {#if showProfileMenu}
                    <div class={styles.profileDropdown} role="menu">
                        <div class={styles.profileDropdownHeader}>
                            <span class={styles.profileCapital}>{app.sessionCurrency} {app.sessionCapital?.toLocaleString() || '0'}</span>
                            <span class={styles.profileMode}>{app.sessionMode} Trading</span>
                        </div>
                        <div class={styles.profileDropdownDivider}></div>
                        <button class={styles.profileDropdownItem} onclick={() => { showProfileMenu = false; app.currentGlobalView = 'dashboard'; }}>
                            📊 General Dashboard
                        </button>
                        <button class={styles.profileDropdownItem} onclick={() => { showProfileMenu = false; app.currentGlobalView = 'instances'; }}>
                            📋 All Instances
                        </button>
                        <button class={styles.profileDropdownItem} onclick={() => { showProfileMenu = false; app.currentGlobalView = 'settings'; }}>
                            ⚙️ Settings
                        </button>
                        <div class={styles.profileDropdownDivider}></div>
                        <button class={styles.profileDropdownItem + " " + styles.danger} onclick={() => { showProfileMenu = false; showQuitDialog = true; }}>
                            🚪 Quit
                        </button>
                    </div>
                {/if}
            </div>
        </div>
    </nav>
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
    {:else}
    {#if !app.apiKeyConfigured}
        <div class={styles.apiKeyBanner}>
            ⚠️ DeepSeek AI API Key is not configured. Falling back to local heuristic mode.
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
                                class={styles.modeBtn}
                                class:mode-active={pair.currentLevel2Mode === mode.key}
                                onclick={() => selectMode(pair, mode.key)}
                            >
                                {mode.label}
                            </button>
                        {/each}
                    </div>
                    <div class={styles.modeStatusGroup}>
                        <span class={styles.execBadge}>ACTIVE: {execLabel(pair.activeExecutionMode)}</span>
                        {#if MODE_TO_OP[pair.currentLevel2Mode]}
                            <button
                                class={styles.applyConfigBtn}
                                disabled={applyBusy || MODE_TO_OP[pair.currentLevel2Mode] === pair.activeExecutionMode}
                                onclick={applyMode}
                            >
                                {applyBusy ? 'Applying…' : 'Apply Workspace Configuration'}
                            </button>
                        {/if}
                    </div>
                </div>

                <!-- Level 3: Feature Panel navbar (subset for the active mode) -->
                <div class={styles.workspaceSubHeader}>
                    <div class={styles.subTabsContainer}>
                        {#each MODE_TABS[pair.currentLevel2Mode] as tab (tab.view)}
                            <button
                                class={styles.subTabBtn}
                                class:sub-tab-active={pair.currentView === tab.view}
                                onclick={() => selectView(pair, tab.view)}
                            >
                                {tab.label}
                            </button>
                        {/each}
                    </div>
                </div>

                <div class={styles.instancePairBanner}>
                    <span class={styles.pairBannerTitle}>{pair.symbol} / USDT</span>
                </div>

                <!-- 1. Live Terminal Inner View -->
                {#if pair.currentView === 'terminal'}
                    <div class={styles.mainLayout + " " + 'animate-fade'}>
                        <LiveTerminal pairKey={tabKey} />
                    </div>

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

{#if showQuitDialog}
    <QuitDialog onclose={() => showQuitDialog = false} />
{/if}
{/if}

