<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import type { CurrentView, InstanceState } from './types';

    import LiveTerminal from './components/LiveTerminal.svelte';
    import TerminalMonitor from './components/TerminalMonitor.svelte';
    import DecisionTrading from './components/DecisionTrading.svelte';
    import CommissionCalculator from './components/CommissionCalculator.svelte';
    import TimeframeSettings from './components/TimeframeSettings.svelte';
    import GeneralDashboard from './components/GeneralDashboard.svelte';
    import GeneralSettings from './components/GeneralSettings.svelte';
    import WorkspaceSettings from './components/WorkspaceSettings.svelte';
    import InstanceList from './components/InstanceList.svelte';
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

    // ─── App store & component-local state ──────────────────────────────────
    const app = useAppStore();
    const wsState: WsState = createWsState();
    let showQuitDialog = $state(false);
    let showProfileMenu = $state(false);

    // ─── Flat sub-tab config ────────────────────────────────────────────────
    const SUB_TABS: { view: CurrentView; label: string }[] = [
        { view: 'terminal', label: '📈 Live Terminal' },
        { view: 'monitor', label: '🖥️ Terminal Monitor' },
        { view: 'decision', label: '🎯 Decision Trading' },
        { view: 'commission', label: '💸 Fee Projection' },
        { view: 'timeframe_settings', label: '🕐 Timeframe Settings' },
        { view: 'settings', label: '⚙️ Workspace Settings' },
    ];

    function selectView(pair: InstanceState, view: CurrentView) {
        pair.currentView = view;
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
        <p>Connecting to Market Monitor...</p>
    </div>
{:else if !app.sessionActive}
    <WelcomeGate />
{:else}
<div class={styles.terminalBody}>
    <!-- Global Top Navbar -->
    <nav class={styles.globalNavbar}>
        <div class={styles.navbarBrand}>
            <span class={styles.navbarLogo}>MARKET MONITOR</span>
            <span class={styles.navbarSessionBadge}>{app.sessionCurrency} ON {app.sessionExchange}</span>
        </div>
        
        <div class={styles.navbarTabs}>
            <button class={styles.navbarTab} class:active={app.currentGlobalView === 'dashboard'} onclick={() => { app.currentGlobalView = 'dashboard'; }}>
                <svg class={styles.navIcon} width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="9"></rect><rect x="14" y="3" width="7" height="5"></rect><rect x="14" y="12" width="7" height="9"></rect><rect x="3" y="16" width="7" height="5"></rect></svg>
                Dashboard
            </button>
            <button class={styles.navbarTab} class:active={app.currentGlobalView === 'instances' || app.currentGlobalView === 'workspace'} onclick={() => { app.currentGlobalView = 'instances'; }}>
                <svg class={styles.navIcon} width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                Instances
            </button>
            
            <div class={styles.profileMenuWrapper}>
                <button class={styles.navbarTab} class:active={showProfileMenu} onclick={() => showProfileMenu = !showProfileMenu}>
                    <svg class={styles.navIcon} width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle></svg>
                    Profile
                </button>
                {#if showProfileMenu}
                    <div class={styles.profileDropdown} role="menu" style="right: 0; left: auto; margin-top: 8px;">
                        <div class={styles.profileDropdownHeader}>
                            <span class={styles.profileMode}>{app.sessionCurrency} on {app.sessionExchange}</span>
                        </div>
                        <div class={styles.profileDropdownDivider}></div>
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
    <div class={styles.appLayout}>
        <div class={styles.workspaceViewport}>
        {#each Object.keys(app.instancesMap) as tabKey (tabKey)}
            {@const pair = app.instancesMap[tabKey]}
            <div class="{styles.workspaceWindow} {tabKey !== app.activeTab ? styles.hiddenPane : ''}">

                <!-- Flat sub-tab navbar -->
                <div class={styles.workspaceSubHeader}>
                    <div class={styles.subTabsContainer}>
                        {#each SUB_TABS as tab (tab.view)}
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
                    <span class={styles.pairBannerTitle}>{app.pairDisplayFor(pair.symbol)}</span>
                </div>

                <!-- Live Terminal -->
                {#if pair.currentView === 'terminal'}
                    <div class={styles.mainLayout + " " + 'animate-fade'}>
                        <LiveTerminal pairKey={tabKey} />
                    </div>

                <!-- Terminal Monitor -->
                {:else if pair.currentView === 'monitor'}
                    <TerminalMonitor pairKey={tabKey} />

                <!-- Decision Trading -->
                {:else if pair.currentView === 'decision'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <DecisionTrading />
                    </div>

                <!-- Commission Fee Projection -->
                {:else if pair.currentView === 'commission'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <CommissionCalculator />
                    </div>

                <!-- Timeframe Settings -->
                {:else if pair.currentView === 'timeframe_settings'}
                    <TimeframeSettings {pair} {tabKey} onApplied={() => connectWs(app, wsState)} />

                <!-- Workspace Settings -->
                {:else if pair.currentView === 'settings'}
                    <WorkspaceSettings {pair} {tabKey} />
                {/if}

            </div>
        {/each}
    </div>
</div>
{/if}
</div>

{#if showQuitDialog}
    <QuitDialog onclose={() => showQuitDialog = false} />
{/if}
{/if}
