<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import type { CurrentView } from './types';

    // ─── Component imports ──────────────────────────────────────────────
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

    // ─── Styles ─────────────────────────────────────────────────────────
    import styles from './styles/brutalist-grid.module.css';

    // ─── Lib imports (live data pipeline — preserved) ───────────────────
    import { fetchConfigFromServer, applyConfigToStore } from './lib/api.svelte';
    import {
        createWsState, disconnectAllWs,
        connectWebsocket as connectWs, shouldReconnect,
        type WsState,
    } from './lib/websocket.svelte';

    const app = useAppStore();
    const wsState: WsState = createWsState();

    // ─── Component-local UI state ───────────────────────────────────────
    let showQuitDialog = $state(false);
    let showInstancesDropdown = $state(false);
    let showMenuDropdown = $state(false);

    // ─── Manage Instances modal state ───────────────────────────────────
    interface InstanceRow { id: string; pair: string; symbol: string; status: string; }
    let manageInstances = $state<InstanceRow[]>([]);
    let manageLoading = $state(false);
    let actionLoading = $state<Record<string, string>>({});

    // ─── Instance sub-tabs (Row 5) ──────────────────────────────────────
    const SUB_TABS: { view: CurrentView; label: string }[] = [
        { view: 'terminal',  label: 'Live Panel' },
        { view: 'monitor',   label: 'Metrics' },
        { view: 'alignment', label: 'Alignment' },
        { view: 'risk',      label: 'Risk & Reward' },
        { view: 'analysis',  label: 'Analysis' },
        { view: 'advisory',  label: 'Advisory' },
    ];

    const activePair = $derived(app.instancesMap[app.activeTab]);
    const pairKeys = $derived(Object.keys(app.instancesMap));

    function closeDropdowns() {
        showInstancesDropdown = false;
        showMenuDropdown = false;
    }

    function selectSubView(view: CurrentView) {
        if (activePair) activePair.currentView = view;
    }

    function viewTitle(): string {
        switch (app.currentGlobalView) {
            case 'dashboard': return 'Portfolio';
            case 'instances': return 'Trade Automation';
            case 'settings': return 'Profile';
            default: return 'Market Monitor';
        }
    }

    // ─── Config & live-data lifecycle (preserved from previous shell) ────
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

    // ─── Manage Instances modal actions ─────────────────────────────────
    async function fetchManageInstances() {
        manageLoading = true;
        try {
            const res = await fetch('/api/instances');
            if (res.ok) {
                const data = await res.json();
                manageInstances = data.instances || [];
            }
        } catch (_) { /* backend may be unavailable */ }
        finally { manageLoading = false; }
    }

    $effect(() => {
        if (app.isManageModalOpen) fetchManageInstances();
    });

    async function handleInstanceAction(id: string, action: 'pause' | 'stop' | 'delete', pair?: string) {
        const verb = action === 'delete' ? 'DELETE' : 'POST';
        const url = action === 'delete'
            ? `/api/instances/${encodeURIComponent(id)}`
            : `/api/instances/${encodeURIComponent(id)}/${action}`;

        actionLoading = { ...actionLoading, [id]: action };
        try {
            await fetch(url, { method: verb });
            if (action === 'delete' && pair) app.removeInstance(pair);
            await fetchManageInstances();
            await app.fetchSessionStatus();
        } finally {
            const next = { ...actionLoading };
            delete next[id];
            actionLoading = next;
        }
    }

    function statusClass(status: string): string {
        switch (status) {
            case 'running': return styles.statusRunning;
            case 'paused': return styles.statusPaused;
            case 'stopped': return styles.statusStopped;
            default: return styles.statusStopped;
        }
    }
</script>

{#if !app.sessionChecked}
    <div class={styles.loading}>
        <div class={styles.spinner}></div>
        <span>Connecting to Market Monitor…</span>
    </div>
{:else if !app.sessionActive}
    <WelcomeGate />
{:else}
    <div class={styles.gridContainer}>

        <!-- Row 1: Navbar (5 columns) -->
        <header class="{styles.row} {styles.rowNavbar}">
            <div class="{styles.cell} {styles.cellBrand}">Trading Platform</div>
            <div class="{styles.cell} {styles.cellMono}">{app.sessionExchange} · {app.sessionCurrency}</div>
            <div class={styles.cell}></div>

            <!-- :: Instances trigger -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.cellClickable} {showInstancesDropdown ? styles.cellActive : ''}"
                onclick={() => { showInstancesDropdown = !showInstancesDropdown; showMenuDropdown = false; }}
            >
                :: Instances
                {#if showInstancesDropdown}
                    <div class={styles.dropdownMenu}>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.isManageModalOpen = true; closeDropdowns(); }}>Manage</button>
                        {#each pairKeys as pKey (pKey)}
                            <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.activeTab = pKey; app.currentGlobalView = 'workspace'; app.activeMainTab = 'overview'; closeDropdowns(); }}>{pKey}</button>
                        {/each}
                        {#if pairKeys.length === 0}
                            <span class={styles.dropdownItem}>No Instances</span>
                        {/if}
                    </div>
                {/if}
            </div>

            <!-- ☰ Menu trigger -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.cellClickable} {showMenuDropdown ? styles.cellActive : ''}"
                onclick={() => { showMenuDropdown = !showMenuDropdown; showInstancesDropdown = false; }}
            >
                ☰ Menu
                {#if showMenuDropdown}
                    <div class={styles.dropdownMenu}>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.currentGlobalView = 'settings'; closeDropdowns(); }}>Profile</button>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.currentGlobalView = 'dashboard'; closeDropdowns(); }}>Portfolio</button>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.currentGlobalView = 'workspace'; app.activeMainTab = 'overview'; closeDropdowns(); }}>Market</button>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.currentGlobalView = 'instances'; closeDropdowns(); }}>Trading</button>
                    </div>
                {/if}
            </div>
        </header>

        <!-- Row 2: View title / Settings split -->
        <section class="{styles.row} {styles.rowSplitHeader}">
            <div class={styles.cell} style="justify-content: flex-start; padding-left: 20px; color: var(--text); letter-spacing: 0.08em;">
                {viewTitle()}
            </div>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.cellClickable} {app.currentGlobalView === 'settings' ? styles.cellActive : ''}"
                onclick={() => app.currentGlobalView = 'settings'}
            >
                Settings
            </div>
        </section>

        <!-- Row 3: Navigation tabs -->
        <nav class="{styles.row} {styles.rowTabs}">
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.tabCell} {styles.cellClickable} {app.currentGlobalView === 'workspace' && app.activeMainTab === 'overview' ? styles.cellActive : ''}"
                onclick={() => { app.activeMainTab = 'overview'; app.currentGlobalView = 'workspace'; }}
            >
                Overview Panel
            </div>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.tabCell} {styles.cellClickable} {app.currentGlobalView === 'instances' ? styles.cellActive : ''}"
                onclick={() => { app.currentGlobalView = 'instances'; }}
            >
                Instances
            </div>
            {#if activePair}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                    class="{styles.cell} {styles.tabCell} {styles.cellClickable} {app.currentGlobalView === 'workspace' && activePair.currentView === 'commission' ? styles.cellActive : ''}"
                    onclick={() => { app.currentGlobalView = 'workspace'; app.activeMainTab = 'fee_projection'; selectSubView('commission'); }}
                >
                    Fee Projection
                </div>
            {/if}
        </nav>

        <!-- Row 4 & 5: Instance sub-header + sub-tabs (workspace only) -->
        {#if app.currentGlobalView === 'workspace' && activePair}
            <section class="{styles.row} {styles.rowSplitHeader}">
                <div class={styles.cell} style="justify-content: flex-start; padding-left: 20px;">
                    <span class={styles.cellMono}>{app.pairDisplayFor(activePair.symbol)}</span>
                </div>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                    class="{styles.cell} {styles.cellClickable} {activePair.currentView === 'settings' ? styles.cellActive : ''}"
                    onclick={() => selectSubView('settings')}
                >
                    Settings
                </div>
            </section>

            {#if app.activeMainTab === 'overview'}
                <nav class="{styles.row} {styles.rowTabs} {styles.rowSubTabs}">
                    {#each SUB_TABS as tab (tab.view)}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div
                            class="{styles.cell} {styles.tabCell} {styles.cellClickable} {activePair.currentView === tab.view ? styles.cellActive : ''}"
                            onclick={() => selectSubView(tab.view)}
                        >
                            {tab.label}
                        </div>
                    {/each}
                </nav>
            {/if}
        {/if}

        <!-- Row 6: Main content frame -->
        <main class={styles.contentArea}>
            {#if app.currentGlobalView === 'dashboard'}
                <GeneralDashboard />
            {:else if app.currentGlobalView === 'instances'}
                <InstanceList />
            {:else if app.currentGlobalView === 'settings'}
                <GeneralSettings />
                <div class={styles.quitContainer}>
                    <button class={styles.btnQuit} onclick={() => showQuitDialog = true}>Quit Session</button>
                </div>
            {:else if app.currentGlobalView === 'workspace' && activePair}
                {#if activePair.currentView === 'terminal'}
                    <LiveTerminal pairKey={app.activeTab} />
                {:else if activePair.currentView === 'monitor'}
                    <TerminalMonitor pairKey={app.activeTab} />
                {:else if activePair.currentView === 'alignment'}
                    <AlignmentPanel pairKey={app.activeTab} />
                {:else if activePair.currentView === 'risk'}
                    <RiskPanel pairKey={app.activeTab} />
                {:else if activePair.currentView === 'analysis'}
                    <AnalysisPanel />
                {:else if activePair.currentView === 'advisory'}
                    <AdvisoryPanel pairKey={app.activeTab} />
                {:else if activePair.currentView === 'commission'}
                    <CommissionCalculator />
                {:else if activePair.currentView === 'settings'}
                    <WorkspaceSettings pair={activePair} tabKey={app.activeTab} />
                {/if}
            {:else if app.currentGlobalView === 'workspace'}
                <div class={styles.modalEmpty} style="padding-top: 64px;">No active instance. Create one under Instances.</div>
            {/if}
        </main>

    </div>

    <!-- Dropdown click-outside backdrop -->
    {#if showInstancesDropdown || showMenuDropdown}
        <div class={styles.dropdownBackdrop} role="presentation" onclick={closeDropdowns}></div>
    {/if}

    <!-- Manage Instances modal -->
    {#if app.isManageModalOpen}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class={styles.backdrop} onclick={() => app.isManageModalOpen = false}>
            <div class={styles.modalWindow} role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
                <div class={styles.modalHeader}>
                    <div class={styles.cell}>::</div>
                    <div class="{styles.cell} {styles.modalTitle}">Manage Instances</div>
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div class="{styles.cell} {styles.cellClickable}" onclick={() => app.isManageModalOpen = false}>✕</div>
                </div>

                <div class={styles.modalBody}>
                    {#if manageLoading}
                        <div class={styles.modalEmpty}>Loading instances…</div>
                    {:else if manageInstances.length === 0}
                        <div class={styles.modalEmpty}>No active instances.</div>
                    {:else}
                        {#each manageInstances as inst (inst.id)}
                            <div class={styles.modalRow}>
                                <div class={styles.cell} style="justify-content: flex-start; padding-left: 16px;">
                                    <span class="{styles.statusDot} {statusClass(inst.status)}"></span>
                                    <span class={styles.cellMono}>{inst.pair}</span>
                                </div>
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                <div class="{styles.cell} {styles.cellClickable}" onclick={() => handleInstanceAction(inst.id, 'pause')}>Pause</div>
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                <div class="{styles.cell} {styles.cellClickable}" onclick={() => handleInstanceAction(inst.id, 'stop')}>Stop</div>
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                <div class="{styles.cell} {styles.cellClickable}" onclick={() => handleInstanceAction(inst.id, 'delete', inst.pair)}>Delete</div>
                            </div>
                        {/each}
                    {/if}
                </div>
            </div>
        </div>
    {/if}

    {#if showQuitDialog}
        <QuitDialog onclose={() => showQuitDialog = false} />
    {/if}
{/if}
