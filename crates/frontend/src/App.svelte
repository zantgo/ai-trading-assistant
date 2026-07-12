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

    // ─── Lib imports (live data pipeline) ───────────────────────────────
    import { fetchConfigFromServer, applyConfigToStore } from './lib/api.svelte';
    import {
        createWsState, disconnectAllWs,
        connectWebsocket as connectWs, shouldReconnect,
        type WsState,
    } from './lib/websocket.svelte';

    const app = useAppStore();
    const wsState: WsState = createWsState();

    // ─── UI state ───────────────────────────────────────────────────────
    let showQuitDialog = $state(false);
    let isSidebarOpen = $state(false);
    let isWorkspacePanelOpen = $state(false);
    let confirmModal = $state<{ action: 'pause' | 'delete'; id: string; pair?: string } | null>(null);

    // ─── Workspace panel data ───────────────────────────────────────────
    interface InstanceRow { id: string; pair: string; symbol: string; status: string; }
    let wsInstances = $state<InstanceRow[]>([]);
    let wsLoading = $state(false);
    let newBase = $state('');
    let createLoading = $state(false);
    let createError = $state<string | null>(null);
    let rowConfirm = $state<{ id: string; action: 'pause' | 'delete'; pair?: string } | null>(null);

    // ─── Engines ────────────────────────────────────────────────────────
    type EngineKey = 'profile' | 'portfolio' | 'market_monitor' | 'trade_automation' | 'performance';
    const ENGINES_SIDEBAR: { key: EngineKey; label: string }[] = [
        { key: 'profile',        label: 'Home' },
        { key: 'portfolio',      label: 'Portfolio' },
        { key: 'market_monitor', label: 'Market' },
        { key: 'trade_automation', label: 'Trading' },
        { key: 'performance',    label: 'Analysis' },
    ];

    const MIDDLE_TABS: { key: string; label: string }[] = [
        { key: 'overview',  label: 'Overview' },
        { key: 'settings',  label: 'Settings' },
    ];

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
    const isHome = $derived(app.currentEngine === 'profile');

    // ─── Top label ──────────────────────────────────────────────────────
    const topLabel = $derived(isHome ? 'TRADING PLATFORM' : engineLabel(app.currentEngine));

    // ─── 24h change for selected pair ───────────────────────────────────
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

    function changeStr(pairKey: string): string {
        const inst = app.instancesMap[pairKey];
        if (!inst) return '';
        const snap = inst.microTerm.latestSnapshot || inst.fastTerm.latestSnapshot;
        if (!snap) return '';
        const mid = parseFloat(String((snap as Record<string, unknown>).mid_price ?? ''));
        const prev = parseFloat(String((snap as Record<string, unknown>).prev_day_px ?? ''));
        if (!isFinite(mid) || !isFinite(prev) || prev === 0) return '';
        const v = ((mid - prev) / prev) * 100;
        return (v > 0 ? '+' : '') + v.toFixed(2) + '%';
    }

    function changeCls(v: string): string {
        if (v.startsWith('+')) return styles.changeUp;
        if (v.startsWith('-')) return styles.changeDown;
        return styles.changeFlat;
    }

    // ─── Sidebar ────────────────────────────────────────────────────────
    function toggleSidebar() { isSidebarOpen = !isSidebarOpen; }
    function closeSidebar() { isSidebarOpen = false; }
    function closeWorkspacePanel() { isWorkspacePanelOpen = false; rowConfirm = null; }
    function openWorkspacePanel() { isWorkspacePanelOpen = true; fetchWorkspaces(); }

    function selectSubView(view: CurrentView) {
        if (activePair) { activePair.currentView = view; app.activeEngineTab = 'instance'; }
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
        } catch (e) { console.error('Failed to fetch config:', e); configReady = true; }
    }

    onMount(async () => { app.fetchSessionStatus(); await fetchConfig(); });

    onDestroy(() => { disconnectAllWs(wsState); });

    $effect(() => {
        const tab = app.activeTab;
        if (configReady && tab && shouldReconnect(app, wsState)) connectWs(app, wsState);
    });

    // ─── Workspace panel CRUD ───────────────────────────────────────────
    async function fetchWorkspaces() {
        wsLoading = true;
        try {
            const res = await fetch('/api/instances');
            if (res.ok) { const data = await res.json(); wsInstances = data.instances || []; }
        } catch (_) {}
        finally { wsLoading = false; }
    }

    async function handleCreateWorkspace() {
        const base = newBase.trim().toUpperCase();
        if (!base) return;
        createLoading = true; createError = null;
        try {
            const result = await createInstance(base, app.quote);
            if (result.ok) { app.initInstance(base); newBase = ''; await fetchWorkspaces(); await app.fetchSessionStatus(); }
            else { createError = result.error || 'Failed to create workspace.'; }
        } catch (_) { createError = 'Failed to create workspace.'; }
        finally { createLoading = false; }
    }

    function handleCreateKeydown(e: KeyboardEvent) { if (e.key === 'Enter') handleCreateWorkspace(); }

    function requestRowConfirm(id: string, action: 'pause' | 'delete', pair?: string) { rowConfirm = { id, action, pair }; }
    function cancelRowConfirm() { rowConfirm = null; }

    async function executeRowConfirm() {
        if (!rowConfirm) return;
        const { id, action, pair } = rowConfirm;
        rowConfirm = null;
        const verb = action === 'delete' ? 'DELETE' : 'POST';
        const url = action === 'delete' ? `/api/instances/${encodeURIComponent(id)}` : `/api/instances/${encodeURIComponent(id)}/${action}`;
        try {
            await fetch(url, { method: verb });
            if (action === 'delete' && pair) { app.removeInstance(pair); if (app.selectedInstance === pair) app.exitInstance(); }
            await fetchWorkspaces(); await app.fetchSessionStatus();
        } catch (_) {}
    }

    function pairDisplay(pairKey: string): string {
        return pairKey.replace('-', '/');
    }

    function priceFor(pairKey: string): string {
        return app.instancesMap[pairKey]?.microTerm?.priceText || '--';
    }

    function statusClass(status: string): string {
        switch (status) { case 'running': return styles.statusRunning; case 'paused': return styles.statusPaused; case 'stopped': return styles.statusStopped; default: return styles.statusStopped; }
    }

    function engineLabel(key: string): string {
        return ENGINES_SIDEBAR.find(e => e.key === key)?.label?.toUpperCase() ?? 'COMING SOON';
    }

    function sidebarItemClass(key: EngineKey): string {
        const base = styles.sidebarItem;
        return app.currentEngine === key ? `${base} ${styles.sidebarItemActive}` : base;
    }

    function navigateTo(engine: EngineKey) { app.selectEngine(engine); closeSidebar(); }

    function sidebarSvg(key: EngineKey): string {
        const paths: Record<string, string> = {
            profile: '<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>',
            portfolio: '<rect x="3" y="3" width="18" height="18" rx="2"/><line x1="9" y1="9" x2="15" y2="9"/><line x1="12" y1="15" x2="16" y2="15"/><line x1="8" y1="15" x2="10" y2="15"/>',
            market_monitor: '<polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/>',
            trade_automation: '<line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>',
            performance: '<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>',
        };
        return `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">${paths[key] || ''}</svg>`;
    }
</script>

{#if !app.sessionChecked}
    <div class={styles.loading}><div class={styles.spinner}></div><span>Connecting to Market Monitor…</span></div>
{:else if !app.sessionActive}
    <WelcomeGate />
{:else}
    <div class={styles.gridContainer}>

        <!-- Navbar 1: Top bar -->
        <header class="{styles.row} {styles.rowNavbar}">
            <div class="{styles.cell} {styles.cellBrand} {styles.cellNavbar} {styles.cellClickable}" onclick={toggleSidebar}>
                {#if isHome}
                    <svg class={styles.navIcon} width="12" height="12" viewBox="0 0 24 24" fill="currentColor" stroke="none" style="opacity:0.6">
                        <path d="M12 17l-8-7h16z"/>
                    </svg>
                {:else}
                    <span class={styles.navIcon}>{@html sidebarSvg(app.currentEngine === 'profile' ? 'profile' : app.currentEngine)}</span>
                {/if}
                {topLabel}
            </div>
            <div class="{styles.cell} {styles.cellMono} {styles.cellNavbar}" style="justify-content: flex-start;">
                <span class={styles.exchangeChip}>{app.sessionExchange} · {app.sessionCurrency}</span>
            </div>
            <div class="{styles.cell} {styles.cellNavbar}"></div>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div class="{styles.cell} {styles.cellNavbar} {styles.cellClickable} {isWorkspacePanelOpen ? styles.cellActive : ''}" onclick={openWorkspacePanel}>
                {#if app.selectedInstance && activePair}
                    <span class={styles.instanceDisplay}>
                        <span class={styles.instancePair}>{app.pairDisplayFor(activePair.symbol)}</span>
                        <span class={styles.instancePrice}>{livePrice}</span>
                        {#if change24h !== null}
                            <span class="{styles.change} {changeClass(change24h)}">{change24h > 0 ? '+' : ''}{change24h.toFixed(2)}%</span>
                        {/if}
                    </span>
                {:else}
                    <span class={styles.navLabel}>
                        <svg class={styles.navIcon} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
                        Workspaces
                    </span>
                {/if}
            </div>
        </header>

        <!-- Navbar 2: Middle tabs -->
        {#if !isHome}
            <nav class="{styles.row} {styles.rowTabs}">
                {#if app.currentEngine === 'market_monitor'}
                    <div class="{styles.cell} {styles.tabCellFill} {styles.cellClickable} {app.middleTab === 'workspace' ? styles.cellActive : ''}" onclick={() => app.middleTab = 'workspace'}>Workspace</div>
                {/if}
                {#each MIDDLE_TABS as tab (tab.key)}
                    <div class="{styles.cell} {styles.tabCellFill} {styles.cellClickable} {app.middleTab === tab.key ? styles.cellActive : ''}" onclick={() => app.middleTab = tab.key}>
                        {tab.label}
                    </div>
                {/each}
            </nav>
        {/if}

        <!-- Navbar 3: Sub-tabs -->
        {#if app.currentEngine === 'market_monitor' && app.middleTab === 'workspace' && app.selectedInstance && activePair}
            <nav class="{styles.row} {styles.rowTabs} {styles.rowSubTabs}">
                {#each SUB_TABS as tab (tab.view)}
                    <div class="{styles.cell} {styles.tabCellFill} {styles.cellClickable} {app.activeEngineTab === 'instance' && activePair.currentView === tab.view ? styles.cellActive + ' ' + styles.cellActiveUnderline : ''}" onclick={() => selectSubView(tab.view)}>
                        {tab.label}
                    </div>
                {/each}
            </nav>
        {/if}

        <!-- Content -->
        <main class={styles.contentArea}>
            {#if app.currentEngine === 'profile'}
                <GeneralSettings />
            {:else if app.currentEngine === 'market_monitor'}
                {#if app.middleTab === 'workspace'}
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
                {:else if app.middleTab === 'overview'}
                    <GeneralDashboard />
                {:else}
                    <WorkspaceSettings pair={activePair} tabKey={app.activeTab} />
                {/if}
            {:else}
                <div class={styles.placeholder}><span class={styles.placeholderTitle}>{engineLabel(app.currentEngine)}</span><span class={styles.placeholderSub}>Coming soon</span></div>
            {/if}
        </main>
    </div>

    <!-- Left sidebar -->
    {#if isSidebarOpen}
        <div class={styles.sidebarOverlay} role="presentation" onclick={closeSidebar}></div>
        <div class={styles.sidebarPanel}>
            <div class={styles.sidebarBrand}>TRADING PLATFORM</div>
            <div class={styles.sidebarNav}>
                {#each ENGINES_SIDEBAR as engine (engine.key)}
                    <button class={sidebarItemClass(engine.key)} onclick={() => navigateTo(engine.key)}>
                        <span class={styles.navIcon}>{@html sidebarSvg(engine.key)}</span>{engine.label}
                    </button>
                {/each}
            </div>
            <div class={styles.sidebarFooter}>
                <button class={styles.sidebarQuitBtn} onclick={() => { closeSidebar(); showQuitDialog = true; }}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg>
                    Quit Session
                </button>
            </div>
        </div>
    {/if}

    <!-- Right workspace panel -->
    {#if isWorkspacePanelOpen}
        <div class={styles.workspacePanelOverlay} role="presentation" onclick={closeWorkspacePanel}></div>
        <div class={styles.workspacePanel}>
            <div class={styles.wsPanelHeader}>
                <div class={styles.wsPanelTitle}>
                    <svg class={styles.navIcon} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
                    Workspaces
                </div>
                <div class={styles.wsPanelClose} onclick={closeWorkspacePanel}>✕</div>
            </div>
            <div class={styles.wsPanelCreateBar}>
                <input type="text" class={styles.wsPanelInput} placeholder="Symbol (e.g. BTC)" bind:value={newBase} maxlength="10" oninput={() => { if (createError) createError = null; }} onkeydown={handleCreateKeydown} />
                <span class={styles.wsPanelQuoteChip}>{app.quote}</span>
                <button class={styles.wsPanelCreateBtn} onclick={handleCreateWorkspace} disabled={createLoading || !newBase.trim()}>
                    {#if createLoading}
                        <span class={styles.wavingDots}><span class={styles.wavingDot}></span><span class={styles.wavingDot}></span><span class={styles.wavingDot}></span></span>
                    {:else}+{/if}
                </button>
            </div>
            {#if createError}<div class={styles.wsPanelError}>{createError}</div>{/if}
            <div class={styles.wsPanelList}>
                {#if wsLoading}
                    <div class={styles.wsPanelEmpty}>Loading workspaces…</div>
                {:else if wsInstances.length === 0}
                    <div class={styles.wsPanelEmpty}>No active workspaces. Create one above.</div>
                {:else}
                    {#each wsInstances as inst (inst.id)}
                        {@const pk = inst.pair}
                        {@const chg = changeStr(pk)}
                        <div class={styles.wsPanelRow} onclick={() => { app.enterInstance(pk); closeWorkspacePanel(); }}>
                            <div class={styles.wsPanelPair}>
                                <span class="{styles.statusDot} {statusClass(inst.status)}"></span>
                                <span class={styles.wsPanelSym}>{pairDisplay(pk)}</span>
                                <span class={styles.wsPanelPrice}>{priceFor(pk)}</span>
                                {#if chg}
                                    <span class="{styles.change} {changeCls(chg)}">{chg}</span>
                                {/if}
                            </div>
                            <div class={styles.wsPanelActionBtn} title="Pause" onclick={(e) => { e.stopPropagation(); requestRowConfirm(inst.id, 'pause'); }}>
                                {#if rowConfirm?.id === inst.id && rowConfirm?.action === 'pause'}
                                    <div class={styles.confirmRow}>
                                        <button class={styles.confirmBtn} onclick={(e) => { e.stopPropagation(); cancelRowConfirm(); }}>Cancel</button>
                                        <button class={styles.confirmBtn} onclick={(e) => { e.stopPropagation(); executeRowConfirm(); }}>Pause</button>
                                    </div>
                                {:else}⏸{/if}
                            </div>
                            <div class="{styles.wsPanelActionBtn} {styles.danger}" title="Delete" onclick={(e) => { e.stopPropagation(); requestRowConfirm(inst.id, 'delete', pk); }}>
                                {#if rowConfirm?.id === inst.id && rowConfirm?.action === 'delete'}
                                    <div class={styles.confirmRow}>
                                        <button class={styles.confirmBtn} onclick={(e) => { e.stopPropagation(); cancelRowConfirm(); }}>Cancel</button>
                                        <button class="{styles.confirmBtn} {styles.confirmBtnDanger}" onclick={(e) => { e.stopPropagation(); executeRowConfirm(); }}>Delete</button>
                                    </div>
                                {:else}🗑{/if}
                            </div>
                        </div>
                    {/each}
                {/if}
            </div>
        </div>
    {/if}

    {#if showQuitDialog}<QuitDialog onclose={() => showQuitDialog = false} />{/if}
{/if}
