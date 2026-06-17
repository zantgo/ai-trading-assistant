<script lang="ts">
    import { getState } from './state.svelte';

    const app = getState();
    let mode = $state('paper');
    let currency = $state('USDT');
    let exchange = $state('Hyperliquid');
    let capital = $state('');
    let error = $state<string | null>(null);
    let loading = $state(false);

    async function handleEnter() {
        error = null;
        const capitalNum = parseFloat(capital);
        if (!capitalNum || capitalNum <= 0) {
            error = 'Please enter a valid initial capital amount greater than 0.';
            return;
        }
        loading = true;
        const result = await app.initSession(mode, currency, exchange, capitalNum);
        if (!result.success) {
            error = result.error || 'Failed to initialize session.';
        }
        loading = false;
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter') {
            handleEnter();
        }
    }
</script>

<div class="welcome-gate">
    <div class="welcome-card">
        <div class="welcome-header">
            <h1 class="welcome-title">AI Trading Assistant</h1>
            <p class="welcome-subtitle">Configure your session to begin</p>
        </div>

        <div class="welcome-form">
            <!-- Trading Mode -->
            <div class="form-group">
                <label class="form-label">Trading Mode</label>
                <div class="radio-group">
                    <label class="radio-option" class:active={mode === 'paper'}>
                        <input type="radio" name="mode" value="paper" bind:group={mode} />
                        <span class="radio-label">Paper Trading</span>
                        <span class="radio-badge enabled">Available</span>
                    </label>
                    <label class="radio-option disabled" class:active={mode === 'live'}>
                        <input type="radio" name="mode" value="live" bind:group={mode} disabled />
                        <span class="radio-label">Live Trading</span>
                        <span class="radio-badge disabled">Coming Soon</span>
                    </label>
                </div>
            </div>

            <!-- Base Currency -->
            <div class="form-group">
                <label class="form-label">Base Currency</label>
                <div class="radio-group">
                    <label class="radio-option" class:active={currency === 'USDT'}>
                        <input type="radio" name="currency" value="USDT" bind:group={currency} />
                        <span class="radio-label">USDT</span>
                        <span class="radio-badge enabled">Available</span>
                    </label>
                    <label class="radio-option disabled" class:active={currency === 'USDC'}>
                        <input type="radio" name="currency" value="USDC" bind:group={currency} disabled />
                        <span class="radio-label">USDC</span>
                        <span class="radio-badge disabled">Coming Soon</span>
                    </label>
                </div>
            </div>

            <!-- Exchange -->
            <div class="form-group">
                <label class="form-label">Exchange</label>
                <select class="form-select" bind:value={exchange} disabled>
                    <option value="Hyperliquid">Hyperliquid</option>
                </select>
                <span class="form-hint">More exchanges coming soon</span>
            </div>

            <!-- Paper Trading Capital -->
            {#if mode === 'paper'}
                <div class="form-group">
                    <label class="form-label" for="capital-input">
                        Initial Portfolio Capital ({currency})
                    </label>
                    <input
                        id="capital-input"
                        type="number"
                        class="form-input"
                        placeholder="e.g. 10000"
                        bind:value={capital}
                        onkeydown={handleKeydown}
                        min="1"
                        step="any"
                    />
                    <span class="form-hint">Enter your starting paper trading balance</span>
                </div>
            {/if}

            {#if error}
                <div class="form-error">{error}</div>
            {/if}

            <button
                class="enter-button"
                onclick={handleEnter}
                disabled={loading}
            >
                {#if loading}
                    <span class="spinner"></span>
                    Initializing...
                {:else}
                    Enter System
                {/if}
            </button>
        </div>
    </div>
</div>

<style>
    .welcome-gate {
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
        background: linear-gradient(135deg, #0f0f1a 0%, #1a1a2e 50%, #16213e 100%);
        padding: 1rem;
    }
    .welcome-card {
        background: #1a1a2e;
        border: 1px solid #2a2a4a;
        border-radius: 16px;
        padding: 2.5rem;
        width: 100%;
        max-width: 480px;
        box-shadow: 0 8px 40px rgba(0, 0, 0, 0.4);
    }
    .welcome-header {
        text-align: center;
        margin-bottom: 2rem;
    }
    .welcome-title {
        font-size: 1.6rem;
        font-weight: 700;
        color: #e0e0ff;
        margin: 0 0 0.5rem 0;
    }
    .welcome-subtitle {
        color: #8888aa;
        font-size: 0.9rem;
        margin: 0;
    }
    .welcome-form {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }
    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }
    .form-label {
        color: #aaaacc;
        font-size: 0.8rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    .radio-group {
        display: flex;
        gap: 0.5rem;
    }
    .radio-option {
        flex: 1;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.65rem 0.8rem;
        background: #252540;
        border: 1px solid #333355;
        border-radius: 8px;
        cursor: pointer;
        transition: all 0.2s;
    }
    .radio-option.active {
        border-color: #5b7fff;
        background: #2a2a50;
    }
    .radio-option.disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .radio-option input[type="radio"] {
        accent-color: #5b7fff;
        margin: 0;
    }
    .radio-label {
        color: #d0d0ee;
        font-size: 0.85rem;
        font-weight: 500;
    }
    .radio-badge {
        margin-left: auto;
        font-size: 0.65rem;
        padding: 0.15rem 0.45rem;
        border-radius: 4px;
        font-weight: 600;
    }
    .radio-badge.enabled {
        background: #1a3a1a;
        color: #4caf50;
    }
    .radio-badge.disabled {
        background: #3a2a1a;
        color: #ff9800;
    }
    .form-select {
        padding: 0.65rem 0.8rem;
        background: #252540;
        border: 1px solid #333355;
        border-radius: 8px;
        color: #d0d0ee;
        font-size: 0.9rem;
        cursor: pointer;
    }
    .form-select:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    .form-input {
        padding: 0.65rem 0.8rem;
        background: #252540;
        border: 1px solid #333355;
        border-radius: 8px;
        color: #e0e0ff;
        font-size: 1rem;
        outline: none;
        transition: border-color 0.2s;
    }
    .form-input:focus {
        border-color: #5b7fff;
    }
    .form-hint {
        color: #666688;
        font-size: 0.7rem;
    }
    .form-error {
        background: #3a1a1a;
        border: 1px solid #663333;
        color: #ff6666;
        padding: 0.6rem 0.8rem;
        border-radius: 8px;
        font-size: 0.8rem;
    }
    .enter-button {
        padding: 0.8rem;
        background: linear-gradient(135deg, #5b7fff, #7b5fff);
        border: none;
        border-radius: 8px;
        color: white;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        transition: opacity 0.2s, transform 0.1s;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
    }
    .enter-button:hover:not(:disabled) {
        opacity: 0.9;
        transform: translateY(-1px);
    }
    .enter-button:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    .spinner {
        width: 16px;
        height: 16px;
        border: 2px solid rgba(255,255,255,0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.6s linear infinite;
    }
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
</style>
