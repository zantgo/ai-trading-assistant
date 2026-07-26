<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import type { CurrentView } from './types';

    import AppEngineSidebar from './components/layout/AppEngineSidebar.svelte';
    import AppWorkspacePanel from './components/layout/AppWorkspacePanel.svelte';
    import AppPageRouter from './components/layout/AppPageRouter.svelte';
    import AppConfirmModal from './components/layout/AppConfirmModal.svelte';
    import BottomConsole from './components/BottomConsole.svelte';
    import FullscreenChartModal from './components/FullscreenChartModal.svelte';
    import SvgIcon from './lib/SvgIcon.svelte';
    import WelcomeGate from './WelcomeGate.svelte';
    import QuitDialog from './QuitDialog.svelte';

    import styles from './styles/brutalist-grid.module.css';
    import { fetchConfigFromServer, applyConfigToStore, syncInstanceIdsFromList } from './lib/api.svelte';
    import { pickInstanceLivePrice } from './lib/livePrice';
    import {
        connectWsForInstance, disconnectWsForInstance, shouldReconnect,
        type WsState,
    } from './lib/websocket.svelte';
    import { buildEngineHash, parseEngineHash } from './lib/router.svelte';

    const app = useAppStore();
    let wssMap = $state<Record<string, WsState>>({});

    let restoringFromHash = $state(false);
    let configReady = false;
    let isSidebarOpen = $state(false);
    let isWorkspacePanelOpen = $state(false);
    let showQuitDialog = $state(false);
    let confirmModal = $state<{ action: 'start' | 'pause' | 'stop' | 'delete'; id: string; pair?: string } | null>(null);

    type EngineKey = 'profile' | 'data_infra' | 'market_monitor' | 'trade_automation' | 'portfolio' | 'performance' | 'exchange_settings';

    const MARKET_TABS: { key: string; label: string }[] = [
        { key: 'overview',  label: 'Overview' },
        { key: 'workspace', label: 'Workspace' },
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

    const activePair = $derived.by(() => {
        const p = app.selectedInstance ? app.instancesMap[app.selectedInstance] : undefined;
        if (p) void p.currentView;
        return p;
    });
    const isHome = $derived(app.currentEngine === 'profile');
    const topLabel = $derived(isHome ? 'TRADING PLATFORM' : engineLabel(app.currentEngine));

    const livePrice = $derived.by(() => {
        if (!activePair) return '--';
        return pickInstanceLivePrice(
            {
                microTerm: activePair.microTerm,
                fastTerm: activePair.fastTerm,
                slowTerm: activePair.slowTerm,
                macroTerm: activePair.macroTerm,
            },
            Date.now(),
        );
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

    function engineLabel(key: string): string {
        if (key === 'exchange_settings') return 'EXCHANGE API KEYS';
        const map: Record<string, string> = {
            data_infra: 'DATA INFRASTRUCTURE', market_monitor: 'MARKET MONITORING',
            trade_automation: 'TRADE AUTOMATION', portfolio: 'PORTFOLIO MANAGEMENT',
            performance: 'PERFORMANCE ANALYTICS',
        };
        return map[key]?.toUpperCase() ?? 'COMING SOON';
    }

    function selectSubView(view: CurrentView) {
        if (activePair) { activePair.currentView = view; app.activeEngineTab = 'instance'; }
    }

    // ─── Hash routing ──────────────────────────────────────────────────
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
        if (instance && app.instancesMap[instance]) {
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

    // ─── Config & lifecycle ────────────────────────────────────────────
    async function fetchConfig() {
        try {
            const config = await fetchConfigFromServer();
            const { firstSymbol } = applyConfigToStore(app, config);
            if (firstSymbol) app.activeTab = app.pairKeyFor(firstSymbol);
            await syncInstanceIdsFromList(app);
            configReady = true;
            for (const sym of Object.keys(app.instancesMap)) {
                connectWsForInstance(app, wssMap, sym);
            }
        } catch (e) { console.error('Failed to fetch config:', e); configReady = true; }
    }

    onMount(async () => {
        await app.fetchSessionStatus();
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

    onDestroy(() => {
        for (const sym of Object.keys(wssMap)) {
            disconnectWsForInstance(wssMap, sym);
        }
    });

    $effect(() => {
        if (!configReady) return;
        // Bumping `wsVersion` after every config save causes this block to
        // re-run, tearing down stale WS connections and re-attaching them
        // with the new per-slot durations from the store.
        void app.wsVersion;
        // Re-check on every engine navigation so a dropped WS connection
        // (tab-idle, network hiccup) recovers as soon as the user returns
        void app.currentEngine;
        for (const sym of Object.keys(app.instancesMap)) {
            const state = wssMap[sym];
            if (!state || shouldReconnect(app, state, sym)) {
                connectWsForInstance(app, wssMap, sym);
            }
        }
    });

    // ─── Workspace panel confirm actions ───────────────────────────────
    function requestRowConfirm(id: string, action: 'start' | 'stop' | 'pause' | 'delete', pair?: string) {
        confirmModal = { id, action, pair };
    }

    async function executeRowConfirm() {
        if (!confirmModal) return;
        const { id, action, pair } = confirmModal;
        confirmModal = null;
        if (action === 'delete') {
            try {
                await fetch(`/api/instances/${encodeURIComponent(id)}`, { method: 'DELETE' });
                if (pair) { disconnectWsForInstance(wssMap, pair); app.removeInstance(pair); if (app.selectedInstance === pair) app.exitInstance(); }
            } catch (_) {}
        } else {
            try {
                await fetch(`/api/instances/${encodeURIComponent(id)}/${action}`, { method: 'POST' });
            } catch (_) {}
        }
        await app.fetchSessionStatus();
    }

    function closeConfirmModal() { confirmModal = null; }

    function pairDisplay(pairKey: string): string { return pairKey.replace('-', '/'); }
</script>

{#if !app.sessionChecked}
    <div class={styles.loading}><div class={styles.spinner}></div><span>Connecting to Trading Platform…</span></div>
{:else if !app.sessionActive}
    <WelcomeGate />
{:else}
    <div class={styles.gridContainer}>

        <!-- Top bar -->
        <header class="{styles.row} {styles.rowNavbar}">
            <div class="{styles.cell} {styles.cellBrand} {styles.cellNavbar} {styles.cellClickable}" role="button" tabindex="0" onclick={() => isSidebarOpen = !isSidebarOpen} onkeydown={(e) => e.key === 'Enter' && (isSidebarOpen = !isSidebarOpen)}>
                <span class={styles.navIcon}><SvgIcon name="menu" size={16} /></span>
                {topLabel}
                <span class={styles.brandChevron}>
                    <SvgIcon name="chevronRight" size={8} />
                </span>
            </div>
            <div class="{styles.cell} {styles.cellMono} {styles.cellNavbar}" style="justify-content: flex-start;">
                <span class={styles.exchangeChip}>{app.sessionExchange} · {app.sessionCurrency}</span>
            </div>
            <div class="{styles.cell} {styles.cellNavbar}"></div>
            <div class="{styles.cell} {styles.cellNavbar} {styles.cellClickable} {isWorkspacePanelOpen ? styles.cellActive : ''}" role="button" tabindex="0" onclick={() => isWorkspacePanelOpen = true} onkeydown={(e) => e.key === 'Enter' && (isWorkspacePanelOpen = true)}>
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
                        <span class={styles.navIcon}><SvgIcon name="grid" size="sm" /></span>
                        Instances
                    </span>
                {/if}
            </div>
        </header>

        <!-- Middle tabs -->
        {#if app.currentEngine === 'market_monitor'}
            <nav class="{styles.row} {styles.rowTabs}">
                {#each MARKET_TABS as tab (tab.key)}
                    <a href={buildEngineHash(app.currentEngine, tab.key)} class="{styles.cell} {styles.tabCellFill} {styles.cellClickable} {app.middleTab === tab.key ? styles.cellActive : ''}" onclick={(e) => { handleNavClick(e); app.middleTab = tab.key; }}>
                        {tab.label}
                    </a>
                {/each}
            </nav>
        {/if}

        <!-- Sub-tabs (market monitor workspace) -->
        {#if app.currentEngine === 'market_monitor' && app.middleTab === 'workspace' && app.selectedInstance && activePair}
            <nav class="{styles.row} {styles.rowTabs} {styles.rowSubTabs}">
                {#each SUB_TABS as tab (tab.view)}
                    <a href={buildEngineHash('market_monitor', 'workspace', app.selectedInstance!, tab.view)} class="{styles.cell} {styles.tabCellFill} {styles.cellClickable} {app.activeEngineTab === 'instance' && activePair.currentView === tab.view ? styles.cellActive + ' ' + styles.cellActiveUnderline : ''}" onclick={(e) => { handleNavClick(e); selectSubView(tab.view); }}>
                        {tab.label}
                    </a>
                {/each}
            </nav>
        {/if}

        <!-- Page content -->
        <AppPageRouter
            currentEngine={app.currentEngine}
            middleTab={app.middleTab}
            selectedInstance={app.selectedInstance}
            activePair={activePair}
            activeTab={app.activeTab}
        />

        <!-- Bottom Console (Positions / Orders / History / Plan) -->
        {#if app.activeConsoleOpen}
            <section class="{styles.row} {styles.rowConsole}">
                <BottomConsole
                    bind:activeConsoleTab={app.activeConsoleTab}
                />
            </section>
        {/if}
    </div>

    <AppEngineSidebar
        isOpen={isSidebarOpen}
        currentEngine={app.currentEngine}
        onclose={() => isSidebarOpen = false}
        onnavigate={(engine) => { app.selectEngine(engine as EngineKey); isSidebarOpen = false; }}
        onquit={() => showQuitDialog = true}
    />

    <AppWorkspacePanel
        isOpen={isWorkspacePanelOpen}
        wssMap={wssMap}
        onclose={() => isWorkspacePanelOpen = false}
        onrequestConfirm={requestRowConfirm}
    />

    {#if confirmModal}
        <AppConfirmModal
            action={confirmModal.action}
            id={confirmModal.id}
            pair={confirmModal.pair}
            displaySymbol={confirmModal.pair ? pairDisplay(confirmModal.pair) : 'this instance'}
            oncancel={closeConfirmModal}
            onconfirm={executeRowConfirm}
        />
    {/if}

    <FullscreenChartModal />

    {#if showQuitDialog}<QuitDialog onclose={() => showQuitDialog = false} />{/if}
{/if}
