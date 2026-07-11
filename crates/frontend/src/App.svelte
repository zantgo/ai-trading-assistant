<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import type { CurrentView, InstanceState } from './types';

    import LiveTerminal from './components/LiveTerminal.svelte';
    import TerminalMonitor from './components/TerminalMonitor.svelte';
    import AlignmentPanel from './components/AlignmentPanel.svelte';
    import RiskPanel from './components/RiskPanel.svelte';
    import AnalysisPanel from './components/AnalysisPanel.svelte';
    import AdvisoryPanel from './components/AdvisoryPanel.svelte';
    import CommissionCalculator from './components/CommissionCalculator.svelte';
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
    const SUB_TABS: { view: CurrentView; label: string; svg: string }[] = [
        { view: 'terminal',   label: 'Live Panel',        svg: '<polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>' },
        { view: 'monitor',    label: 'Metrics Panel',     svg: '<line x1="18" y1="20" x2="18" y2="10"></line><line x1="12" y1="20" x2="12" y2="4"></line><line x1="6" y1="20" x2="6" y2="14"></line>' },
        { view: 'alignment', label: 'Alignment Panel',  svg: '<path d="M17 18a5 5 0 0 0-10 0"></path><line x1="12" y1="9" x2="12" y2="2"></line><line x1="4.22" y1="10.22" x2="1.5" y2="8"></line><line x1="19.78" y1="10.22" x2="22.5" y2="8"></line>' },
        { view: 'risk',      label: 'Risk Panel',       svg: '<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line>' },
        { view: 'analysis',  label: 'Analysis Panel',   svg: '<circle cx="12" cy="12" r="10"></circle><line x1="22" y1="12" x2="18" y2="12"></line><line x1="6" y1="12" x2="2" y2="12"></line><line x1="12" y1="6" x2="12" y2="2"></line><line x1="12" y1="22" x2="12" y2="18"></line>' },
        { view: 'advisory',  label: 'Advisory Panel',   svg: '<path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>' },
        { view: 'commission', label: 'Fee Projection',    svg: '<line x1="12" y1="1" x2="12" y2="23"></line><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path>' },
        { view: 'settings',   label: 'Workspace Settings', svg: '<circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>' },
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
                <button class={styles.navbarTab} class:active={showProfileMenu || app.currentGlobalView === 'settings'} onclick={() => showProfileMenu = !showProfileMenu}>
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
                                <svg class="sub-tab-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{@html tab.svg}</svg>
                                {tab.label}
                            </button>
                        {/each}
                    </div>
                </div>

                <div class={styles.instancePairBanner}>
                    <span class={styles.pairBannerTitle}>{app.pairDisplayFor(pair.symbol)}</span>
                </div>

                <!-- Live Panel -->
                {#if pair.currentView === 'terminal'}
                    <div class={styles.mainLayout + " " + 'animate-fade'}>
                        <LiveTerminal pairKey={tabKey} />
                    </div>

                <!-- Metrics Panel -->
                {:else if pair.currentView === 'monitor'}
                    <TerminalMonitor pairKey={tabKey} />

                <!-- Alignment Panel -->
                {:else if pair.currentView === 'alignment'}
                    <AlignmentPanel pairKey={tabKey} />

                <!-- Risk Panel -->
                {:else if pair.currentView === 'risk'}
                    <RiskPanel pairKey={tabKey} />

                <!-- Analysis Panel -->
                {:else if pair.currentView === 'analysis'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <AnalysisPanel />
                    </div>

                <!-- Advisory Panel -->
                {:else if pair.currentView === 'advisory'}
                    <AdvisoryPanel pairKey={tabKey} />

                <!-- Commission Fee Projection -->
                {:else if pair.currentView === 'commission'}
                    <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                        <CommissionCalculator />
                    </div>

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
