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
    import DataInfraDashboard from './components/DataInfraDashboard.svelte';
    import EngineOverview from './components/EngineOverview.svelte';
    import PerformanceDashboard from './components/PerformanceDashboard.svelte';
    import TradeAutomationDashboard from './components/TradeAutomationDashboard.svelte';
    import PortfolioDashboard from './components/PortfolioDashboard.svelte';
    import ExchangeSettings from './components/ExchangeSettings.svelte';
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
    import { buildEngineHash, parseEngineHash } from './lib/router.svelte';
    import { getIcon } from './lib/icons';

    const app = useAppStore();
    const wsState: WsState = createWsState();

    // ─── URL hash sync for right-click "open in new tab" ─────────────────
    let restoringFromHash = $state(false);

    function currentHash(): string {
        const pair = app.selectedInstance ? app.instancesMap[app.selectedInstance] : undefined;
        return buildEngineHash(
            app.currentEngine,
            app.currentEngine === 'exchange_settings' ? undefined : app.middleTab,
            app.selectedInstance ?? undefined,
            pair?.currentView !== 'terminal' ? pair?.currentView : undefined,
        );
    }

    function applyRoute(engine: string, middleTab?: string, instance?: string, view?: string) {
        restoringFromHash = true;
        const e = engine as EngineKey;
        app.selectEngine(e);
        if (middleTab) app.middleTab = middleTab;
        if (instance) {
            if (!app.instancesMap[instance]) {
                const base = instance.includes('-') ? instance.split('-')[0] : instance;
                app.initInstance(base);
            }
            app.selectedInstance = instance;
            app.activeTab = instance;
            app.activeEngineTab = 'instance';
            const p = app.instancesMap[instance];
            if (p) p.currentView = (view as CurrentView) ?? 'terminal';
        }
        setTimeout(() => { restoringFromHash = false; }, 50);
    }

    function handleNavClick(e: MouseEvent) {
        if (e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey) return;
        e.preventDefault();
        const anchor = e.currentTarget as HTMLAnchorElement;
        const href = anchor.getAttribute('href');
        if (!href) return;
        const route = parseEngineHash(href);
        if (!route) return;
        applyRoute(route.engine, route.middleTab, route.instance, route.view);
    }

    $effect(() => {
        if (restoringFromHash || !configReady || !app.sessionActive) return;
        const hash = currentHash();
        if (window.location.hash !== hash) {
            history.replaceState(null, '', hash);
        }
    });

    onMount(async () => {
        app.fetchSessionStatus();
        await fetchConfig();
        const route = parseEngineHash(window.location.hash);
        if (route && configReady && app.sessionActive) {
            applyRoute(route.engine, route.middleTab, route.instance, route.view);
        } else if (app.sessionActive) {
            history.replaceState(null, '', currentHash());
        }
        window.addEventListener('hashchange', () => {
            const r = parseEngineHash(window.location.hash);
            if (r) applyRoute(r.engine, r.middleTab, r.instance, r.view);
        });
    });
    let showQuitDialog = $state(false);
    let isSidebarOpen = $state(false);
    let isWorkspacePanelOpen = $state(false);
    let confirmModal = $state<{ action: 'start' | 'pause' | 'stop' | 'delete'; id: string; pair?: string } | null>(null);

    // ─── Workspace panel data ───────────────────────────────────────────
    interface InstanceRow { id: string; pair: string; symbol: string; status: string; }
    let wsInstances = $state<InstanceRow[]>([]);
    let wsLoading = $state(false);
    let newBase = $state('');
    let createLoading = $state(false);
    let createError = $state<string | null>(null);

    // ─── Engines ────────────────────────────────────────────────────────
    type EngineKey = 'profile' | 'data_infra' | 'market_monitor' | 'trade_automation' | 'portfolio' | 'performance' | 'exchange_settings';
    const ENGINES_SIDEBAR: { key: EngineKey; label: string; divider?: boolean }[] = [
        { key: 'data_infra',        label: 'Data Infrastructure' },
        { key: 'market_monitor',    label: 'Market Monitoring' },
        { key: 'trade_automation',  label: 'Trade Automation' },
        { key: 'portfolio',         label: 'Portfolio Management' },
        { key: 'performance',       label: 'Performance Analytics' },
        { key: 'exchange_settings', label: 'Exchange API Keys', divider: true },
    ];

    const MIDDLE_TABS: { key: string; label: string }[] = [
        { key: 'overview',  label: 'Overview' },
        { key: 'settings',  label: 'Settings' },
    ];

    const MARKET_TABS: { key: string; label: string }[] = [
        { key: 'overview',  label: 'Overview' },
        { key: 'workspace', label: 'Workspace' },
        { key: 'settings',  label: 'Settings' },
    ];

    const activeMiddleTabs = $derived(app.currentEngine === 'market_monitor' ? MARKET_TABS : MIDDLE_TABS);

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
    const isSimplePage = $derived(app.currentEngine === 'exchange_settings');

    // ─── Top label ──────────────────────────────────────────────────────
    const topLabel = $derived(isHome ? 'TRADING PLATFORM' : engineLabel(app.currentEngine));

    // ─── 24h change for selected pair ───────────────────────────────────
    const livePrice = $derived.by(() => {
        if (!activePair) return '--';
        const tfs = [activePair.microTerm, activePair.fastTerm, activePair.slowTerm, activePair.macroTerm];
        for (const tf of tfs) {
            const p = tf?.priceText;
            if (p && p !== '0' && p !== 'NaN' && parseFloat(p) > 0) {
                const snap = tf?.latestSnapshot;
                if (snap) {
                    const age = (Date.now() / 1000) - ((snap as Record<string, unknown>).timestamp as number);
                    if (age < 30) return p;
                }
            }
        }
        return activePair.microTerm.priceText || '--';
    });
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
        const tfs = [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm];
        for (const tf of tfs) {
            const snap = tf?.latestSnapshot;
            if (!snap) continue;
            const mid = parseFloat(String((snap as Record<string, unknown>).mid_price ?? ''));
            const prev = parseFloat(String((snap as Record<string, unknown>).prev_day_px ?? ''));
            if (!isFinite(mid) || !isFinite(prev) || prev === 0) continue;
            const age = (Date.now() / 1000) - ((snap as Record<string, unknown>).timestamp as number);
            if (age < 60) {
                const v = ((mid - prev) / prev) * 100;
                return (v > 0 ? '+' : '') + v.toFixed(2) + '%';
            }
        }
        return '';
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

    function requestRowConfirm(id: string, action: 'start' | 'stop' | 'pause' | 'delete', pair?: string) { confirmModal = { id, action, pair }; }

    async function executeRowConfirm() {
        if (!confirmModal) return;
        const { id, action, pair } = confirmModal;
        confirmModal = null;
        if (action === 'delete') {
            try {
                await fetch(`/api/instances/${encodeURIComponent(id)}`, { method: 'DELETE' });
                if (pair) { app.removeInstance(pair); if (app.selectedInstance === pair) app.exitInstance(); }
            } catch (_) {}
        } else {
            try {
                await fetch(`/api/instances/${encodeURIComponent(id)}/${action}`, { method: 'POST' });
            } catch (_) {}
        }
        await fetchWorkspaces(); await app.fetchSessionStatus();
    }

    function pairDisplay(pairKey: string): string {
        return pairKey.replace('-', '/');
    }

    function priceFor(pairKey: string): string {
        const inst = app.instancesMap[pairKey];
        if (!inst) return '--';
        const tfs = [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm];
        for (const tf of tfs) {
            const p = tf?.priceText;
            if (p && p !== '0' && p !== 'NaN' && parseFloat(p) > 0) return p;
        }
        return inst.microTerm?.priceText || '--';
    }

    function statusClass(status: string): string {
        switch (status) { case 'running': return styles.statusRunning; case 'paused': return styles.statusPaused; case 'stopped': return styles.statusStopped; default: return styles.statusStopped; }
    }

    function engineLabel(key: string): string {
        if (key === 'exchange_settings') return 'EXCHANGE API KEYS';
        return ENGINES_SIDEBAR.find(e => e.key === key)?.label?.toUpperCase() ?? 'COMING SOON';
    }

    function sidebarItemClass(key: EngineKey): string {
        const base = styles.sidebarItem;
        return app.currentEngine === key ? `${base} ${styles.sidebarItemActive}` : base;
    }

    function navigateTo(engine: EngineKey) { app.selectEngine(engine); closeSidebar(); }

    function sidebarSvg(key: EngineKey): string {
        const map: Record<EngineKey, string> = {
            profile: 'home',
            data_infra: 'database',
            market_monitor: 'trend',
            trade_automation: 'cycle',
            portfolio: 'dollar',
            performance: 'search',
            exchange_settings: 'key',
        };
        return getIcon(map[key] || 'home', 15);
    }
</script>

{#if !app.sessionChecked}
    <div class={styles.loading}><div class={styles.spinner}></div><span>Connecting to Trading Platform…</span></div>
{:else if !app.sessionActive}
    <WelcomeGate />
{:else}
    <div class={styles.gridContainer}>

        <!-- Navbar 1: Top bar -->
        <header class="{styles.row} {styles.rowNavbar}">
            <div class="{styles.cell} {styles.cellBrand} {styles.cellNavbar} {styles.cellClickable}" role="button" tabindex="0" onclick={toggleSidebar} onkeydown={(e) => e.key === 'Enter' && toggleSidebar()}>
                <span class={styles.navIcon}>{@html getIcon('menu', 16)}</span>
                {topLabel}
                <span class={styles.brandChevron}>
                    {@html getIcon('chevronRight', 8)}
                </span>
            </div>
            <div class="{styles.cell} {styles.cellMono} {styles.cellNavbar}" style="justify-content: flex-start;">
                <span class={styles.exchangeChip}>{app.sessionExchange} · {app.sessionCurrency}</span>
            </div>
            <div class="{styles.cell} {styles.cellNavbar}"></div>
            <div class="{styles.cell} {styles.cellNavbar} {styles.cellClickable} {isWorkspacePanelOpen ? styles.cellActive : ''}" role="button" tabindex="0" onclick={openWorkspacePanel} onkeydown={(e) => e.key === 'Enter' && openWorkspacePanel()}>
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
                        <span class={styles.navIcon}>{@html getIcon('grid', 14)}</span>
                        Instances
                    </span>
                {/if}
            </div>
        </header>

        <!-- Navbar 2: Middle tabs -->
        {#if !isHome && !isSimplePage}
            <nav class="{styles.row} {styles.rowTabs}">
                {#each activeMiddleTabs as tab (tab.key)}
                    <a href={buildEngineHash(app.currentEngine, tab.key)} class="{styles.cell} {styles.tabCellFill} {styles.cellClickable} {app.middleTab === tab.key ? styles.cellActive : ''}" onclick={(e) => { handleNavClick(e); app.middleTab = tab.key; }}>
                        {tab.label}
                    </a>
                {/each}
            </nav>
        {/if}

        <!-- Navbar 3: Sub-tabs -->
        {#if app.currentEngine === 'market_monitor' && app.middleTab === 'workspace' && app.selectedInstance && activePair}
            <nav class="{styles.row} {styles.rowTabs} {styles.rowSubTabs}">
                {#each SUB_TABS as tab (tab.view)}
                    <a href={buildEngineHash('market_monitor', 'workspace', app.selectedInstance!, tab.view)} class="{styles.cell} {styles.tabCellFill} {styles.cellClickable} {app.activeEngineTab === 'instance' && activePair.currentView === tab.view ? styles.cellActive + ' ' + styles.cellActiveUnderline : ''}" onclick={(e) => { handleNavClick(e); selectSubView(tab.view); }}>
                        {tab.label}
                    </a>
                {/each}
            </nav>
        {/if}

        <!-- Content -->
        <main class={styles.contentArea}>
            {#if app.currentEngine === 'profile'}
                <GeneralSettings />
            {:else if app.currentEngine === 'data_infra'}
                {#if app.middleTab === 'overview'}
                    <DataInfraDashboard />
                {:else}
                    <div class={styles.profileCard} style="padding:2rem">
                        <h3>Data Infrastructure Settings</h3>
                        <p class={styles.cardSub}>Exchange endpoints and NTP clock monitor configuration.</p>
                        <p class={styles.cardSub}>Edit <code>config.toml</code> → <code>[hyperliquid]</code>, <code>[bitget]</code>, <code>[clock_monitor]</code> sections directly. Restart the daemon after changes.</p>
                    </div>
                {/if}
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
            {:else if app.currentEngine === 'performance'}
                {#if app.middleTab === 'overview'}
                    <PerformanceDashboard />
                {:else}
                    <div class={styles.profileCard} style="padding:2rem">
                        <h3>Performance Analytics Settings</h3>
                        <p class={styles.cardSub}>Configure analytics execution cadences and optimizer intervals in <code>config.toml</code> → <code>[workspace]</code> → <code>eval_interval_secs</code> and <code>optimizer_interval_secs</code>.</p>
                    </div>
                {/if}
            {:else if app.currentEngine === 'trade_automation'}
                {#if app.middleTab === 'overview'}
                    <TradeAutomationDashboard />
                {:else}
                    <div class={styles.profileCard} style="padding:2rem">
                        <h3>Trade Automation Settings</h3>
                        <p class={styles.cardSub}>Configure execution policies, trigger modes, risk parameters, and paper/live trading adapter settings in <code>config.toml</code> → <code>[execution_engine]</code>. Edit policy files in <code>config/policies/</code>.</p>
                    </div>
                {/if}
            {:else if app.currentEngine === 'portfolio'}
                {#if app.middleTab === 'overview'}
                    <PortfolioDashboard />
                {:else}
                    <div class={styles.profileCard} style="padding:2rem">
                        <h3>Portfolio Management Settings</h3>
                        <p class={styles.cardSub}>Configure safety thresholds, fee rates, leverage caps, concentration limits, and drawdown enforcement in <code>config.toml</code> → <code>[portfolio]</code>. Edit risk profiles in <code>config/</code>.</p>
                    </div>
                {/if}
            {:else if app.currentEngine === 'exchange_settings'}
                <ExchangeSettings />
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
                    {#if engine.divider}
                        <div class={styles.sidebarDivider}></div>
                    {/if}
                    <a href={buildEngineHash(engine.key)} class={sidebarItemClass(engine.key)} onclick={(e) => { handleNavClick(e); navigateTo(engine.key); }}>
                        <span class={styles.navIcon}>{@html sidebarSvg(engine.key)}</span>{engine.label}
                    </a>
                {/each}
            </div>
            <div class={styles.sidebarFooter}>
                <button class={styles.sidebarQuitBtn} onclick={() => { closeSidebar(); showQuitDialog = true; }}>
                    <span class={styles.navIcon}>{@html getIcon('logout', 14)}</span>
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
                    <span class={styles.navIcon}>{@html getIcon('grid', 14)}</span>
                    Instances
                </div>
                <button class={styles.wsPanelClose} onclick={closeWorkspacePanel}>{@html getIcon('x', 16)}</button>
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
                    <div class={styles.wsPanelEmpty}>Loading instances…</div>
                {:else if wsInstances.length === 0}
                    <div class={styles.wsPanelEmpty}>No active instances. Create one above.</div>
                {:else}
                    {#each wsInstances as inst (inst.id)}
                        {@const pk = inst.pair}
                        {@const chg = changeStr(pk)}
                        <a href={buildEngineHash('market_monitor', 'workspace', pk)} class={styles.wsPanelRow} onclick={(e) => { handleNavClick(e); app.enterInstance(pk); closeWorkspacePanel(); }}>
                            <div class={styles.wsPanelPair}>
                                <span class="{styles.statusDot} {statusClass(inst.status)}"></span>
                                <span class={styles.wsPanelSym}>{pairDisplay(pk)}</span>
                                <span class={styles.wsPanelPrice}>{priceFor(pk)}</span>
                                {#if chg}
                                    <span class="{styles.change} {changeCls(chg)}">{chg}</span>
                                {/if}
                            </div>
                            <div class={styles.wsPanelActionBtn} title="Pause" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); requestRowConfirm(inst.id, 'pause'); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); requestRowConfirm(inst.id, 'pause'); } }}>{@html getIcon('pause', 12)}</div>
                            <div class="{styles.wsPanelActionBtn} {styles.start}" title="Start" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); requestRowConfirm(inst.id, 'start'); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); requestRowConfirm(inst.id, 'start'); } }}>{@html getIcon('play', 12)}</div>
                            <div class="{styles.wsPanelActionBtn} {styles.stop}" title="Stop" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); requestRowConfirm(inst.id, 'stop'); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); requestRowConfirm(inst.id, 'stop'); } }}>{@html getIcon('stop', 12)}</div>
                            <div class="{styles.wsPanelActionBtn} {styles.danger}" title="Delete" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); requestRowConfirm(inst.id, 'delete', pk); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); requestRowConfirm(inst.id, 'delete', pk); } }}>{@html getIcon('trash', 12)}</div>
                        </a>
                    {/each}
                {/if}
            </div>
        </div>
    {/if}

    {#if confirmModal}
        {@const actionLabels: Record<string, string> = { start: 'Start', pause: 'Pause', stop: 'Stop', delete: 'Delete' }}
        {@const actionLabel = actionLabels[confirmModal.action] ?? confirmModal.action}
        {@const isDelete = confirmModal.action === 'delete'}
        {@const displaySymbol = confirmModal.pair ? pairDisplay(confirmModal.pair) : 'this instance'}
        <div class={styles.confirmOverlay} role="presentation" onclick={closeConfirmModal}>
            <div class={styles.confirmDialog} role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
                <div class={styles.confirmIcon}>
                    {#if isDelete}
                        {@html getIcon('x', 32)}
                    {:else}
                        {@html getIcon('info', 32)}
                    {/if}
                </div>
                <h2 class={styles.confirmTitle}>{actionLabel} {displaySymbol}?</h2>
                <p class={styles.confirmText}>
                    {#if isDelete}
                        This will permanently delete <strong>{displaySymbol}</strong> and all associated data.
                    {:else if confirmModal.action === 'start'}
                        This will start the <strong>{displaySymbol}</strong> instance.
                    {:else if confirmModal.action === 'stop'}
                        This will stop the <strong>{displaySymbol}</strong> instance.
                    {:else}
                        This will pause the <strong>{displaySymbol}</strong> instance. It can be resumed later.
                    {/if}
                </p>
                <div class={styles.confirmActions}>
                    <button class={styles.confirmCancelBtn} onclick={closeConfirmModal}>Cancel</button>
                    <button class={styles.confirmDangerBtn} onclick={executeRowConfirm}
                        style={isDelete ? 'background:#ef5350;color:#fff;border:none' : ''}>
                        {actionLabel}
                    </button>
                </div>
            </div>
        </div>
    {/if}

    {#if showQuitDialog}<QuitDialog onclose={() => showQuitDialog = false} />{/if}
{/if}
