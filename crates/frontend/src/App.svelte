<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import { createInstance } from './lib/api.svelte';
    import type { CurrentView } from './types';

    // ─── Component imports ──────────────────────────────────────────────
    import LiveTerminal from './components/LiveTerminal.svelte';
    import TerminalMonitor from './components/TerminalMonitor.svelte';
    import AlignmentPanel from './components/AlignmentPanel.svelte';
    import OpportunitiesPanel from './components/OpportunitiesPanel.svelte';
    import RiskPanel from './components/RiskPanel.svelte';
    import AnalysisPanel from './components/AnalysisPanel.svelte';
    import AdvisoryPanel from './components/AdvisoryPanel.svelte';
    import GeneralDashboard from './components/GeneralDashboard.svelte';
    import GeneralSettings from './components/GeneralSettings.svelte';
    import WorkspaceSettings from './components/WorkspaceSettings.svelte';
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
    let isSidebarOpen = $state(false);

    // ─── Manage Workspaces modal state ──────────────────────────────────
    interface InstanceRow { id: string; pair: string; symbol: string; status: string; }
    let manageInstances = $state<InstanceRow[]>([]);
    let manageLoading = $state(false);
    let actionLoading = $state<Record<string, string>>({});
    let newBase = $state('');
    let createLoading = $state(false);
    let createError = $state<string | null>(null);
    let confirmAction = $state<{ id: string; action: 'pause' | 'delete'; pair?: string } | null>(null);

    // ─── Engines ────────────────────────────────────────────────────────
    type EngineKey = 'profile' | 'portfolio' | 'market_monitor' | 'trade_automation' | 'performance';
    const ENGINES_SIDEBAR: { key: EngineKey; label: string; icon: string }[] = [
        { key: 'profile',        label: 'Home',     icon: '🏠' },
        { key: 'portfolio',      label: 'Portfolio', icon: '📊' },
        { key: 'market_monitor', label: 'Market',    icon: '📈' },
        { key: 'trade_automation', label: 'Trading', icon: '💰' },
        { key: 'performance',    label: 'Analysis',  icon: '🔍' },
    ];

    const MIDDLE_TABS: { key: string; label: string }[] = [
        { key: 'overview',  label: 'Overview' },
        { key: 'settings',  label: 'Settings' },
    ];

    // ─── Sub-tabs (Navbar 3, Market + workspace selected) ───────────────
    const SUB_TABS: { view: CurrentView; label: string }[] = [
        { view: 'terminal',    label: 'Charts' },
        { view: 'monitor',     label: 'Metrics' },
        { view: 'alignment',   label: 'Alignment' },
        { view: 'opportunity', label: 'Opportunities' },
        { view: 'risk',        label: 'Risks' },
        { view: 'analysis',    label: 'Analysis' },
        { view: 'advisory',    label: 'Decision' },
    ];

    const activePair = $derived(app.selectedInstance ? app.instancesMap[app.selectedInstance] : undefined);
    const pairKeys = $derived(Object.keys(app.instancesMap));

    // ─── Derived top label ───────────────────────────────────────────────
    const topLabel = $derived(app.currentEngine === 'profile' ? 'TRADING PLATFORM' : engineLabel(app.currentEngine));
    const isHome = $derived(app.currentEngine === 'profile');

    // ─── 24h change ─────────────────────────────────────────────────────
    const livePrice = $derived(activePair ? activePair.microTerm.priceText : '--');
    const change24h = $derived.by<number | null>(() => {
        if (!activePair) return null;
        const snap = activePair.microTerm.latestSnapshot || activePair.fastTerm.latestSnapshot;
        if (!snap) return null;
        const mid = parseFloat(String((snap as Record<string, unknown>).mid_price ?? ''));
        const prev = parseFloat(String((snap as Record<string, unknown>).prev_day_px ?? ''));
        if (!isFinite(mid) || !isFinite(prev) || prev === 0) return null;
        return ((mid - prev) / prev) * 100;
    });

    function changeClass(v: number): string {
        if (v > 0) return styles.changeUp;
        if (v < 0) return styles.changeDown;
        return styles.changeFlat;
    }

    function closeInstancesDropdown() {
        showInstancesDropdown = false;
    }

    function toggleSidebar() { isSidebarOpen = !isSidebarOpen; }
    function closeSidebar() { isSidebarOpen = false; }

    function selectSubView(view: CurrentView) {
        if (activePair) {
            activePair.currentView = view;
            app.activeEngineTab = 'instance';
        }
    }

    // ─── Config & live-data lifecycle ───────────────────────────────────
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

    // ─── Manage Workspaces modal actions ────────────────────────────────
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

    async function handleCreateWorkspace() {
        const base = newBase.trim().toUpperCase();
        if (!base) return;
        createLoading = true;
        createError = null;
        try {
            const result = await createInstance(base, app.quote);
            if (result.ok) {
                app.initInstance(base);
                newBase = '';
                await fetchManageInstances();
                await app.fetchSessionStatus();
            } else {
                createError = result.error || 'Failed to create workspace.';
            }
        } catch (_) {
            createError = 'Failed to create workspace.';
        } finally {
            createLoading = false;
        }
    }

    function handleCreateKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter') handleCreateWorkspace();
    }

    function requestConfirm(id: string, action: 'pause' | 'delete', pair?: string) {
        confirmAction = { id, action, pair };
    }

    function cancelConfirm() { confirmAction = null; }

    async function executeConfirmed() {
        if (!confirmAction) return;
        const { id, action, pair } = confirmAction;
        confirmAction = null;
        const verb = action === 'delete' ? 'DELETE' : 'POST';
        const url = action === 'delete'
            ? `/api/instances/${encodeURIComponent(id)}`
            : `/api/instances/${encodeURIComponent(id)}/${action}`;

        actionLoading = { ...actionLoading, [id]: action };
        try {
            await fetch(url, { method: verb });
            if (action === 'delete' && pair) {
                app.removeInstance(pair);
                if (app.selectedInstance === pair) app.exitInstance();
            }
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

    function engineLabel(key: string): string {
        return ENGINES_SIDEBAR.find(e => e.key === key)?.label?.toUpperCase() ?? 'COMING SOON';
    }

    function sidebarItemClass(key: EngineKey): string {
        const base = styles.sidebarItem;
        return app.currentEngine === key ? `${base} ${styles.sidebarItemActive}` : base;
    }

    function navigateTo(engine: EngineKey) {
        app.selectEngine(engine);
        closeSidebar();
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

        <!-- Navbar 1: Top bar (4 columns) -->
        <header class="{styles.row} {styles.rowNavbar}">
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="{styles.cell} {styles.cellBrand} {styles.cellClickable}" onclick={toggleSidebar}>
                {topLabel}
            </div>
            <div class="{styles.cell} {styles.cellMono}" style="justify-content: flex-start; padding-left: 4px;">
                {#if !isHome}
                    <span class={styles.exchangeChip}>{app.sessionExchange} · {app.sessionCurrency}</span>
                {/if}
            </div>
            <div class={styles.cell}></div>

            <!-- Workspaces -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.cellClickable} {showInstancesDropdown ? styles.cellActive : ''}"
                onclick={() => { showInstancesDropdown = !showInstancesDropdown; }}
            >
                {#if app.selectedInstance && activePair}
                    <span class={styles.instanceDisplay}>
                        <span class={styles.instancePair}>{app.pairDisplayFor(activePair.symbol)}</span>
                        <span class={styles.instancePrice}>{livePrice}</span>
                        {#if change24h !== null}
                            <span class="{styles.change} {changeClass(change24h)}">
                                {change24h > 0 ? '+' : ''}{change24h.toFixed(2)}%
                            </span>
                        {/if}
                    </span>
                {:else}
                    <span class={styles.navLabel}>
                        <svg class={styles.navIcon} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/>
                            <rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
                        </svg>
                        Workspaces
                    </span>
                {/if}

                {#if showInstancesDropdown}
                    <div class={styles.dropdownMenu}>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.isManageModalOpen = true; closeInstancesDropdown(); }}>Manage</button>
                        {#if app.selectedInstance}
                            <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.exitInstance(); closeInstancesDropdown(); }}>Deselect</button>
                        {/if}
                        {#each pairKeys as pKey (pKey)}
                            <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.enterInstance(pKey); closeInstancesDropdown(); }}>{pKey}</button>
                        {/each}
                        {#if pairKeys.length === 0}
                            <span class={styles.dropdownItem}>No Workspaces</span>
                        {/if}
                    </div>
                {/if}
            </div>
        </header>

        <!-- Navbar 2: Middle tabs (Overview / Settings) — all engines except HOME -->
        {#if !isHome}
            <nav class="{styles.row} {styles.rowTabs}">
                {#each MIDDLE_TABS as tab (tab.key)}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div
                        class="{styles.cell} {styles.tabCell} {styles.cellClickable} {app.middleTab === tab.key ? styles.cellActive : ''}"
                        onclick={() => app.middleTab = tab.key}
                    >
                        {tab.label}
                    </div>
                {/each}
            </nav>
        {/if}

        <!-- Navbar 3: Sub-tabs (only Market + Overview + workspace selected) -->
        {#if app.currentEngine === 'market_monitor' && app.middleTab === 'overview' && app.selectedInstance && activePair}
            <nav class="{styles.row} {styles.rowTabs} {styles.rowSubTabs}">
                {#each SUB_TABS as tab (tab.view)}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div
                        class="{styles.cell} {styles.tabCell} {styles.cellClickable} {app.activeEngineTab === 'instance' && activePair.currentView === tab.view ? styles.cellActive : ''}"
                        onclick={() => selectSubView(tab.view)}
                    >
                        {tab.label}
                    </div>
                {/each}
            </nav>
        {/if}

        <!-- Content frame -->
        <main class={styles.contentArea}>
            {#if app.currentEngine === 'profile'}
                <GeneralSettings />
                <div class={styles.quitContainer}>
                    <button class={styles.btnQuit} onclick={() => showQuitDialog = true}>Quit Session</button>
                </div>
            {:else if app.currentEngine === 'market_monitor'}
                {#if app.middleTab === 'overview'}
                    {#if app.selectedInstance && activePair}
                        {#if activePair.currentView === 'terminal'}
                            <LiveTerminal pairKey={app.activeTab} />
                        {:else if activePair.currentView === 'monitor'}
                            <TerminalMonitor pairKey={app.activeTab} />
                        {:else if activePair.currentView === 'alignment'}
                            <AlignmentPanel pairKey={app.activeTab} />
                        {:else if activePair.currentView === 'opportunity'}
                            <OpportunitiesPanel pairKey={app.activeTab} />
                        {:else if activePair.currentView === 'risk'}
                            <RiskPanel pairKey={app.activeTab} />
                        {:else if activePair.currentView === 'analysis'}
                            <AnalysisPanel />
                        {:else if activePair.currentView === 'advisory'}
                            <AdvisoryPanel pairKey={app.activeTab} />
                        {/if}
                    {:else}
                        <GeneralDashboard />
                    {/if}
                {:else}
                    <WorkspaceSettings pair={activePair} tabKey={app.activeTab} />
                {/if}
            {:else}
                <div class={styles.placeholder}>
                    <span class={styles.placeholderTitle}>{engineLabel(app.currentEngine)}</span>
                    <span class={styles.placeholderSub}>Coming soon</span>
                </div>
            {/if}
        </main>

    </div>

    <!-- Lateral sidebar overlay -->
    {#if isSidebarOpen}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class={styles.sidebarOverlay} role="presentation" onclick={closeSidebar}></div>
        <div class={styles.sidebarPanel}>
            <div class={styles.sidebarBrand}>TRADING PLATFORM</div>
            <div class={styles.sidebarNav}>
                {#each ENGINES_SIDEBAR as engine (engine.key)}
                    <button class={sidebarItemClass(engine.key)} onclick={() => navigateTo(engine.key)}>
                        <span>{engine.icon}</span>
                        {engine.label}
                    </button>
                {/each}
            </div>
        </div>
    {/if}

    <!-- Dropdown click-outside backdrop -->
    {#if showInstancesDropdown}
        <div class={styles.dropdownBackdrop} role="presentation" onclick={closeInstancesDropdown}></div>
    {/if}

    <!-- Manage Workspaces modal -->
    {#if app.isManageModalOpen}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class={styles.backdrop} onclick={() => { confirmAction = null; app.isManageModalOpen = false; }}>
            <div class={styles.modalWindow} role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
                <div class={styles.modalHeader}>
                    <div class={styles.cell}>::</div>
                    <div class="{styles.cell} {styles.modalTitle}">Manage Workspaces</div>
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div class="{styles.cell} {styles.cellClickable}" onclick={() => { confirmAction = null; app.isManageModalOpen = false; }}>✕</div>
                </div>

                <!-- Create workspace bar -->
                <div class={styles.modalCreateBar}>
                    <input
                        type="text"
                        class={styles.modalInput}
                        placeholder="Symbol (e.g. BTC)"
                        bind:value={newBase}
                        maxlength="10"
                        oninput={() => { if (createError) createError = null; }}
                        onkeydown={handleCreateKeydown}
                    />
                    <span class={styles.modalQuoteChip}>{app.quote}</span>
                    <button class={styles.modalCreateBtn} onclick={handleCreateWorkspace} disabled={createLoading || !newBase.trim()}>
                        {#if createLoading}
                            <span class={styles.wavingDots}>
                                <span class={styles.wavingDot}></span>
                                <span class={styles.wavingDot}></span>
                                <span class={styles.wavingDot}></span>
                            </span>
                        {:else}
                            +
                        {/if}
                    </button>
                </div>

                {#if createError}
                    <div class={styles.modalError}>{createError}</div>
                {/if}

                <div class={styles.modalBody}>
                    {#if manageLoading}
                        <div class={styles.modalEmpty}>Loading workspaces…</div>
                    {:else if manageInstances.length === 0}
                        <div class={styles.modalEmpty}>No active workspaces. Create one above.</div>
                    {:else}
                        {#each manageInstances as inst (inst.id)}
                            <div class={styles.modalRow}>
                                <div class={styles.modalPairCell}>
                                    <span class="{styles.statusDot} {statusClass(inst.status)}"></span>
                                    {inst.pair}
                                </div>
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                <div class={styles.modalActionCell} onclick={() => requestConfirm(inst.id, 'pause')}>
                                    {#if confirmAction?.id === inst.id && confirmAction?.action === 'pause'}
                                        <div class={styles.confirmRow}>
                                            <button class="{styles.confirmBtn}" onclick={(e) => { e.stopPropagation(); cancelConfirm(); }}>Cancel</button>
                                            <button class="{styles.confirmBtn}" onclick={(e) => { e.stopPropagation(); executeConfirmed(); }}>Pause</button>
                                        </div>
                                    {:else}
                                        ⏸
                                    {/if}
                                </div>
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                <div class="{styles.modalActionCell} {styles.danger}" onclick={() => requestConfirm(inst.id, 'delete', inst.pair)}>
                                    {#if confirmAction?.id === inst.id && confirmAction?.action === 'delete'}
                                        <div class={styles.confirmRow}>
                                            <button class={styles.confirmBtn} onclick={(e) => { e.stopPropagation(); cancelConfirm(); }}>Cancel</button>
                                            <button class="{styles.confirmBtn} {styles.confirmBtnDanger}" onclick={(e) => { e.stopPropagation(); executeConfirmed(); }}>Delete</button>
                                        </div>
                                    {:else}
                                        🗑
                                    {/if}
                                </div>
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
