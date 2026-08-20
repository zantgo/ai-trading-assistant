<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { fmtPrice } from '../lib/telemetry';
    import engine from '../styles/engine-dashboard.module.css';
    import styles from './GeneralSettings.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import ExchangeSettings from './ExchangeSettings.svelte';
    import { PROFILE_TABS } from '../lib/engineTabs';

    const app = useAppStore();

    // Section pages are driven by the engine navbar (profile /
    // exchange_settings tab rows). Falls back to `settings` for any
    // middleTab value that is not one of the Home page's sections.
    let section = $derived(
        ['fee', 'exchange', 'share', 'settings'].includes(app.middleTab) ? app.middleTab : 'settings',
    );

    const sectionTitles: Record<string, string> = {
        fee: 'Fee Reference Calculator',
        exchange: 'Exchange Settings',
        share: 'Share Configuration',
        settings: 'General Settings',
    };

    const sectionTabLabel = $derived(
        PROFILE_TABS.find((t) => t.key === section)?.label ?? 'Settings',
    );

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
                draftFailoverMax = config.api_failover.max_consecutive_failures ?? 30;
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
    <header class={engine.unifiedHeader}>
        <div class={engine.headerTop}>
            <div class={engine.titleGroup}>
                <h2 class={engine.title}>{sectionTitles[section]}</h2>
            </div>
            <div class={engine.headerRight}>
                <span class={engine.tabLabel}>{sectionTabLabel}</span>
            </div>
        </div>
    </header>

    <div class={styles.profileContent}>
        {#if section === 'fee'}
            <div class={engine.card}>
                <p class={engine.infoLine}>Calculate round-trip fees and minimum profit needed to break even</p>
                <div class={styles.calcRow}>
                    <div class={styles.calcField}>
                        <label class={engine.fieldLabel} for="frc-leverage">Leverage</label>
                        <input class={engine.fieldInput} id="frc-leverage" type="number" min="1" max="150" bind:value={calcLeverage} />
                    </div>
                    <div class={styles.calcField}>
                        <label class={engine.fieldLabel} for="frc-capital">Capital ($)</label>
                        <input class={engine.fieldInput} id="frc-capital" type="number" min="1" step="100" bind:value={calcCapital} />
                    </div>
                    <div class={styles.calcField}>
                        <label class={engine.fieldLabel} for="frc-fee">Exchange Fee (%)</label>
                        <input class={engine.fieldInput} id="frc-fee" type="number" min="0" max="10" step="0.01" bind:value={calcFeePct} />
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
        {:else if section === 'exchange'}
            <ExchangeSettings />
        {:else if section === 'share'}
            <div class={engine.card}>
                <p class={engine.infoLine}>
                    Download your workspace (instances, timeframes, indicators, fees, safety rules) as a single <code class={engine.code}>config.toml</code> file.
                    Copy it to another machine and start with <code class={engine.code}>--mode headless</code> to run the same setup there.
                    Platform-level fields (exchange URLs, clock monitor) are preserved from the target machine.
                </p>
                <div class={styles.shareActions} style="display:flex; gap:1rem; margin-top:1rem; flex-wrap:wrap;">
                    <a
                        href="/api/workspace/toml"
                        download="config.toml"
                        class="{engine.btn} {engine.btnPrimary}"
                        style="text-decoration:none; display:inline-block;"
                    >
                        <SvgIcon name="upload" size="sm" /> Download config.toml
                    </a>
                    <label class="{engine.btn} {engine.btnPrimary}" style="cursor:pointer; display:inline-block; margin:0;">
                        <SvgIcon name="upload" size="sm" /> Import config.toml
                        <input
                            type="file"
                            accept=".toml"
                            onchange={handleFilePicked}
                            style="display:none;"
                        />
                    </label>
                </div>
                {#if importStatus === 'importing'}
                    <p class={engine.infoLine} style="margin-top:0.75rem;">Importing...</p>
                {:else if importStatus === 'success'}
                    <p class="{engine.pos} {engine.infoLine}" style="margin-top:0.75rem;">{importMessage}</p>
                {:else if importStatus === 'error'}
                    <p class="{engine.neg} {engine.infoLine}" style="margin-top:0.75rem;">{importMessage}</p>
                {/if}
            </div>
        {:else}
            {#if !loaded}
                <div class={styles.loadingMsg}>Loading settings...</div>
            {:else}
                <div class={engine.card}>
                    <div class={engine.formRow}>
                        <div class={engine.field}>
                            <label class={engine.fieldLabel} for="failover-retries">Max Retries Per Call</label>
                            <input class={engine.fieldInput} id="failover-retries" type="number" bind:value={draftFailoverRetries} min="1" max="20" />
                        </div>
                        <div class={engine.field}>
                            <label class={engine.fieldLabel} for="failover-delay">Retry Delay (seconds)</label>
                            <input class={engine.fieldInput} id="failover-delay" type="number" bind:value={draftFailoverDelay} min="1" max="300" />
                        </div>
                        <div class={engine.field}>
                            <label class={engine.fieldLabel} for="failover-max">Max Consecutive Failures</label>
                            <input class={engine.fieldInput} id="failover-max" type="number" bind:value={draftFailoverMax} min="1" max="50" />
                            <span class={styles.fieldHint}>halt workspace after this many</span>
                        </div>
                    </div>
                    <button class="{engine.btn} {engine.btnPrimary}" onclick={saveFailover} disabled={saveStatus === 'saving'}>
                        {saveStatus === 'saving' ? 'Saving...' : saveStatus === 'success' ? 'Saved' : 'Save API Failover'}
                    </button>
                </div>
            {/if}
        {/if}
    </div>
</div>
