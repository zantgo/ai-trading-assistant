<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { createInstance } from '../lib/api.svelte';
    import type { InstanceState, PositionScalingConfig } from '../types';
    import ExchangeSettings from './ExchangeSettings.svelte';
    import IndicatorWeightPanel from './settings/IndicatorWeightPanel.svelte';
    import ScoringWeightsPanel from './settings/ScoringWeightsPanel.svelte';
    import PositionScalingPanel from './settings/PositionScalingPanel.svelte';
    import TriggerConfigPanel from './settings/TriggerConfigPanel.svelte';
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
            showEmas: true, showBb: true, showVwap: true, showVolume: true,
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
        apiKey: '' as string,
        rules: '' as string,
    });

    let apiKeyStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');
    let apiKeyError = $state('');
    let rulesStatus = $state<'idle' | 'loading' | 'saving' | 'success' | 'error'>('idle');
    let draftCostInputPrice = $state(app.costPriceInput);
    let draftCostOutputPrice = $state(app.costPriceOutput);
    let costSaveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    let weightOverrides = $state<Record<string, number>>({});
    let positionScaling = $state<PositionScalingConfig | null>(null);
    let aiTriggerConfig = $state<{ trigger: import('../types').TriggerModeConfig } | null>(null);
    let operationalMode = $state<import('../types').OperationalMode>('HybridAiCopilot');

    $effect(() => {
        draft.symbol = pair.symbol; draft.exchange = pair.exchange;
        draft.analysisLimit = pair.microTerm.analysisLimit;
        for (const f of ['showEmas','showBb','showVwap','showVolume','showAdx','showAtr','showRsi','showMacd','showSqueeze','showBbwp','showFib','showRvol','showStochastic','showChandeMo','showSupertrend','showKeltner','showDonchian','showObv','showCmf','showMfi','showHv','showAroon','showChoppiness','showLinregSlope','showZscore']) {
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

    async function saveCostConfig() {
        costSaveStatus = 'saving';
        try {
            const res = await fetch('/api/config/costs', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    price_per_1m_input_tokens: Number(draftCostInputPrice),
                    price_per_1m_output_tokens: Number(draftCostOutputPrice),
                }),
            });
            costSaveStatus = res.ok ? 'success' : 'error';
            if (res.ok) {
                app.costPriceInput = Number(draftCostInputPrice);
                app.costPriceOutput = Number(draftCostOutputPrice);
                setTimeout(() => { costSaveStatus = 'idle'; }, 2000);
            }
        } catch (_) {
            costSaveStatus = 'error';
        }
    }

    async function saveApiKey() {
        const key = draft.apiKey.trim();
        if (!key) return;
        apiKeyStatus = 'saving';
        try {
            const res = await fetch('/api/config/key', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ api_key: key }),
            });
            if (res.ok) {
                app.apiKeyConfigured = true;
                draft.apiKey = '';
                apiKeyStatus = 'success';
                setTimeout(() => { apiKeyStatus = 'idle'; }, 2000);
            } else {
                apiKeyError = 'Rejected by Server';
                apiKeyStatus = 'error';
            }
        } catch (e: any) {
            apiKeyError = e.message || 'Connection failed';
            apiKeyStatus = 'error';
        }
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
            showEmas: vis.showEmas, showBb: vis.showBb, showVwap: vis.showVwap,
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

    let aiConfigSaveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    async function saveAiConfig() {
        aiConfigSaveStatus = 'saving';
        try {
            const payload: Record<string, unknown> = {
                operational_mode: operationalMode,
                weight_overrides: weightOverrides && Object.keys(weightOverrides).length > 0 ? weightOverrides : null,
                position_scaling: positionScaling,
                ai_trigger: aiTriggerConfig,
            };
            const res = await fetch(`/api/instances/${encodeURIComponent(tabKey)}/config`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload),
            });
            aiConfigSaveStatus = res.ok ? 'success' : 'error';
            if (res.ok) {
                setTimeout(() => { aiConfigSaveStatus = 'idle'; }, 2000);
            }
        } catch (_) {
            aiConfigSaveStatus = 'error';
        }
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
                </div>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Indicator Panels</span>
                <div class={styles.toggleGrid}>
                    {#each panelToggles as [key, lbl]}
                        <button class="{styles.selectorBtn} {(draft.visuals as any)[key] ? styles.active : ''}" onclick={() => (draft.visuals as any)[key] = !(draft.visuals as any)[key]}>{lbl}</button>
                    {/each}
                </div>
            </div>

            <div style="margin-top: 12px;">
                <ScoringWeightsPanel />
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>AI & Automation</span>
                <div class={styles.toggleRow}>
                    <span class={styles.toggleLabel}>Status</span>
                    <button class="{styles.selectorBtn} {draft.automation.enabled ? styles.active : ''}"
                            onclick={() => draft.automation.enabled = !draft.automation.enabled}>
                        {draft.automation.enabled ? 'ON' : 'OFF'}
                    </button>
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="opMode">Operational Mode:</label>
                    <select id="opMode" bind:value={operationalMode} class={styles.tfUnitSelect}>
                        <option value="HybridAiCopilot">AI Copilot</option>
                        <option value="DeterministicHeuristics">Heuristics Only</option>
                        <option value="ManualOnly">Manual Only</option>
                    </select>
                </div>
                <p style="font-size: 8px; color: #64748b; margin: 4px 0 0 0;">
                    AI Copilot: full LLM pipeline. Heuristics Only: local indicators, no AI calls. Manual Only: local indicators + on-demand AI via sidebar.
                </p>
                {#if draft.automation.enabled}
                    <div class={styles.inputRow} style="margin-top: 8px;">
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
                    <div class={styles.liveCounter} style="margin-top: 8px; font-size: 10px; color: #3b82f6;">
                        Next evaluation in: {pair.nextEvaluationIn}
                    </div>
                {/if}
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Identity</span>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="exchange">Exchange Source:</label>
                    <select id="exchange" bind:value={draft.exchange} class={styles.tfUnitSelect}>
                        <option value="Hyperliquid">Hyperliquid</option>
                    </select>
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="symbol">Market Pair:</label>
                    <input id="symbol" type="text" bind:value={draft.symbol} />
                </div>
            </div>

            <div class="settings-footer-row" style="margin-top: 16px;">
                <button class={styles.applyWorkspaceBtn} onclick={applySettings}>
                    Apply Workspace Configuration
                </button>
            </div>
            {#if identityError}
                <div class={styles.identityError} role="alert">⚠ {identityError}</div>
            {/if}
        </div>

        <!-- Backend Secrets & Prompts Guide Column -->
        <div class={styles.settingsCol}>
            <h3 class={styles.cardTitle}>Backend & AI Prompts</h3>

            <!-- API Key Config -->
            <div class={styles.settingGroupBox}>
                <span class={styles.selectorsLabel}>DeepSeek API Secret Key</span>
                <div class={styles.keyInputRow}>
                    <input type="password" class={styles.keyField} placeholder="sk-..." bind:value={draft.apiKey} />
                    <button class={styles.keySaveBtn} disabled={apiKeyStatus === 'saving'} onclick={saveApiKey}>
                        {apiKeyStatus === 'saving' ? '...' : 'Save'}
                    </button>
                </div>
                {#if apiKeyStatus === 'success'}
                    <div class="{styles.statusMsg} {styles.successMsg}">Key saved.</div>
                {/if}
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <div class={styles.inputRow}>
                    <label for="wsAnalysisLimit">AI Analysis Lookback (Candles):</label>
                    <input id="wsAnalysisLimit" type="number" bind:value={draft.analysisLimit} min="10" max="500" step="5" />
                </div>
            </div>

            <!-- Indicator Weight Overrides -->
            <div style="margin-top: 12px;">
                <IndicatorWeightPanel initial={weightOverrides} onchange={(w) => { weightOverrides = w; }} />
            </div>

            <!-- Position Sizing & Leverage -->
            <div style="margin-top: 12px;">
                <PositionScalingPanel initial={positionScaling} onchange={(c) => { positionScaling = c; }} />
            </div>

            <!-- AI Trigger Configuration -->
            <div style="margin-top: 12px;">
                <TriggerConfigPanel initial={aiTriggerConfig} onchange={(c) => { aiTriggerConfig = c; }} />
            </div>

            <button class={styles.keySaveBtn} style="margin-top: 8px; width: 100%;"
                    disabled={aiConfigSaveStatus === 'saving'} onclick={saveAiConfig}>
                {aiConfigSaveStatus === 'saving' ? 'Saving...' : 'Save AI Configuration'}
            </button>
            {#if aiConfigSaveStatus === 'success'}
                <div class="{styles.statusMsg} {styles.successMsg}">AI config saved.</div>
            {/if}

            <!-- Rules Editor -->
            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Technical rules guide handbook (Markdown)</span>
                <textarea class={styles.rulesEditor} rows="6" bind:value={draft.rules}></textarea>
                <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 4px;">
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
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="paperUSD">Initial USD:</label>
                    <input id="paperUSD" type="number" bind:value={app.paperInitialUSD} min="100" step="100" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperAlloc">Allocation %:</label>
                    <input id="paperAlloc" type="number" bind:value={app.paperAllocationPct} min="1" max="100" step="1" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperMaxRisk">Max Risk %:</label>
                    <input id="paperMaxRisk" type="number" bind:value={app.paperMaxRiskPct} min="0.5" max="10" step="0.1" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperLeverage">Leverage:</label>
                    <input id="paperLeverage" type="number" bind:value={app.paperLeverage} min="1" max="20" step="1" />
                </div>
                <button class={styles.keySaveBtn} style="margin-top: 8px; width: 100%;"
                        onclick={() => app.savePaperConfig(
                            app.paperInitialUSD,
                            app.paperAllocationPct,
                            app.paperAutoExecute
                        )}>
                    Save Paper Config
                </button>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>AI Orchestrator Settings</span>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="paperInterval">Eval Interval (min):</label>
                    <input id="paperInterval" type="number" bind:value={app.paperAutoExecuteIntervals} min="1" max="1440" step="1" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperLookback">Lookback Trades:</label>
                    <input id="paperLookback" type="number" bind:value={app.paperLookbackTrades} min="1" max="50" step="1" />
                </div>
                <p style="font-size: 9px; color: #64748b; margin: 6px 0 0 0;">
                    Number of past trades fed to the Master Orchestrator for context.
                </p>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
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
                <p style="font-size: 9px; color: #64748b; margin: 6px 0 0 0;">
                    When enabled, automated AI signals will automatically place paper orders.
                </p>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>AI Token Cost Calculator (per 1M tokens)</span>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="costInput">Input Price $/1M:</label>
                    <input id="costInput" type="number" bind:value={draftCostInputPrice} min="0" step="0.01" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="costOutput">Output Price $/1M:</label>
                    <input id="costOutput" type="number" bind:value={draftCostOutputPrice} min="0" step="0.01" />
                </div>
                <button class={styles.keySaveBtn} style="margin-top: 8px; width: 100%;"
                        disabled={costSaveStatus === 'saving'} onclick={saveCostConfig}>
                    {costSaveStatus === 'saving' ? 'Saving...' : 'Save Cost Config'}
                </button>
                {#if costSaveStatus === 'success'}
                    <div class="{styles.statusMsg} {styles.successMsg}">Pricing saved.</div>
                {/if}
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
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
