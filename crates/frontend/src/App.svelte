<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from './state.svelte';
    import { createInstance } from './lib/api.svelte';
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

    // ─── Manage Workspaces modal state ──────────────────────────────────
    interface InstanceRow { id: string; pair: string; symbol: string; status: string; }
    let manageInstances = $state<InstanceRow[]>([]);
    let manageLoading = $state(false);
    let actionLoading = $state<Record<string, string>>({});
    let newBase = $state('');
    let createLoading = $state(false);
    let createError = $state<string | null>(null);

    // ─── Engines (Navbar 2) ─────────────────────────────────────────────
    type EngineKey = 'portfolio' | 'market_monitor' | 'trade_automation' | 'performance';
    const ENGINES: { key: EngineKey; label: string }[] = [
        { key: 'portfolio',        label: 'Portfolio' },
        { key: 'market_monitor',   label: 'Market' },
        { key: 'trade_automation', label: 'Trading' },
        { key: 'performance',      label: 'Analysis' },
    ];

    // ─── Sub-tabs (Navbar 3, when inside an instance) ───────────────────
    const SUB_TABS: { view: CurrentView; label: string }[] = [
        { view: 'terminal',   label: 'Live Panel' },
        { view: 'monitor',    label: 'Metrics Panel' },
        { view: 'alignment',  label: 'Alignment' },
        { view: 'risk',       label: 'Risk & Reward' },
        { view: 'analysis',   label: 'Analysis' },
        { view: 'advisory',   label: 'Advisory' },
        { view: 'commission', label: 'Fee Projection' },
        { view: 'settings',   label: 'Workspace Settings' },
    ];

    const activePair = $derived(app.selectedInstance ? app.instancesMap[app.selectedInstance] : undefined);
    const pairKeys = $derived(Object.keys(app.instancesMap));

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

    function closeDropdowns() {
        showInstancesDropdown = false;
        showMenuDropdown = false;
    }

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

    async function handleInstanceAction(id: string, action: 'pause' | 'stop' | 'delete', pair?: string) {
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
        return ENGINES.find(e => e.key === key)?.label ?? 'Coming Soon';
    }

    function actionLabel(id: string, action: 'pause' | 'stop' | 'delete'): string {
        if (actionLoading[id] === action) return '...';
        if (action === 'pause') return '⏸';
        if (action === 'stop') return '⏹';
        return '🗑';
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

        <!-- Navbar 1: Top bar -->
        <header class="{styles.row} {styles.rowNavbar}">
            <div class="{styles.cell} {styles.cellBrand}">Trading Platform</div>
            <div class="{styles.cell} {styles.cellMono}" style="justify-content: flex-start; padding-left: 4px;">
                <span class={styles.exchangeChip}>{app.sessionExchange} · {app.sessionCurrency}</span>
            </div>
            <div class={styles.cell}></div>

            <!-- Workspaces -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.cellClickable} {showInstancesDropdown ? styles.cellActive : ''}"
                onclick={() => { showInstancesDropdown = !showInstancesDropdown; showMenuDropdown = false; }}
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
                            <rect x="3" y="3" width="7" height="7" rx="1"/>
                            <rect x="14" y="3" width="7" height="7" rx="1"/>
                            <rect x="3" y="14" width="7" height="7" rx="1"/>
                            <rect x="14" y="14" width="7" height="7" rx="1"/>
                        </svg>
                        Workspaces
                    </span>
                {/if}

                {#if showInstancesDropdown}
                    <div class={styles.dropdownMenu}>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.isManageModalOpen = true; closeDropdowns(); }}>Manage</button>
                        {#if app.selectedInstance}
                            <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.exitInstance(); closeDropdowns(); }}>Deselect</button>
                        {/if}
                        {#each pairKeys as pKey (pKey)}
                            <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.enterInstance(pKey); closeDropdowns(); }}>{pKey}</button>
                        {/each}
                        {#if pairKeys.length === 0}
                            <span class={styles.dropdownItem}>No Workspaces</span>
                        {/if}
                    </div>
                {/if}
            </div>

            <!-- Menu -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="{styles.cell} {styles.cellClickable} {showMenuDropdown ? styles.cellActive : ''}"
                onclick={() => { showMenuDropdown = !showMenuDropdown; showInstancesDropdown = false; }}
            >
                <span class={styles.navLabel}>
                    <svg class={styles.navIcon} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <line x1="4" y1="6" x2="20" y2="6"/>
                        <line x1="4" y1="12" x2="20" y2="12"/>
                        <line x1="4" y1="18" x2="20" y2="18"/>
                    </svg>
                    Menu
                </span>
                {#if showMenuDropdown}
                    <div class={styles.dropdownMenu}>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.selectEngine('profile'); closeDropdowns(); }}>Profile</button>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.selectEngine('portfolio'); closeDropdowns(); }}>Portfolio</button>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.selectEngine('market_monitor'); closeDropdowns(); }}>Market</button>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.selectEngine('trade_automation'); closeDropdowns(); }}>Trading</button>
                        <button class={styles.dropdownItem} onclick={(e) => { e.stopPropagation(); app.selectEngine('performance'); closeDropdowns(); }}>Analysis</button>
                    </div>
                {/if}
            </div>
        </header>

        <!-- Navbar 2: Engines -->
        <nav class="{styles.row} {styles.rowTabs}">
            {#each ENGINES as engine (engine.key)}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                    class="{styles.cell} {styles.tabCell} {styles.cellClickable} {app.currentEngine === engine.key ? styles.cellActive : ''}"
                    onclick={() => app.selectEngine(engine.key)}
                >
                    {engine.label}
                </div>
            {/each}
        </nav>

        <!-- Navbar 3: Engine tabs (Market Monitor only) -->
        {#if app.currentEngine === 'market_monitor'}
            <nav class="{styles.row} {styles.rowTabs} {styles.rowSubTabs}">
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                    class="{styles.cell} {styles.tabCell} {styles.cellClickable} {app.activeEngineTab === 'overview' ? styles.cellActive : ''}"
                    onclick={() => { app.activeEngineTab = 'overview'; }}
                >
                    Overview
                </div>
                {#if app.selectedInstance && activePair}
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
                {/if}
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
                {#if app.activeEngineTab === 'overview'}
                    <GeneralDashboard />
                {:else if app.activeEngineTab === 'instance' && activePair}
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
                {:else}
                    <GeneralDashboard />
                {/if}
            {:else}
                <div class={styles.placeholder}>
                    <span class={styles.placeholderTitle}>{engineLabel(app.currentEngine)}</span>
                    <span class={styles.placeholderSub}>Coming soon</span>
                </div>
            {/if}
        </main>

    </div>

    <!-- Dropdown click-outside backdrop -->
    {#if showInstancesDropdown || showMenuDropdown}
        <div class={styles.dropdownBackdrop} role="presentation" onclick={closeDropdowns}></div>
    {/if}

    <!-- Manage Workspaces modal -->
    {#if app.isManageModalOpen}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class={styles.backdrop} onclick={() => app.isManageModalOpen = false}>
            <div class={styles.modalWindow} role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
                <div class={styles.modalHeader}>
                    <div class={styles.cell}>::</div>
                    <div class="{styles.cell} {styles.modalTitle}">Manage Workspaces</div>
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div class="{styles.cell} {styles.cellClickable}" onclick={() => app.isManageModalOpen = false}>✕</div>
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
                        {createLoading ? '...' : 'Create'}
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
                                <div class={styles.modalActionCell} onclick={() => handleInstanceAction(inst.id, 'pause')}>
                                    {actionLabel(inst.id, 'pause')}
                                </div>
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                <div class="{styles.modalActionCell} {styles.danger}" onclick={() => handleInstanceAction(inst.id, 'delete', inst.pair)}>
                                    {actionLabel(inst.id, 'delete')}
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
