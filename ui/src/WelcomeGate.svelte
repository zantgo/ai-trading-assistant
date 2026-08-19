<script lang="ts">
    import { useAppStore } from './state.svelte';
    import styles from './WelcomeGate.module.css';

    const app = useAppStore();
    let exchange = $state('Hyperliquid');
    let currency = $state('USDC');
    let mode = $state<'paper' | 'live'>('paper');
    let capital = $state(1000);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Prefill the paper capital + mode from an existing session.
    $effect(() => {
        if (app.sessionChecked && app.sessionActive) {
            mode = (app.sessionMode as 'paper' | 'live') ?? 'paper';
            if (app.sessionCapital > 0) capital = app.sessionCapital;
        }
    });

// Perpetual-futures settlement rules per exchange:
//  - Hyperliquid settles exclusively in USDC.
//  - Bitget's dashboard exposes only USDT-M futures. (The backend's
//    `ExchangeChoice::supports_currency` still returns true for USDC,
//    but we don't surface it in the welcome modal — keeping the
//    selector aligned with the operator's preferred Bitget product.)
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
        loading = true;
        const result = await app.initSession(
            currency,
            exchange,
            mode,
            mode === 'paper' ? Number(capital) : undefined,
        );
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

<div class={styles.welcomeGate}>
    <div class={styles.welcomeCard}>
        <div class={styles.welcomeHeader}>
            <h1 class={styles.welcomeTitle}>Trading Platform</h1>
            <p class={styles.welcomeSubtitle}>Configure your session to begin</p>
        </div>

        <div class={styles.welcomeForm}>
            <!-- Exchange -->
            <div class={styles.formGroup}>
                <label class={styles.formLabel} for="exchange-select">Exchange</label>
                <select id="exchange-select" class={styles.formSelect} bind:value={exchange}>
                    <option value="Hyperliquid">Hyperliquid</option>
                    <option value="Bitget">Bitget</option>
                </select>
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

            <!-- Execution Mode -->
            <div class={styles.formGroup}>
                <span class={styles.formLabel}>Execution Mode</span>
                <div class={styles.radioGroup}>
                    <label class="{styles.radioOption} {mode === 'paper' ? styles.active : ''}">
                        <input type="radio" name="mode" value="paper" bind:group={mode} />
                        <span class={styles.radioLabel}>Paper Trading</span>
                        <span class="{styles.radioBadge} {styles.enabled}">Simulated</span>
                    </label>
                    <label class="{styles.radioOption} {mode === 'live' ? styles.active : ''}">
                        <input type="radio" name="mode" value="live" bind:group={mode} />
                        <span class={styles.radioLabel}>Live Trading</span>
                        <span class="{styles.radioBadge} {styles.enabled}">Real orders</span>
                    </label>
                </div>
            </div>

            {#if mode === 'paper'}
                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="paper-capital">Paper Session Capital (USD)</label>
                    <input id="paper-capital" type="number" min="100" step="100"
                        class={styles.formInput} bind:value={capital} />
                    <p class={styles.formHint}>The paper balance for instances created in this session.</p>
                </div>
            {:else}
                <div class={styles.formGroup}>
                    <p class={styles.formHint}>
                        Add your exchange API key in <strong>Settings → Exchange API Keys</strong> first
                        (with EXCHANGE_SECRET_KEY set on the server). Live trading uses the balance of your
                        exchange account — there is no paper balance.
                    </p>
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
