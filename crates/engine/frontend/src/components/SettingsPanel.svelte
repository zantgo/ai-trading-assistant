<script lang="ts">
    import { getState } from '../state.svelte';
    const app = getState();

    let validationError = $state<string | null>(null);

    let tfValue = $state(5);
    let tfUnit = $state<'seconds' | 'minutes' | 'hours'>('seconds');

    let draftSymbol = $state('');
    let draftExchange = $state('Hyperliquid');
    let draftEmaFast = $state(10);
    let draftEmaMedium = $state(50);
    let draftEmaSlow = $state(100);
    let draftEmaLong = $state(200);
    let draftRsiPeriod = $state(14);
    let draftMacdFast = $state(12);
    let draftMacdSlow = $state(26);
    let draftMacdSignal = $state(9);
    let draftAdxPeriod = $state(14);
    let draftAtrPeriod = $state(14);
    let draftSqueezePeriod = $state(20);

    let draftShowEmas = $state(true);
    let draftShowBb = $state(true);
    let draftShowVwap = $state(true);
    let draftShowVolume = $state(true);
    let draftShowAdx = $state(true);
    let draftShowAtr = $state(true);
    let draftShowRsi = $state(true);
    let draftShowMacd = $state(true);
    let draftShowSqueeze = $state(true);

    let draftApiKey = $state('');
    let apiKeyStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');
    let apiKeyError = $state('');

    let showRulesEditor = $state(false);
    let draftRules = $state('');
    let rulesStatus = $state<'idle' | 'loading' | 'saving' | 'success' | 'error'>('idle');

    $effect(() => {
        if (app.showSettingsPanel) {
            validationError = null;

            draftSymbol = app.activeSymbol;
            draftExchange = app.activeExchange;
            draftEmaFast = app.emaFastVal;
            draftEmaMedium = app.emaMediumVal;
            draftEmaSlow = app.emaSlowVal;
            draftEmaLong = app.emaLongVal;
            draftRsiPeriod = app.rsiPeriodVal;
            draftMacdFast = app.macdFastVal;
            draftMacdSlow = app.macdSlowVal;
            draftMacdSignal = app.macdSignalVal;
            draftAdxPeriod = app.adxPeriodVal;
            draftAtrPeriod = app.atrPeriodVal;
            draftSqueezePeriod = app.squeezePeriodVal;

            const duration = app.barDurationSec;
            if (duration % 3600 === 0) {
                tfValue = duration / 3600;
                tfUnit = 'hours';
            } else if (duration % 60 === 0) {
                tfValue = duration / 60;
                tfUnit = 'minutes';
            } else {
                tfValue = duration;
                tfUnit = 'seconds';
            }

            draftShowEmas = app.showEmas;
            draftShowBb = app.showBb;
            draftShowVwap = app.showVwap;
            draftShowVolume = app.showVolume;
            draftShowAdx = app.showAdx;
            draftShowAtr = app.showAtr;
            draftShowRsi = app.showRsi;
            draftShowMacd = app.showMacd;
            draftShowSqueeze = app.showSqueeze;
        }
    });

    let draftDuration = $derived.by(() => {
        const val = Number(tfValue) || 1;
        if (tfUnit === 'hours') return val * 3600;
        if (tfUnit === 'minutes') return val * 60;
        return val;
    });

    let isTechnicalChanged = $derived(
        draftSymbol.trim().toUpperCase() !== app.activeSymbol ||
        draftExchange !== app.activeExchange ||
        draftDuration !== app.barDurationSec ||
        draftEmaFast !== app.emaFastVal ||
        draftEmaMedium !== app.emaMediumVal ||
        draftEmaSlow !== app.emaSlowVal ||
        draftEmaLong !== app.emaLongVal ||
        draftRsiPeriod !== app.rsiPeriodVal ||
        draftMacdFast !== app.macdFastVal ||
        draftMacdSlow !== app.macdSlowVal ||
        draftMacdSignal !== app.macdSignalVal ||
        draftAdxPeriod !== app.adxPeriodVal ||
        draftAtrPeriod !== app.atrPeriodVal ||
        draftSqueezePeriod !== app.squeezePeriodVal
    );

    let isVisualChanged = $derived(
        draftShowEmas !== app.showEmas ||
        draftShowBb !== app.showBb ||
        draftShowVwap !== app.showVwap ||
        draftShowVolume !== app.showVolume ||
        draftShowAdx !== app.showAdx ||
        draftShowAtr !== app.showAtr ||
        draftShowRsi !== app.showRsi ||
        draftShowMacd !== app.showMacd ||
        draftShowSqueeze !== app.showSqueeze
    );

    let isChanged = $derived(isTechnicalChanged || isVisualChanged);

    const closePanel = () => {
        app.showSettingsPanel = false;
    };

    const fetchRules = async () => {
        rulesStatus = 'loading';
        try {
            const res = await fetch('/api/rules');
            const data = await res.json();
            draftRules = data.content || '';
            app.rulesContent = draftRules;
            rulesStatus = 'idle';
        } catch (e) {
            console.error('Failed to fetch rules:', e);
            rulesStatus = 'error';
        }
    };

    const saveRules = async () => {
        rulesStatus = 'saving';
        try {
            const res = await fetch('/api/rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: draftRules }),
            });
            if (res.ok) {
                app.rulesContent = draftRules;
                rulesStatus = 'success';
                setTimeout(() => { rulesStatus = 'idle'; }, 2000);
            } else {
                rulesStatus = 'error';
            }
        } catch (e) {
            console.error('Failed to save rules:', e);
            rulesStatus = 'error';
        }
    };

    const saveApiKey = async () => {
        const key = draftApiKey.trim();
        if (!key) return;

        apiKeyStatus = 'saving';
        apiKeyError = '';
        try {
            const res = await fetch('/api/config/key', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ api_key: key }),
            });
            if (res.ok) {
                app.apiKeyConfigured = true;
                draftApiKey = '';
                apiKeyStatus = 'success';
                setTimeout(() => { apiKeyStatus = 'idle'; }, 2000);
            } else {
                const text = await res.text();
                apiKeyError = text;
                apiKeyStatus = 'error';
            }
        } catch (e: any) {
            apiKeyError = e.message || 'Connection failed';
            apiKeyStatus = 'error';
        }
    };

    const applyVisualsOnly = () => {
        app.showEmas = draftShowEmas;
        app.showBb = draftShowBb;
        app.showVwap = draftShowVwap;
        app.showVolume = draftShowVolume;
        app.showAdx = draftShowAdx;
        app.showAtr = draftShowAtr;
        app.showRsi = draftShowRsi;
        app.showMacd = draftShowMacd;
        app.showSqueeze = draftShowSqueeze;
    };

    const applyChanges = async () => {
        validationError = null;

        const cleanedSymbol = draftSymbol.trim().toUpperCase();
        const tickerRegex = /^[A-Z0-9]{2,10}$/;
        if (!tickerRegex.test(cleanedSymbol)) {
            validationError = "Invalid Pair Name. Ticker must be alphanumeric and 2-10 characters (e.g. ETH, BTC, HYPE).";
            return;
        }

        if (isTechnicalChanged) {
            const body = {
                candles: {
                    duration_seconds: Number(draftDuration)
                },
                indicators: {
                    ema_fast: Number(draftEmaFast),
                    ema_medium: Number(draftEmaMedium),
                    ema_slow: Number(draftEmaSlow),
                    ema_long: Number(draftEmaLong),
                    rsi_period: Number(draftRsiPeriod),
                    macd_fast: Number(draftMacdFast),
                    macd_slow: Number(draftMacdSlow),
                    macd_signal: Number(draftMacdSignal),
                    adx_period: Number(draftAdxPeriod),
                    atr_period: Number(draftAtrPeriod),
                    squeeze_period: Number(draftSqueezePeriod)
                }
            };

            const isIdentityChanged = cleanedSymbol !== app.activeSymbol || draftExchange !== app.activeExchange;

            try {
                if (isIdentityChanged) {
                    const oldPairKey = app.activeTab;
                    const newPairKey = `${draftExchange}-${cleanedSymbol}`;

                    // 1. Add the new pair on the backend
                    const addRes = await fetch(`/api/pairs`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ symbol: cleanedSymbol, exchange: draftExchange }),
                    });
                    if (!addRes.ok) throw new Error('API server failed to add new pair');

                    // 2. Save technical configuration for the new pair key
                    const configRes = await fetch(`/api/pairs/${encodeURIComponent(newPairKey)}/config`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(body)
                    });
                    if (!configRes.ok) throw new Error('API server failed to save configuration for new pair');

                    // 3. Delete old pair on the backend
                    await fetch(`/api/pairs/${encodeURIComponent(oldPairKey)}`, {
                        method: 'DELETE'
                    });

                    // 4. Initialize new pair locally
                    app.initPair(cleanedSymbol, draftExchange);

                    // 5. Apply configuration locally
                    const targetState = app.pairsMap[newPairKey];
                    if (targetState) {
                        targetState.barDurationSec = draftDuration;
                        targetState.emaFastVal = draftEmaFast;
                        targetState.emaMediumVal = draftEmaMedium;
                        targetState.emaSlowVal = draftEmaSlow;
                        targetState.emaLongVal = draftEmaLong;
                        targetState.rsiPeriodVal = draftRsiPeriod;
                        targetState.macdFastVal = draftMacdFast;
                        targetState.macdSlowVal = draftMacdSlow;
                        targetState.macdSignalVal = draftMacdSignal;
                        targetState.adxPeriodVal = draftAdxPeriod;
                        targetState.atrPeriodVal = draftAtrPeriod;
                        targetState.squeezePeriodVal = draftSqueezePeriod;
                    }

                    // 6. Cleanly wipe the old Svelte tab state
                    app.removePair(oldPairKey);

                    // 7. Route the user to the newly updated tab
                    app.activeTab = newPairKey;

                } else {
                    // Identity remains unchanged, update current configuration
                    const pairKey = app.activeTab;
                    const res = await fetch(`/api/pairs/${encodeURIComponent(pairKey)}/config`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(body)
                    });
                    if (!res.ok) throw new Error('API server rejected config save');

                    app.barDurationSec = draftDuration;
                    app.emaFastVal = draftEmaFast;
                    app.emaMediumVal = draftEmaMedium;
                    app.emaSlowVal = draftEmaSlow;
                    app.emaLongVal = draftEmaLong;
                    app.rsiPeriodVal = draftRsiPeriod;
                    app.macdFastVal = draftMacdFast;
                    app.macdSlowVal = draftMacdSlow;
                    app.macdSignalVal = draftMacdSignal;
                    app.adxPeriodVal = draftAdxPeriod;
                    app.atrPeriodVal = draftAtrPeriod;
                    app.squeezePeriodVal = draftSqueezePeriod;
                }

                applyVisualsOnly();
                closePanel();
            } catch (err) {
                validationError = "Save Failed: Unable to contact engine API to write changes.";
                console.error(err);
            }
        } else if (isVisualChanged) {
            applyVisualsOnly();
            closePanel();
        }
    };
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
{#if app.showSettingsPanel}
    <div class="panel-overlay" onclick={closePanel}></div>
{/if}

<div class="panel-drawer" class:panel-open={app.showSettingsPanel}>
    <div class="panel-header">
        <h2 class="panel-title">System Cockpit</h2>
        <button class="panel-close-btn" onclick={closePanel}>✕ Close</button>
    </div>

    <div class="panel-body">
        <div class="panel-section">
            <h3 class="panel-section-title">Visual Layout</h3>

            <div class="setting-group-box">
                <span class="selectors-label">Chart Overlays</span>
                <div class="toggle-grid">
                    <button class="selector-btn" class:active={draftShowEmas} onclick={() => draftShowEmas = !draftShowEmas}>
                        EMAs
                    </button>
                    <button class="selector-btn" class:active={draftShowBb} onclick={() => draftShowBb = !draftShowBb}>
                        BB
                    </button>
                    <button class="selector-btn" class:active={draftShowVwap} onclick={() => draftShowVwap = !draftShowVwap}>
                        VWAP
                    </button>
                </div>
            </div>

            <div class="setting-group-box">
                <span class="selectors-label">Indicators Panels</span>
                <div class="toggle-grid">
                    <button class="selector-btn" class:active={draftShowVolume} onclick={() => draftShowVolume = !draftShowVolume}>
                        Volume
                    </button>
                    <button class="selector-btn" class:active={draftShowAdx} onclick={() => draftShowAdx = !draftShowAdx}>
                        ADX
                    </button>
                    <button class="selector-btn" class:active={draftShowAtr} onclick={() => draftShowAtr = !draftShowAtr}>
                        ATR
                    </button>
                    <button class="selector-btn" class:active={draftShowRsi} onclick={() => draftShowRsi = !draftShowRsi}>
                        RSI
                    </button>
                    <button class="selector-btn" class:active={draftShowMacd} onclick={() => draftShowMacd = !draftShowMacd}>
                        MACD
                    </button>
                    <button class="selector-btn" class:active={draftShowSqueeze} onclick={() => draftShowSqueeze = !draftShowSqueeze}>
                        Squeeze
                    </button>
                </div>
            </div>
        </div>

        <div class="panel-section">
            <h3 class="panel-section-title">Technical Parameters</h3>

            <div class="parameter-inputs-scroll">
                <div class="input-row">
                    <label for="exchange">Exchange Source:</label>
                    <select id="exchange" bind:value={draftExchange} class="tf-unit-select" style="width: 140px; text-align: left;">
                        <option value="Hyperliquid">Hyperliquid</option>
                        <option value="Bybit">Bybit</option>
                        <option value="Coinbase">Coinbase</option>
                        <option value="Kraken">Kraken</option>
                        <option value="Bitget">Bitget</option>
                        <option value="EdgeX">EdgeX</option>
                    </select>
                </div>

                <div class="input-row">
                    <label for="symbol">Market Pair:</label>
                    <input id="symbol" type="text" bind:value={draftSymbol} placeholder="e.g. ETH" />
                </div>

                <div class="input-row">
                    <label for="tf">Timeframe:</label>
                    <div class="tf-split-group">
                        <input id="tf" type="number" bind:value={tfValue} min="1" class="tf-number-input" />
                        <select bind:value={tfUnit} class="tf-unit-select">
                            <option value="seconds">Seconds</option>
                            <option value="minutes">Minutes</option>
                            <option value="hours">Hours</option>
                        </select>
                    </div>
                </div>

                <hr class="section-divider" />

                <div class="input-row">
                    <label for="emafast">EMA Fast:</label>
                    <input id="emafast" type="number" bind:value={draftEmaFast} min="1" />
                </div>
                <div class="input-row">
                    <label for="emamed">EMA Medium:</label>
                    <input id="emamed" type="number" bind:value={draftEmaMedium} min="1" />
                </div>
                <div class="input-row">
                    <label for="emaslow">EMA Slow:</label>
                    <input id="emaslow" type="number" bind:value={draftEmaSlow} min="1" />
                </div>
                <div class="input-row">
                    <label for="emalong">EMA Long:</label>
                    <input id="emalong" type="number" bind:value={draftEmaLong} min="1" />
                </div>
                <div class="input-row">
                    <label for="rsilength">RSI Period:</label>
                    <input id="rsilength" type="number" bind:value={draftRsiPeriod} min="1" />
                </div>
                <div class="input-row">
                    <label for="macdfast">MACD Fast:</label>
                    <input id="macdfast" type="number" bind:value={draftMacdFast} min="1" />
                </div>
                <div class="input-row">
                    <label for="macdslow">MACD Slow:</label>
                    <input id="macdslow" type="number" bind:value={draftMacdSlow} min="1" />
                </div>
                <div class="input-row">
                    <label for="macdsig">MACD Signal:</label>
                    <input id="macdsig" type="number" bind:value={draftMacdSignal} min="1" />
                </div>
                <div class="input-row">
                    <label for="adxlen">ADX Period:</label>
                    <input id="adxlen" type="number" bind:value={draftAdxPeriod} min="1" />
                </div>
                <div class="input-row">
                    <label for="atrlen">ATR Window:</label>
                    <input id="atrlen" type="number" bind:value={draftAtrPeriod} min="1" />
                </div>
                <div class="input-row">
                    <label for="sqzlen">Squeeze Wave:</label>
                    <input id="sqzlen" type="number" bind:value={draftSqueezePeriod} min="1" />
                </div>
            </div>
        </div>

        <div class="panel-section">
            <h3 class="panel-section-title">API Key Configuration</h3>
            <div class="setting-group-box">
                <span class="selectors-label">DeepSeek API Key</span>
                <div class="key-input-row">
                    <input
                        type="password"
                        class="key-field"
                        placeholder="sk-..."
                        bind:value={draftApiKey}
                    />
                    <button
                        class="key-save-btn"
                        disabled={apiKeyStatus === 'saving'}
                        onclick={saveApiKey}
                    >
                        {apiKeyStatus === 'saving' ? '...' : 'Save Key'}
                    </button>
                </div>
                {#if apiKeyStatus === 'success'}
                    <div class="status-msg success-msg">Key validated and saved.</div>
                {/if}
                {#if apiKeyStatus === 'error' && apiKeyError}
                    <div class="status-msg error-msg">{apiKeyError}</div>
                {/if}
            </div>
        </div>

        <div class="panel-section">
            <h3 class="panel-section-title">
                Edit Technical Rules
                <button class="rules-toggle-btn" onclick={() => {
                    showRulesEditor = !showRulesEditor;
                    if (showRulesEditor && !draftRules) fetchRules();
                }}>
                    {showRulesEditor ? '▲ Hide' : '▼ Show'}
                </button>
            </h3>
            {#if showRulesEditor}
                <div class="setting-group-box">
                    <textarea
                        class="rules-editor"
                        rows="12"
                        bind:value={draftRules}
                    ></textarea>
                    <div class="rules-actions">
                        <button
                            class="key-save-btn"
                            disabled={rulesStatus === 'saving'}
                            onclick={saveRules}
                        >
                            {rulesStatus === 'saving' ? 'Saving...' : 'Save Rules'}
                        </button>
                        {#if rulesStatus === 'success'}
                            <span class="status-msg success-msg">Rules updated successfully.</span>
                        {/if}
                        {#if rulesStatus === 'error'}
                            <span class="status-msg error-msg">Failed to save rules.</span>
                        {/if}
                    </div>
                </div>
            {/if}
        </div>
    </div>

    {#if validationError}
        <div class="validation-error-box">
            {validationError}
        </div>
    {/if}

    <div class="panel-footer">
        <button class="action-btn cancel-btn" onclick={closePanel}>Cancel</button>
        <button 
            class="action-btn apply-btn" 
            class:btn-active={isChanged} 
            class:btn-inactive={!isChanged} 
            disabled={!isChanged}
            onclick={applyChanges}
        >
            Apply Settings
        </button>
    </div>
</div>

<style>
    .panel-overlay {
        position: fixed;
        inset: 0;
        z-index: 40;
        background-color: rgba(0, 0, 0, 0.5);
        backdrop-filter: blur(2px);
    }
    .panel-drawer {
        position: fixed;
        left: 0;
        top: 0;
        bottom: 0;
        width: 360px;
        background-color: #131722;
        border-right: 1px solid #2a2e39;
        z-index: 45;
        display: flex;
        flex-direction: column;
        transform: translateX(-100%);
        transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
        box-shadow: 10px 0 30px rgba(0,0,0,0.5);
    }
    .panel-open {
        transform: translateX(0);
    }
    .panel-header {
        padding: 16px 20px;
        border-bottom: 1px solid #1e293b;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .panel-title {
        font-size: 12px;
        font-weight: 800;
        letter-spacing: 0.1em;
        color: #cbd5e1;
        margin: 0;
        text-transform: uppercase;
    }
    .panel-close-btn {
        background: none;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 10px;
        font-weight: 700;
        text-transform: uppercase;
        cursor: pointer;
        padding: 4px 8px;
        border-radius: 4px;
        transition: all 0.2s;
    }
    .panel-close-btn:hover {
        color: #ef5350;
        border-color: rgba(239, 83, 80, 0.4);
        background-color: rgba(239, 83, 80, 0.05);
    }
    .panel-body {
        flex: 1;
        overflow-y: auto;
        padding: 20px;
        display: flex;
        flex-direction: column;
        gap: 20px;
    }
    .panel-section {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }
    .panel-section-title {
        font-size: 10px;
        font-weight: 800;
        letter-spacing: 0.08em;
        color: #3b82f6;
        text-transform: uppercase;
        margin: 0;
        border-left: 2px solid #3b82f6;
        padding-left: 8px;
    }
    .setting-group-box {
        background-color: #0e111a;
        border: 1px solid #1e293b;
        border-radius: 6px;
        padding: 10px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .selectors-label {
        font-size: 9px;
        font-weight: 700;
        text-transform: uppercase;
        color: #64748b;
    }
    .toggle-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 6px;
    }
    .selector-btn {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 9px;
        font-weight: 800;
        padding: 6px 0;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s ease;
        text-transform: uppercase;
        text-align: center;
    }
    .selector-btn:hover {
        border-color: #4c526e;
        color: #cbd5e1;
    }
    .selector-btn.active {
        background-color: rgba(59, 130, 246, 0.12);
        border-color: #3b82f6;
        color: #3b82f6;
        box-shadow: 0 0 8px rgba(59, 130, 246, 0.15);
    }
    .parameter-inputs-scroll {
        background-color: #0e111a;
        border: 1px solid #1e293b;
        border-radius: 6px;
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        max-height: 280px;
        overflow-y: auto;
    }
    .input-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
    }
    .input-row label {
        font-size: 10px;
        color: #94a3b8;
        font-weight: 600;
    }
    .input-row input {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #cbd5e1;
        font-family: ui-monospace, monospace;
        font-size: 11px;
        padding: 4px 8px;
        border-radius: 4px;
        width: 80px;
        text-align: right;
        outline: none;
        transition: border-color 0.2s;
    }
    .input-row input:focus {
        border-color: #3b82f6;
    }
    .section-divider {
        border: 0;
        border-top: 1px solid #1e293b;
        margin: 6px 0;
    }
    .tf-split-group {
        display: flex;
        gap: 4px;
        width: 140px;
    }
    .tf-number-input {
        width: 50px !important;
    }
    .tf-unit-select {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #cbd5e1;
        font-size: 10px;
        font-weight: bold;
        padding: 2px 4px;
        border-radius: 4px;
        flex: 1;
        outline: none;
        cursor: pointer;
    }
    .tf-unit-select:focus {
        border-color: #3b82f6;
    }
    .validation-error-box {
        background-color: rgba(239, 83, 80, 0.1);
        border: 1px solid #ef5350;
        border-radius: 6px;
        padding: 10px;
        margin: 0 20px 10px 20px;
        font-size: 10px;
        color: #ef5350;
        font-weight: 600;
        line-height: 1.4;
    }
    .panel-footer {
        padding: 16px 20px;
        border-top: 1px solid #1e293b;
        display: flex;
        gap: 10px;
    }
    .action-btn {
        flex: 1;
        padding: 10px 0;
        border-radius: 6px;
        font-size: 10px;
        font-weight: 800;
        text-transform: uppercase;
        cursor: pointer;
        transition: all 0.25s ease-in-out;
        text-align: center;
    }
    .cancel-btn {
        background-color: transparent;
        border: 1px solid #2a2e39;
        color: #8f929d;
    }
    .cancel-btn:hover {
        border-color: #ef5350;
        color: #ef5350;
    }
    .btn-inactive {
        background: #2a2e39 !important;
        border: 1px solid #1e293b !important;
        color: #4c525e !important;
        cursor: not-allowed !important;
    }
    .btn-active {
        background: linear-gradient(135deg, #1e40af, #3b82f6) !important;
        border: 1px solid #3b82f6 !important;
        color: #f1f5f9 !important;
        cursor: pointer !important;
        box-shadow: 0 0 10px rgba(59, 130, 246, 0.25);
    }
    .btn-active:hover {
        background: linear-gradient(135deg, #1e3a8a, #2563eb) !important;
    }
    .key-input-row {
        display: flex;
        gap: 6px;
    }
    .key-field {
        flex: 1;
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #f1f5f9;
        font-family: 'Courier New', monospace;
        font-size: 11px;
        padding: 6px 8px;
        border-radius: 4px;
        outline: none;
    }
    .key-field:focus {
        border-color: #3b82f6;
    }
    .key-save-btn {
        background-color: #1e40af;
        border: 1px solid #3b82f6;
        color: #f1f5f9;
        font-size: 10px;
        font-weight: 700;
        padding: 6px 12px;
        border-radius: 4px;
        cursor: pointer;
        text-transform: uppercase;
        white-space: nowrap;
        transition: all 0.15s;
    }
    .key-save-btn:hover {
        background-color: #1e3a8a;
    }
    .key-save-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .status-msg {
        font-size: 10px;
        padding: 4px 0;
    }
    .success-msg {
        color: #10b981;
    }
    .error-msg {
        color: #ef4444;
    }
    .rules-toggle-btn {
        background: none;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 9px;
        font-weight: 600;
        cursor: pointer;
        border-radius: 3px;
        padding: 1px 6px;
        margin-left: 8px;
    }
    .rules-toggle-btn:hover {
        color: #cbd5e1;
        border-color: #3b82f6;
    }
    .rules-editor {
        width: 100%;
        background-color: #0a0d14;
        border: 1px solid #2a2e39;
        color: #cbd5e1;
        font-family: 'Courier New', monospace;
        font-size: 10px;
        line-height: 1.5;
        padding: 8px;
        border-radius: 4px;
        resize: vertical;
        outline: none;
    }
    .rules-editor:focus {
        border-color: #3b82f6;
    }
    .rules-actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }
</style>
