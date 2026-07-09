<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import type { CurrentView } from './types';

    import LiveTerminal from './components/LiveTerminal.svelte';
    import WorkflowState from './components/WorkflowState.svelte';
    import WorkflowMetrics from './components/WorkflowMetrics.svelte';
    import WorkflowDecision from './components/WorkflowDecision.svelte';
    import PerformanceDashboard from './components/PerformanceDashboard.svelte';
    import TradeListLedger from './components/TradeListLedger.svelte';
    import PairDashboard from './components/PairDashboard.svelte';

    import GeneralDashboard from './components/GeneralDashboard.svelte';
    import GeneralSettings from './components/GeneralSettings.svelte';
    import InstanceList from './components/InstanceList.svelte';

    import WelcomeGate from './WelcomeGate.svelte';
    import QuitDialog from './QuitDialog.svelte';
    import UserProfile from './components/UserProfile.svelte';
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

    // ─── App store & component-local state ──────────────────────────────────
    const app = useAppStore();
    const wsState: WsState = createWsState();
    let showProfileMenu = $state(false);
    let manageInstances = $state(false);

    // ─── Workflow sub-tabs ────────────────────────────────────────────────
    const WORKFLOW_TABS: { view: CurrentView; label: string; icon: IconName }[] = [
        { view: 'workflow_charts', label: 'Charts', icon: 'trending-up' },
        { view: 'workflow_state', label: 'State', icon: 'target' },
        { view: 'workflow_metrics', label: 'Metrics', icon: 'bar-chart' },
        { view: 'workflow_decision', label: 'Decision', icon: 'compass' },
        { view: 'workflow_performance', label: 'Performance', icon: 'bar-chart' },
        { view: 'workflow_ledger', label: 'Ledger', icon: 'book' },
    ];

    function selectWorkflowTab(view: CurrentView) {
        app.workflowView = view;
        if (view === 'workflow_state') app.fetchPaperStatus();
    }

    function selectPair(pairKey: string) {
        app.activeTab = pairKey;
        app.currentGlobalView = 'instances';
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

    // ─── Derived: pair tabs ──────────────────────────────────────────
    const pairKeys = $derived(Object.keys(app.instancesMap));
</script>

{#if !app.sessionChecked}
    <div class={styles.sessionLoading}>
        <div class={styles.loadingSpinner}></div>
        <p>Connecting to Quantitative Trading Engine...</p>
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
                <span class={styles.brandHeaderTitle}>QUANTITATIVE TRADING ENGINE</span>
                <span class={styles.navbarPillBadge}>
                    <span class={styles.exchangeBadgeCurrency}>{app.sessionCurrency}</span>
                    <span class={styles.exchangeBadgeOn}> on </span>
                    <span class={styles.exchangeBadgeExchange}>{app.sessionExchange}</span>
                </span>
                <span class="{styles.navbarPillBadge} {app.sessionMode === 'live' ? styles.navbarPillBadgeActiveLive : styles.navbarPillBadgeActivePaper}">{app.sessionMode?.toUpperCase()}</span>
                <!-- Pair tabs -->
                {#if pairKeys.length > 0}
                    <div class={styles.pairTabsGroup}>
                        {#each pairKeys as pk}
                            <button class={styles.pairTab}
                                class:pairTabActive={pk === app.activeTab && app.currentGlobalView === 'instances'}
                                onclick={() => selectPair(pk)}>
                                {app.instancesMap[pk].symbol}
                            </button>
                        {/each}
                    </div>
                {/if}
            </div>
            <!-- Right: view switcher + profile -->
            <div class={styles.navRightGroup}>
                <div class={styles.navbarTabs}>
                    <button class={styles.navbarTab} class:active={app.currentGlobalView === 'dashboard'} onclick={() => { app.currentGlobalView = 'dashboard'; manageInstances = false; }}>
                        <Icon name="dashboard" size={14} /> Dashboard
                    </button>
                    <button class={styles.navbarTab} class:active={app.currentGlobalView === 'instances'} onclick={() => { app.currentGlobalView = 'instances'; }}>
                        <Icon name="instances" size={14} /> Instances
                    </button>
                    <button class={styles.navbarTab} class:active={app.currentGlobalView === 'settings'} onclick={() => { app.currentGlobalView = 'settings'; manageInstances = false; }}>
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
    {:else if app.currentGlobalView === 'settings'}
        <GeneralSettings />
    {:else if app.currentGlobalView === 'user_profile'}
        <UserProfile />
    {:else if app.currentGlobalView === 'instances'}
        {#if manageInstances || pairKeys.length === 0}
            <div class={styles.appLayout}>
                <div class={styles.manageHeader}>
                    {#if pairKeys.length > 0}
                        <button class={styles.backBtn} onclick={() => manageInstances = false}>
                            ← Back to instances
                        </button>
                    {/if}
                </div>
                <InstanceList />
            </div>
        {:else}
        <div class={styles.appLayout}>
            <div class={styles.workspaceViewport}>
                <!-- Active pair workspace -->
                {#if app.activeTab && app.instancesMap[app.activeTab]}
                    {@const pair = app.instancesMap[app.activeTab]}
                    <div class={styles.workspaceWindow}>

                        <!-- Instance-level navbar: General | Workflow -->
                        <div class={styles.instanceNavbar}>
                            <button class={styles.instanceNavTab}
                                class:active={app.instanceView === 'general'}
                                onclick={() => app.instanceView = 'general'}>
                                <Icon name="dashboard" size={13} /> General
                            </button>
                            <button class={styles.instanceNavTab}
                                class:active={app.instanceView === 'workflow'}
                                onclick={() => app.instanceView = 'workflow'}>
                                <Icon name="target" size={13} /> Workflow
                            </button>
                            <button class={styles.instanceNavMuted} onclick={() => manageInstances = true}>
                                <Icon name="settings" size={12} /> Manage
                            </button>
                        </div>

                        <!-- Pair banner -->
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

                        <!-- Content based on instanceView -->
                        {#if app.instanceView === 'general'}
                            <PairDashboard pairKey={app.activeTab} />
                        {:else}
                            <!-- Workflow sub-tab navigation -->
                            <div class={styles.workspaceSubHeader}>
                                <div class={styles.subTabsContainer}>
                                    {#each WORKFLOW_TABS as tab (tab.view)}
                                        <button
                                            class="{styles.subTabBtn} {app.workflowView === tab.view ? styles.subTabActive : ''}"
                                            onclick={() => selectWorkflowTab(tab.view)}
                                        >
                                            <Icon name={tab.icon} size={13} /> {tab.label}
                                        </button>
                                    {/each}
                                </div>
                            </div>

                            <!-- Workflow Tab Content -->
                            {#if app.workflowView === 'workflow_charts'}
                                <div class={styles.mainLayout + " " + 'animate-fade'}>
                                    <LiveTerminal pairKey={app.activeTab} />
                                </div>
                            {:else if app.workflowView === 'workflow_state'}
                                <WorkflowState pairKey={app.activeTab} />
                            {:else if app.workflowView === 'workflow_metrics'}
                                <WorkflowMetrics pairKey={app.activeTab} />
                            {:else if app.workflowView === 'workflow_decision'}
                                <WorkflowDecision pairKey={app.activeTab} />
                            {:else if app.workflowView === 'workflow_performance'}
                                <div class={styles.workspaceInnerContent}>
                                    <PerformanceDashboard />
                                </div>
                            {:else if app.workflowView === 'workflow_ledger'}
                                <div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
                                    <TradeListLedger />
                                </div>
                            {/if}
                        {/if}

                    </div>
                {:else}
                    <div class={styles.emptyState}>Select an instance pair above to begin.</div>
                {/if}
            </div>
        </div>
        {/if}
    {/if}
</div>

{#if app.showQuitDialog}
    <QuitDialog onclose={() => app.showQuitDialog = false} />
{/if}
{/if}
