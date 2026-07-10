<script lang="ts">
    import { useAppStore } from './state.svelte';
    import styles from './WelcomeGate.module.css';

    const app = useAppStore();
    let mode = $state('paper');
    let exchange = $state('Hyperliquid');
    let currency = $state('USDC');
    let capital = $state('');
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Perpetual-futures settlement rules per exchange:
    //  - Hyperliquid settles exclusively in USDC.
    //  - Bitget supports USDT-M futures only.
    const supportedCurrencies = $derived(
        exchange === 'Hyperliquid' ? ['USDC'] : ['USDT']
    );

    // Keep the selected currency valid whenever the exchange changes.
    $effect(() => {
        if (!supportedCurrencies.includes(currency)) {
            currency = supportedCurrencies[0];
        }
    });

    function currencyAvailable(c: string): boolean {
        return supportedCurrencies.includes(c);
    }

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

    function sanitizeCapital(e: Event) {
        const input = e.target as HTMLInputElement;
        const val = input.value;
        if (val.startsWith('-') || parseFloat(val) < 0) {
            input.value = val.replace(/[^0-9.]/g, '');
        }
        capital = input.value;
    }
</script>

<div class={styles.welcomeGate}>
    <div class={styles.welcomeCard}>
        <div class={styles.welcomeHeader}>
            <h1 class={styles.welcomeTitle}>AI Trading Assistant</h1>
            <p class={styles.welcomeSubtitle}>Configure your session to begin</p>
        </div>

        <div class={styles.welcomeForm}>
            <!-- Trading Mode -->
            <div class={styles.formGroup}>
                <span class={styles.formLabel}>Trading Mode</span>
                <div class={styles.radioGroup}>
                    <label class="{styles.radioOption} {mode === 'paper' ? styles.active : ''}">
                        <input type="radio" name="mode" value="paper" bind:group={mode} />
                        <span class={styles.radioLabel}>Paper Trading</span>
                        <span class="{styles.radioBadge} {styles.enabled}">Available</span>
                    </label>
                    <label class="{styles.radioOption} {styles.disabled} {mode === 'live' ? styles.active : ''}">
                        <input type="radio" name="mode" value="live" bind:group={mode} disabled />
                        <span class={styles.radioLabel}>Live Trading</span>
                        <span class="{styles.radioBadge} {styles.disabled}">Not available</span>
                    </label>
                </div>
            </div>

            <!-- Exchange -->
            <div class={styles.formGroup}>
                <label class={styles.formLabel} for="exchange-select">Exchange</label>
                <select id="exchange-select" class={styles.formSelect} bind:value={exchange}>
                    <option value="Hyperliquid">Hyperliquid</option>
                    <option value="Bitget">Bitget</option>
                </select>
                <span class={styles.formHint}>Perpetual futures. Hyperliquid settles in USDC; Bitget supports USDT and USDC.</span>
            </div>

            <!-- Settlement Currency -->
            <div class={styles.formGroup}>
                <span class={styles.formLabel}>Settlement Currency</span>
                <div class={styles.radioGroup}>
                    <label class="{styles.radioOption} {!currencyAvailable('USDT') ? styles.disabled : ''} {currency === 'USDT' ? styles.active : ''}">
                        <input type="radio" name="currency" value="USDT" bind:group={currency} disabled={!currencyAvailable('USDT')} />
                        <span class={styles.radioLabel}>USDT</span>
                        <span class="{styles.radioBadge} {currencyAvailable('USDT') ? styles.enabled : styles.disabled}">
                            {currencyAvailable('USDT') ? 'Available' : 'Not available'}
                        </span>
                    </label>
                    <label class="{styles.radioOption} {!currencyAvailable('USDC') ? styles.disabled : ''} {currency === 'USDC' ? styles.active : ''}">
                        <input type="radio" name="currency" value="USDC" bind:group={currency} disabled={!currencyAvailable('USDC')} />
                        <span class={styles.radioLabel}>USDC</span>
                        <span class="{styles.radioBadge} {currencyAvailable('USDC') ? styles.enabled : styles.disabled}">
                            {currencyAvailable('USDC') ? 'Available' : 'Not available'}
                        </span>
                    </label>
                </div>
            </div>

            <!-- Paper Trading Capital -->
            {#if mode === 'paper'}
                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="capital-input">
                        Initial Portfolio Capital ({currency})
                    </label>
                    <input
                        id="capital-input"
                        type="number"
                        class={styles.formInput}
                        placeholder="e.g. 10000"
                        bind:value={capital}
                        onkeydown={handleKeydown}
                        oninput={sanitizeCapital}
                        min="0"
                        step="any"
                    />
                    <span class={styles.formHint}>Enter your starting paper trading balance</span>
                </div>
            {/if}

            {#if error}
                <div class={styles.formError}>{error}</div>
            {/if}

            <button
                class={styles.enterButton}
                onclick={handleEnter}
                disabled={loading}
            >
                {#if loading}
                    <span class={styles.spinner}></span>
                    Initializing...
                {:else}
                    Enter System
                {/if}
            </button>
        </div>
    </div>
</div>
