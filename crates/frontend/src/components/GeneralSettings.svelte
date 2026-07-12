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

    $effect(() => { loadSettings(); });
</script>

<div class={styles.settingsView}>
    <h2>Profile Settings</h2>

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
                    <span class={styles.hint}>halt workspace after this many</span>
                </div>
                <button class={styles.saveBtn} onclick={saveFailover} disabled={saveStatus === 'saving'}>
                    {saveStatus === 'saving' ? 'Saving...' : saveStatus === 'success' ? 'Saved' : 'Save API Failover'}
                </button>
            </div>
        </div>
    {/if}
</div>
