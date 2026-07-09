<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { createInstance } from '../lib/api.svelte';
    import Icon from '../lib/Icon.svelte';
    import type { InstanceState } from '../types';
    import ExchangeSettings from './ExchangeSettings.svelte';
    import styles from './WorkspaceSettings.module.css';

    let { pair, tabKey }: { pair: InstanceState; tabKey: string } = $props();

    const app = useAppStore();

    let identityError = $state<string | null>(null);

    // Indicator-panel visibility toggles rendered as a grid.
    const panelToggles: Array<[string, string]> = [
        ['showVolume', 'Volume'], ['showAdx', 'ADX'], ['showAtr', 'ATR'], ['showRsi', 'RSI'],
        ['showMacd', 'MACD'], ['showSqueeze', 'Squeeze'], ['showBbwp', 'BBWP'], ['showRvol', 'RVOL'],
        ['showFib', 'Fibonacci'], ['showStochastic', 'STOCH'], ['showChandeMo', 'CHANDE MO'],
        ['showSupertrend', 'SUPERTREND'], ['showKeltner', 'KELTNER'], ['showDonchian', 'DONCHIAN'],
        ['showObv', 'OBV'], ['showCmf', 'CMF'], ['showMfi', 'MFI'], ['showHv', 'HIST VOL'],
        ['showAroon', 'AROON'], ['showChoppiness', 'CHOP'], ['showLinregSlope', 'LINREG'], ['showZscore', 'Z-SCORE'],
    ];

    let draft = $state({
        symbol: '',
        exchange: 'Hyperliquid' as string,
        analysisLimit: 100 as number,
        visuals: {
            showEmas: true, showBb: true, showVwap: true, showAvwap: true, showVolume: true,
            showAdx: true, showAtr: true, showRsi: true, showMacd: true,
            showSqueeze: true, showBbwp: true, showFib: true,
            showRvol: true, showStochastic: true, showChandeMo: true,
            showSupertrend: true, showKeltner: true, showDonchian: true,
            showObv: true, showCmf: true, showMfi: true, showHv: true,
            showAroon: true, showChoppiness: true, showLinregSlope: true, showZscore: true,
        },
        automation: {
            enabled: false as boolean,
            intervalValue: 15 as number,
            intervalUnit: 'minutes' as 'seconds' | 'minutes' | 'hours',
        },
        rules: '' as string,
    });

    let rulesStatus = $state<'idle' | 'loading' | 'saving' | 'success' | 'error'>('idle');



    $effect(() => {
        draft.symbol = pair.symbol; draft.exchange = pair.exchange;
        draft.analysisLimit = pair.microTerm.analysisLimit;
        for (const f of ['showEmas','showBb','showVwap','showAvwap','showVolume','showAdx','showAtr','showRsi','showMacd','showSqueeze','showBbwp','showFib','showRvol','showStochastic','showChandeMo','showSupertrend','showKeltner','showDonchian','showObv','showCmf','showMfi','showHv','showAroon','showChoppiness','showLinregSlope','showZscore']) {
            (draft.visuals as any)[f] = (pair.microTerm as any)[f];
        }
        draft.automation.enabled = pair.automationEnabled;
        draft.automation.intervalValue = pair.automationIntervalValue;
        draft.automation.intervalUnit = pair.automationIntervalUnit;
    });

    let calculatedAutomationInterval = $derived.by(() => {
        const val = Number(draft.automation.intervalValue) || 1;
        if (draft.automation.intervalUnit === 'hours') return val * 3600;
        if (draft.automation.intervalUnit === 'minutes') return val * 60;
        return val;
    });

    function formatIntervalRemaining(totalSeconds: number): string {
        const h = Math.floor(totalSeconds / 3600);
        const m = Math.floor((totalSeconds % 3600) / 60);
        const s = totalSeconds % 60;
        if (h > 0) return `${h}h ${m.toString().padStart(2, '0')}m`;
        if (m > 0) return `${m}m ${s.toString().padStart(2, '0')}s`;
        return `${s}s`;
    }



    async function fetchRules() {
        rulesStatus = 'loading';
        try {
            const res = await fetch('/api/rules');
            const data = await res.json();
            draft.rules = data.content || '';
            app.rulesContent = draft.rules;
            rulesStatus = 'idle';
        } catch (_) {
            rulesStatus = 'error';
        }
    }

    async function saveRules() {
        rulesStatus = 'saving';
        try {
            const res = await fetch('/api/rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: draft.rules }),
            });
            if (res.ok) {
                app.rulesContent = draft.rules;
                rulesStatus = 'success';
                setTimeout(() => { rulesStatus = 'idle'; }, 2000);
            } else {
                rulesStatus = 'error';
            }
        } catch (_) {
            rulesStatus = 'error';
        }
    }

    function applyVisualsToTerm(term: Record<string, any>, vis: typeof draft.visuals) {
        Object.assign(term, {
            showEmas: vis.showEmas, showBb: vis.showBb, showVwap: vis.showVwap, showAvwap: vis.showAvwap,
            showVolume: vis.showVolume, showAdx: vis.showAdx, showAtr: vis.showAtr,
            showRsi: vis.showRsi, showMacd: vis.showMacd, showSqueeze: vis.showSqueeze,
            showBbwp: vis.showBbwp, showFib: vis.showFib,
            showRvol: vis.showRvol,
            showStochastic: vis.showStochastic, showChandeMo: vis.showChandeMo,
            showSupertrend: vis.showSupertrend, showKeltner: vis.showKeltner, showDonchian: vis.showDonchian,
            showObv: vis.showObv, showCmf: vis.showCmf, showMfi: vis.showMfi, showHv: vis.showHv,
            showAroon: vis.showAroon, showChoppiness: vis.showChoppiness, showLinregSlope: vis.showLinregSlope, showZscore: vis.showZscore,
        });
    }

    async function applySettings() {
        const cleanedSymbol = draft.symbol.trim().toUpperCase();
        identityError = null;
        if (!/^[A-Z0-9]{2,10}$/.test(cleanedSymbol)) {
            identityError = 'Invalid ticker. Must be 2-10 alphanumeric characters.';
            return;
        }

        const { automation: auto, visuals: vis } = draft;
        const isIdentityChanged = cleanedSymbol !== pair.symbol || draft.exchange !== pair.exchange;
        let target = pair;

        if (isIdentityChanged) {
            const newPairKey = app.pairKeyFor(cleanedSymbol);
            const result = await createInstance(cleanedSymbol, app.quote);
            if (!result.ok) {
                identityError = result.error || 'Failed to update instance.';
                return;
            }
            app.initInstance(cleanedSymbol, draft.exchange);
            target = app.instancesMap[newPairKey] || pair;
            app.removeInstance(tabKey);
            app.activeTab = newPairKey;
        }

        for (const tf of [target.microTerm, target.fastTerm, target.slowTerm, target.macroTerm]) {
            applyVisualsToTerm(tf, vis);
            tf.analysisLimit = draft.analysisLimit;
        }

        target.automationEnabled = auto.enabled;
        target.automationIntervalValue = auto.intervalValue;
        target.automationIntervalUnit = auto.intervalUnit;
    }

</script>

<div class="{styles.settingsWorkspaceTab} animate-fade">
    <div class={styles.settingsGrid}>

        <!-- Visual Layout Column -->
        <div class={styles.settingsCol}>
            <h3 class={styles.cardTitle}>Visual Overlays</h3>
            <div class={styles.settingGroupBox}>
                <span class={styles.selectorsLabel}>Chart Display Items</span>
                <div class={styles.toggleGrid}>
                    <button class="{styles.selectorBtn} {draft.visuals.showEmas ? styles.active : ''}" onclick={() => draft.visuals.showEmas = !draft.visuals.showEmas}>EMAs</button>
                    <button class="{styles.selectorBtn} {draft.visuals.showBb ? styles.active : ''}" onclick={() => draft.visuals.showBb = !draft.visuals.showBb}>Bollinger</button>
                    <button class="{styles.selectorBtn} {draft.visuals.showVwap ? styles.active : ''}" onclick={() => draft.visuals.showVwap = !draft.visuals.showVwap}>VWAP</button>
                    <button class="{styles.selectorBtn} {draft.visuals.showAvwap ? styles.active : ''}" onclick={() => draft.visuals.showAvwap = !draft.visuals.showAvwap}>A-VWAP</button>
                </div>
            </div>

            <div class="{styles.settingGroupBox} {styles.spacer12}">
                <span class={styles.selectorsLabel}>Indicator Panels</span>
                <div class={styles.toggleGrid}>
                    {#each panelToggles as [key, lbl]}
                        <button class="{styles.selectorBtn} {(draft.visuals as any)[key] ? styles.active : ''}" onclick={() => (draft.visuals as any)[key] = !(draft.visuals as any)[key]}>{lbl}</button>
                    {/each}
                </div>
            </div>


            <div class="{styles.settingGroupBox} {styles.spacer12}">
                <span class={styles.selectorsLabel}>Automation</span>
                <div class={styles.toggleRow}>
                    <span class={styles.toggleLabel}>Status</span>
                    <button class="{styles.selectorBtn} {draft.automation.enabled ? styles.active : ''}"
                            onclick={() => draft.automation.enabled = !draft.automation.enabled}>
                        {draft.automation.enabled ? 'ON' : 'OFF'}
                    </button>
                </div>
                {#if draft.automation.enabled}
                    <div class="{styles.inputRow} {styles.spacer8}">
                        <label for="autoInterval">Interval:</label>
                        <div class={styles.tfSplitGroup}>
                            <input id="autoInterval" type="number" bind:value={draft.automation.intervalValue} min="1" class={styles.tfNumberInput} />
                            <select bind:value={draft.automation.intervalUnit} class={styles.tfUnitSelect}>
                                <option value="seconds">Seconds</option>
                                <option value="minutes">Minutes</option>
                                <option value="hours">Hours</option>
                            </select>
                        </div>
                    </div>
                    <div class="{styles.liveCounter} {styles.liveCountValue}">
                        Next evaluation in: {pair.nextEvaluationIn}
                    </div>
                {/if}
            </div>

            <div class="{styles.settingGroupBox} {styles.spacer12}">
                <span class={styles.selectorsLabel}>Identity</span>
                <div class="{styles.inputRow} {styles.spacer4}">
                    <label for="exchange">Exchange Source:</label>
                    <select id="exchange" bind:value={draft.exchange} class={styles.tfUnitSelect}>
                        <option value="Hyperliquid">Hyperliquid</option>
                    </select>
                </div>
                <div class="{styles.inputRow} {styles.spacer8}">
                    <label for="symbol">Market Pair:</label>
                    <input id="symbol" type="text" bind:value={draft.symbol} />
                </div>
            </div>

            <div class={styles.spacer16}>
                <button class={styles.applyWorkspaceBtn} onclick={applySettings}>
                    Apply Workspace Configuration
                </button>
            </div>
            {#if identityError}
                <div class={styles.identityError} role="alert"><Icon name="alert" size={12} /> {identityError}</div>
            {/if}
        </div>

        <!-- Backend Secrets & Prompts Guide Column -->
        <div class={styles.settingsCol}>
            <h3 class={styles.cardTitle}>Backend Configuration</h3>

            <!-- Rules Editor -->
            <div class="{styles.settingGroupBox} {styles.spacer12}">
                <span class={styles.selectorsLabel}>Technical rules guide handbook (Markdown)</span>
                <textarea class={styles.rulesEditor} rows="6" bind:value={draft.rules}></textarea>
                <div class={styles.rulesActionBar}>
                    <button class={styles.keySaveBtn} onclick={fetchRules}>Fetch</button>
                    <button class={styles.keySaveBtn} disabled={rulesStatus === 'saving'} onclick={saveRules}>
                        {rulesStatus === 'saving' ? '...' : 'Update Rules'}
                    </button>
                </div>
            </div>
        </div>

        <!-- Paper Trading Rules Column -->
        <div class={styles.settingsCol}>
            <h3 class={styles.cardTitle}>Paper Trading Rules</h3>

            <div class={styles.settingGroupBox}>
                <span class={styles.selectorsLabel}>Account Configuration</span>
                <div class="{styles.inputRow} {styles.spacer4}">
                    <label for="paperUSD">Initial USD:</label>
                    <input id="paperUSD" type="number" bind:value={app.paperInitialUSD} min="100" step="100" />
                </div>
                <div class="{styles.inputRow} {styles.spacer8}">
                    <label for="paperAlloc">Allocation %:</label>
                    <input id="paperAlloc" type="number" bind:value={app.paperAllocationPct} min="1" max="100" step="1" />
                </div>
                <div class="{styles.inputRow} {styles.spacer8}">
                    <label for="paperMaxRisk">Max Risk %:</label>
                    <input id="paperMaxRisk" type="number" bind:value={app.paperMaxRiskPct} min="0.5" max="10" step="0.1" />
                </div>
                <div class="{styles.inputRow} {styles.spacer8}">
                    <label for="paperLeverage">Leverage:</label>
                    <input id="paperLeverage" type="number" bind:value={app.paperLeverage} min="1" max="20" step="1" />
                </div>
                <button class="{styles.keySaveBtn} {styles.spacer8} {styles.fullWidth}"
                        onclick={() => app.savePaperConfig(
                            app.paperInitialUSD,
                            app.paperAllocationPct,
                            app.paperAutoExecute
                        )}>
                    Save Paper Config
                </button>
            </div>


            <div class="{styles.settingGroupBox} {styles.spacer12}">
                <span class={styles.selectorsLabel}>Auto-Execution</span>
                <div class={styles.toggleRow}>
                    <span class={styles.toggleLabel}>Auto-Place Orders</span>
                    <button class="{styles.selectorBtn} {app.paperAutoExecute ? styles.active : ''}"
                            onclick={() => {
                                app.paperAutoExecute = !app.paperAutoExecute;
                                app.savePaperConfig(
                                    app.paperInitialUSD,
                                    app.paperAllocationPct,
                                    app.paperAutoExecute
                                );
                            }}>
                        {app.paperAutoExecute ? 'ON' : 'OFF'}
                    </button>
                </div>
                <p class={styles.helpTextSm}>
                    When enabled, automated signals will automatically place paper orders.
                </p>
            </div>


            <div class="{styles.settingGroupBox} {styles.spacer12}">
                <button class={styles.paperResetBtn} onclick={() => {
                    if (confirm('Reset paper account? This will close any active position and restore initial balance.')) {
                        app.resetPaperAccount();
                    }
                }}>
                    Reset Account Balance
                </button>
            </div>
        </div>

    </div>

    <div class={styles.exchangeContainer}>
        <h3 class={styles.cardTitle}>Exchange Accounts</h3>
        <ExchangeSettings />
    </div>
</div>
