<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './GeneralSettings.module.css';

    const app = useAppStore();

    let draftFailoverRetries = $state(5);
    let draftFailoverDelay = $state(30);
    let draftFailoverMax = $state(10);
    let loaded = $state(false);
    let saveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    async function loadSettings() {
        try {
            const res = await fetch('/api/config');
            const config = await res.json();
            if (config.api_failover) {
                draftFailoverRetries = config.api_failover.max_retries_per_call ?? 5;
                draftFailoverDelay = config.api_failover.retry_delay_seconds ?? 30;
                draftFailoverMax = config.api_failover.max_consecutive_failures ?? 10;
            }
            loaded = true;
        } catch (e) {
            console.error('Failed to load settings:', e);
        }
    }

    async function saveFailover() {
        saveStatus = 'saving';
        try {
            const res = await fetch('/api/config');
            const config = await res.json();
            config.api_failover = {
                max_retries_per_call: Number(draftFailoverRetries),
                retry_delay_seconds: Number(draftFailoverDelay),
                max_consecutive_failures: Number(draftFailoverMax),
            };
            const saveRes = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(config),
            });
            saveStatus = saveRes.ok ? 'success' : 'error';
            if (saveRes.ok) setTimeout(() => { saveStatus = 'idle'; }, 2000);
        } catch (_) {
            saveStatus = 'error';
        }
    }

    // ─── Session Reconfiguration ─────────────────────────────────────────

    let reiExchange = $state(app.sessionExchange);
    let reiCurrency = $state(app.sessionCurrency);
    let reiLoading = $state(false);
    let reiError = $state<string | null>(null);
    let reiSuccess = $state(false);

    const reiSupported = $derived(
        reiExchange === 'Hyperliquid' ? ['USDC'] : ['USDT']
    );

    function reiCurrencyAvailable(c: string): boolean {
        return reiSupported.includes(c);
    }

    // Keep currency valid when exchange changes
    $effect(() => {
        if (!reiSupported.includes(reiCurrency)) {
            reiCurrency = reiSupported[0];
        }
    });

    async function handleReinitialize() {
        reiError = null;
        reiSuccess = false;
        reiLoading = true;
        const result = await app.initSession(reiCurrency, reiExchange);
        if (!result.success) {
            reiError = result.error || 'Failed to reinitialize session.';
        } else {
            reiSuccess = true;
            setTimeout(() => { reiSuccess = false; }, 3000);
        }
        reiLoading = false;
    }

    $effect(() => { loadSettings(); });
</script>

<div class={styles.settingsView}>
    <h2>Settings</h2>

    {#if !loaded}
        <div class={styles.loadingRow}>Loading settings...</div>
    {:else}
        <div class={styles.settingsGrid}>
            <div class={styles.settingsCard}>
                <h3>API Failover</h3>
                <div class={styles.inputRow}>
                    <label for="failover-retries">Max Retries Per Call:</label>
                    <input id="failover-retries" type="number" bind:value={draftFailoverRetries} min="1" max="20" />
                </div>
                <div class={styles.inputRow}>
                    <label for="failover-delay">Retry Delay (seconds):</label>
                    <input id="failover-delay" type="number" bind:value={draftFailoverDelay} min="1" max="300" />
                </div>
                <div class={styles.inputRow}>
                    <label for="failover-max">Max Consecutive Failures:</label>
                    <input id="failover-max" type="number" bind:value={draftFailoverMax} min="1" max="50" />
                    <span class={styles.hint}>halt instance after this many</span>
                </div>
                <button class={styles.saveBtn} onclick={saveFailover} disabled={saveStatus === 'saving'}>
                    {saveStatus === 'saving' ? 'Saving...' : saveStatus === 'success' ? 'Saved' : 'Save API Failover'}
                </button>
            </div>

            <!-- Session Reconfiguration -->
            <div class={styles.settingsCard}>
                <h3>Session Configuration</h3>

                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="rei-exchange">Exchange</label>
                    <select id="rei-exchange" class={styles.formSelect} bind:value={reiExchange}>
                        <option value="Hyperliquid">Hyperliquid</option>
                        <option value="Bitget">Bitget</option>
                    </select>
                </div>

                <div class={styles.formGroup}>
                    <span class={styles.formLabel}>Settlement Currency</span>
                    <div class={styles.radioGroup}>
                        <label class="{styles.radioOption} {!reiCurrencyAvailable('USDT') ? styles.disabled : ''} {reiCurrency === 'USDT' ? styles.active : ''}">
                            <input type="radio" name="rei-currency" value="USDT" bind:group={reiCurrency} disabled={!reiCurrencyAvailable('USDT')} />
                            <span class={styles.radioLabel}>USDT</span>
                            <span class="{styles.radioBadge} {reiCurrencyAvailable('USDT') ? styles.enabled : styles.disabled}">
                                {reiCurrencyAvailable('USDT') ? 'Available' : 'N/A'}
                            </span>
                        </label>
                        <label class="{styles.radioOption} {!reiCurrencyAvailable('USDC') ? styles.disabled : ''} {reiCurrency === 'USDC' ? styles.active : ''}">
                            <input type="radio" name="rei-currency" value="USDC" bind:group={reiCurrency} disabled={!reiCurrencyAvailable('USDC')} />
                            <span class={styles.radioLabel}>USDC</span>
                            <span class="{styles.radioBadge} {reiCurrencyAvailable('USDC') ? styles.enabled : styles.disabled}">
                                {reiCurrencyAvailable('USDC') ? 'Available' : 'N/A'}
                            </span>
                        </label>
                    </div>
                </div>

                <div class={styles.warningText}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
                        <line x1="12" y1="9" x2="12" y2="13"></line>
                        <line x1="12" y1="17" x2="12.01" y2="17"></line>
                    </svg>
                    Reinitializing will close all active instances and restart with the new exchange and currency.
                </div>

                {#if reiError}
                    <div class={styles.formError}>{reiError}</div>
                {/if}
                {#if reiSuccess}
                    <div class={styles.formSuccess}>Session reinitialized successfully.</div>
                {/if}

                <button class={styles.reinitBtn} onclick={handleReinitialize} disabled={reiLoading}>
                    {#if reiLoading}
                        <span class={styles.spinner}></span>
                        Reinitializing...
                    {:else}
                        Reinitialize Session
                    {/if}
                </button>
            </div>
        </div>
    {/if}
</div>
