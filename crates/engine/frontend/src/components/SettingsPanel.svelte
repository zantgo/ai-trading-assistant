<script lang="ts">
    import { getState } from '../state.svelte';
    const app = getState();

    let draftSymbol = $state('');
    let draftDuration = $state(5);
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

    $effect(() => {
        if (app.showSettingsPanel) {
            draftSymbol = app.activeSymbol;
            draftDuration = app.barDurationSec;
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
        }
    });

    const closePanel = () => {
        app.showSettingsPanel = false;
    };

    const applyTechnicalChanges = async () => {
        app.activeSymbol = draftSymbol;
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

        try {
            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body)
            });
            if (!res.ok) throw new Error('API server rejected config save');
            console.log("✏️ Config successfully persisted on backend.");
        } catch (err) {
            console.error("❌ Failed to push config to backend server:", err);
        }

        closePanel();
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
                    <button class="selector-btn" class:active={app.showEmas} onclick={() => app.showEmas = !app.showEmas}>
                        EMAs
                    </button>
                    <button class="selector-btn" class:active={app.showBb} onclick={() => app.showBb = !app.showBb}>
                        BB
                    </button>
                    <button class="selector-btn" class:active={app.showVwap} onclick={() => app.showVwap = !app.showVwap}>
                        VWAP
                    </button>
                </div>
            </div>

            <div class="setting-group-box">
                <span class="selectors-label">Indicators Panels</span>
                <div class="toggle-grid">
                    <button class="selector-btn" class:active={app.showVolume} onclick={() => app.showVolume = !app.showVolume}>
                        Volume
                    </button>
                    <button class="selector-btn" class:active={app.showAdx} onclick={() => app.showAdx = !app.showAdx}>
                        ADX
                    </button>
                    <button class="selector-btn" class:active={app.showAtr} onclick={() => app.showAtr = !app.showAtr}>
                        ATR
                    </button>
                    <button class="selector-btn" class:active={app.showRsi} onclick={() => app.showRsi = !app.showRsi}>
                        RSI
                    </button>
                    <button class="selector-btn" class:active={app.showMacd} onclick={() => app.showMacd = !app.showMacd}>
                        MACD
                    </button>
                    <button class="selector-btn" class:active={app.showSqueeze} onclick={() => app.showSqueeze = !app.showSqueeze}>
                        Squeeze
                    </button>
                </div>
            </div>
        </div>

        <div class="panel-section">
            <h3 class="panel-section-title">Technical Parameters</h3>

            <div class="parameter-inputs-scroll">
                <div class="input-row">
                    <label for="symbol">Market Pair:</label>
                    <input id="symbol" type="text" bind:value={draftSymbol} placeholder="e.g. ETH" />
                </div>

                <div class="input-row">
                    <label for="tf">Timeframe (sec):</label>
                    <input id="tf" type="number" bind:value={draftDuration} min="1" />
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
    </div>

    <div class="panel-footer">
        <button class="action-btn cancel-btn" onclick={closePanel}>Cancel</button>
        <button class="action-btn apply-btn" onclick={applyTechnicalChanges}>Apply Settings</button>
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
        transition: all 0.2s;
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
    .apply-btn {
        background: linear-gradient(135deg, #1e40af, #3b82f6);
        border: 1px solid #3b82f6;
        color: #f1f5f9;
    }
    .apply-btn:hover {
        background: linear-gradient(135deg, #1e3a8a, #2563eb);
    }
</style>
