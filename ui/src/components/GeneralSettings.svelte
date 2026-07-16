<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { fmtPrice } from '../lib/telemetry';
    import styles from './GeneralSettings.module.css';

    const app = useAppStore();

    let activeSection = $state<'fee' | 'settings' | 'share'>('settings');

    // ─── Config sharing ───────────────────────────────────────────────
    let importStatus = $state<'idle' | 'importing' | 'success' | 'error'>('idle');
    let importMessage = $state('');

    function handleFilePicked(e: Event) {
        const input = e.target as HTMLInputElement;
        const file = input.files?.[0];
        if (!file) return;
        importStatus = 'importing';
        importMessage = '';
        const reader = new FileReader();
        reader.onload = async (ev) => {
            const toml = ev.target?.result as string;
            try {
                const res = await fetch('/api/workspace/toml', {
                    method: 'POST',
                    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
                    body: toml,
                });
                const msg = await res.text();
                importStatus = res.ok ? 'success' : 'error';
                importMessage = msg;
                if (res.ok) {
                    setTimeout(() => { importStatus = 'idle'; importMessage = ''; }, 5000);
                }
            } catch (e: any) {
                importStatus = 'error';
                importMessage = e?.message || 'Import failed';
            }
        };
        reader.readAsText(file);
    }

    // ─── API Failover settings ───────────────────────────────────────────
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

    // ─── Fee Reference Calculator ───────────────────────────────────────
    let calcLeverage = $state(10);
    let calcCapital = $state(1000);
    let calcFeePct = $state(0.06);

    const calcNotional = $derived(calcCapital * calcLeverage);
    const calcFees = $derived((calcFeePct / 100) * calcNotional * 2);
    const calcMinProfitPct = $derived(calcCapital > 0 ? (calcFees / calcCapital) * 100 : 0);

    function formatUsd(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }

    function formatPct(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '0.00%';
        return v.toFixed(2) + '%';
    }

    $effect(() => { loadSettings(); });
</script>

<div class={styles.profileLayout}>
    <div class={styles.profileSidebar}>
        <h2 class={styles.profileTitle}>HOME</h2>
        <button
            class="{styles.sidebarItem} {activeSection === 'fee' ? styles.sidebarActive : ''}"
            onclick={() => activeSection = 'fee'}
        >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
            </svg>
            Fee Projection
        </button>
        <button
            class="{styles.sidebarItem} {activeSection === 'settings' ? styles.sidebarActive : ''}"
            onclick={() => activeSection = 'settings'}
        >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            </svg>
            Settings
        </button>
        <button
            class="{styles.sidebarItem} {activeSection === 'share' ? styles.sidebarActive : ''}"
            onclick={() => activeSection = 'share'}
        >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/>
            </svg>
            Share Config
        </button>
    </div>

    <div class={styles.profileContent}>
        {#if activeSection === 'share'}
            <div class={styles.profileCard}>
                <h3>Share Configuration</h3>
                <p class={styles.cardSub}>
                    Download your workspace (instances, timeframes, indicators, fees, safety rules) as a single <code>config.toml</code> file.
                    Copy it to another machine and start with <code>--mode headless</code> to run the same setup there.
                    Platform-level fields (exchange URLs, clock monitor) are preserved from the target machine.
                </p>
                <div class={styles.shareActions} style="display:flex; gap:1rem; margin-top:1rem; flex-wrap:wrap;">
                    <a
                        href="/api/workspace/toml"
                        download="config.toml"
                        class={styles.saveBtn}
                        style="text-decoration:none; display:inline-block;"
                    >
                        ⬇ Download config.toml
                    </a>
                    <label class={styles.saveBtn} style="cursor:pointer; display:inline-block; margin:0;">
                        ⬆ Import config.toml
                        <input
                            type="file"
                            accept=".toml"
                            onchange={handleFilePicked}
                            style="display:none;"
                        />
                    </label>
                </div>
                {#if importStatus === 'importing'}
                    <p class={styles.cardSub} style="margin-top:0.75rem;">Importing...</p>
                {:else if importStatus === 'success'}
                    <p class={styles.cardSub} style="margin-top:0.75rem; color: #4caf50;">{importMessage}</p>
                {:else if importStatus === 'error'}
                    <p class={styles.cardSub} style="margin-top:0.75rem; color: #f44336;">{importMessage}</p>
                {/if}
            </div>
        {:else if activeSection === 'fee'}
            <div class={styles.profileCard}>
                <h3>Fee Reference Calculator</h3>
                <p class={styles.cardSub}>Calculate round-trip fees and minimum profit needed to break even</p>
                <div class={styles.calcRow}>
                    <div class={styles.calcField}>
                        <label for="frc-leverage">Leverage</label>
                        <input id="frc-leverage" type="number" min="1" max="150" bind:value={calcLeverage} />
                    </div>
                    <div class={styles.calcField}>
                        <label for="frc-capital">Capital ($)</label>
                        <input id="frc-capital" type="number" min="1" step="100" bind:value={calcCapital} />
                    </div>
                    <div class={styles.calcField}>
                        <label for="frc-fee">Exchange Fee (%)</label>
                        <input id="frc-fee" type="number" min="0" max="10" step="0.01" bind:value={calcFeePct} />
                    </div>
                </div>
                <div class={styles.calcResults}>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Notional Value</span>
                        <span class={styles.calcValue}>{formatUsd(calcNotional)}</span>
                    </div>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Round-Trip Fees</span>
                        <span class="{styles.calcValue} {calcMinProfitPct > 3 ? styles.feeWarn : ''}">{formatUsd(calcFees)}</span>
                    </div>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Min Profit to Cover</span>
                        <span class={styles.calcValue}>{formatUsd(calcFees)} <span class={styles.calcResultSub}>(open + close)</span></span>
                    </div>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Min Profit %</span>
                        <span class="{styles.calcValue} {calcMinProfitPct > 3 ? styles.feeWarn : ''}">{formatPct(calcMinProfitPct)}</span>
                    </div>
                </div>
            </div>
        {:else}
            {#if !loaded}
                <div class={styles.loadingMsg}>Loading settings...</div>
            {:else}
                <div class={styles.profileCard}>
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
                        <span class={styles.fieldHint}>halt workspace after this many</span>
                    </div>
                    <button class={styles.saveBtn} onclick={saveFailover} disabled={saveStatus === 'saving'}>
                        {saveStatus === 'saving' ? 'Saving...' : saveStatus === 'success' ? 'Saved' : 'Save API Failover'}
                    </button>
                </div>
            {/if}
        {/if}
    </div>
</div>
