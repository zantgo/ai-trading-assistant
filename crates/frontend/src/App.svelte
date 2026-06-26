<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    // types inferred from store, no explicit type imports needed

    import LiveTerminal from './components/LiveTerminal.svelte';
    import PerformanceDashboard from './components/PerformanceDashboard.svelte';
    import DecisionTrading from './components/DecisionTrading.svelte';
    import RiskCalculator from './components/RiskCalculator.svelte';
    import ExchangeSettings from './components/ExchangeSettings.svelte';
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

                <!-- Secondary navigation bar within each pair's self-contained layout -->
                <div class={styles.workspaceSubHeader}>
                    <div class={styles.subTabsContainer}>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'terminal'}
                            onclick={() => pair.currentView = 'terminal'}
                        >
                            📈 Live Terminal
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'assistant'}
                            onclick={() => pair.currentView = 'assistant'}
                        >
                            🤖 AI Assistant
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'positions'}
                            onclick={() => { pair.currentView = 'positions'; app.fetchPaperStatus(); }}
                        >
                            💰 Positions
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'performance'}
                            onclick={() => pair.currentView = 'performance'}
                        >
                            📊 Performance Metrics
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'settings'}
                            onclick={() => { pair.currentView = 'settings'; }}
                        >
                            ⚙️ Workspace Settings
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'decision'}
                            onclick={() => { pair.currentView = 'decision'; }}
                        >
                            🎯 Decision Trading
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'risk'}
                            onclick={() => { pair.currentView = 'risk'; }}
                        >
                            🛡️ Risk Management
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'commission'}
                            onclick={() => { pair.currentView = 'commission'; }}
                        >
                            💸 Fee Projection
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'exchange'}
                            onclick={() => { pair.currentView = 'exchange'; }}
                        >
                            🔐 Exchange Settings
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'analytics'}
                            onclick={() => { pair.currentView = 'analytics'; }}
                        >
                            📊 Trade Audit
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'ledger'}
                            onclick={() => { pair.currentView = 'ledger'; }}
                        >
                            📋 Trade Ledger
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'costs'}
                            onclick={() => { pair.currentView = 'costs'; app.fetchCostEstimate(); }}
                        >
                            💰 Token Costs
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'timeframe_settings'}
                            onclick={() => { pair.currentView = 'timeframe_settings'; }}
                        >
                            🕐 Timeframe Settings
                        </button>
                        <button
                            class={styles.subTabBtn}
                            class:sub-tab-active={pair.currentView === 'observability'}
                            onclick={() => { pair.currentView = 'observability'; }}
                        >
                            🎯 DECISION HUD
                        </button>
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

                <!-- 6. Exchange Settings View -->
                {:else if pair.currentView === 'exchange'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <ExchangeSettings />
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
                {:else if pair.currentView === 'observability'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <ObservabilityHub />
                    </div>
                {:else if pair.currentView === 'timeframe_settings'}
                    <TimeframeSettings {pair} {tabKey} />
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

