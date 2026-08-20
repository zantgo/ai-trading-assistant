<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import engine from '../styles/engine-dashboard.module.css';
    import styles from './GeneralSettings.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import ExchangeSettings from './ExchangeSettings.svelte';
    import SettingsSaveButton, { type SettingsSaveState } from './SettingsSaveButton.svelte';
    import ConfigSourceChip from './ConfigSourceChip.svelte';
    import { costProjection } from '../lib/costProjection';
    import { PROFILE_TABS } from '../lib/engineTabs';

    const app = useAppStore();

    // Section pages are driven by the engine navbar (profile /
    // exchange_settings tab rows). Falls back to `settings` for any
    // middleTab value that is not one of the Home page's sections.
    let section = $derived(
        ['fee', 'exchange', 'share', 'settings'].includes(app.middleTab) ? app.middleTab : 'settings',
    );

    const sectionTitles: Record<string, string> = {
        fee: 'Fees, Leverage & Cost Projection',
        exchange: 'Exchange Settings',
        share: 'Share Configuration',
        settings: 'General Settings',
    };

    const sectionTabLabel = $derived(
        PROFILE_TABS.find((t) => t.key === section)?.label ?? 'Settings',
    );

    // ─── Fees & Leverage editor — the single source for economics ────────
    interface FeesCfg { maker_fee_pct?: number; taker_fee_pct?: number; funding_rate_8h?: number }
    let feeCfg: { fees?: FeesCfg; leverage?: { cross_leverage?: number } } | null = $state(null);
    let feeDraft = $state({ maker: 0.02, taker: 0.06, funding: 0.01, leverage: 20 });
    let feeLoaded = $state(false);
    let feeSaveState = $state<SettingsSaveState>('idle');
    let feeError: string | null = $state(null);

    async function loadFeeConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) return;
            const data = await res.json();
            feeCfg = data;
            feeDraft = {
                maker: data.fees?.maker_fee_pct ?? 0.02,
                taker: data.fees?.taker_fee_pct ?? 0.06,
                funding: data.fees?.funding_rate_8h ?? 0.01,
                leverage: data.leverage?.cross_leverage ?? 20,
            };
        } catch {
            // Non-fatal: defaults stand.
        } finally {
            feeLoaded = true;
        }
    }

    $effect(() => {
        if (section === 'fee' && !feeLoaded) void loadFeeConfig();
    });

    const feeDirty = $derived.by(() => {
        const c = feeCfg;
        if (!c) return false;
        return JSON.stringify(feeDraft) !== JSON.stringify({
            maker: c.fees?.maker_fee_pct ?? 0.02,
            taker: c.fees?.taker_fee_pct ?? 0.06,
            funding: c.fees?.funding_rate_8h ?? 0.01,
            leverage: c.leverage?.cross_leverage ?? 20,
        });
    });

    $effect(() => {
        if (feeDirty && feeSaveState !== 'saving' && feeSaveState !== 'error') feeSaveState = 'dirty';
    });

    async function saveFee() {
        if (feeSaveState !== 'dirty' && feeSaveState !== 'error') return;
        feeError = null;
        feeSaveState = 'saving';
        try {
            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    fees: {
                        maker_fee_pct: Number(feeDraft.maker),
                        taker_fee_pct: Number(feeDraft.taker),
                        funding_rate_8h: Number(feeDraft.funding),
                    },
                    leverage: { cross_leverage: Number(feeDraft.leverage) },
                }),
            });
            if (res.ok) {
                await loadFeeConfig();
                feeSaveState = 'saved';
                setTimeout(() => { feeSaveState = 'idle'; }, 2000);
            } else {
                feeError = (await res.text()) || 'Save failed';
                feeSaveState = 'error';
            }
        } catch (e) {
            feeError = e instanceof Error ? e.message : 'Save failed';
            feeSaveState = 'error';
        }
    }

    // ─── Cost projection (what-if, config-driven defaults) ───────────────
    let calcCapital = $state(1000);
    let calcLeverage = $state(20);
    let holdPeriods = $state(1);
    let leveragePrefilled = $state(false);

    $effect(() => {
        if (feeLoaded && !leveragePrefilled) {
            calcLeverage = feeDraft.leverage;
            leveragePrefilled = true;
        }
    });

    const projection = $derived(
        costProjection({
            capital: calcCapital,
            leverage: calcLeverage,
            takerFeePct: feeDraft.taker,
            fundingRatePct: feeDraft.funding,
            holdPeriods,
        }),
    );

    function formatUsd(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }

    function formatPct(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '0.00%';
        return v.toFixed(2) + '%';
    }

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

    // ─── API Failover settings (lazy-loaded on the Settings tab) ─────────
    let failoverDraft = $state({ retries: 5, delay: 30, max: 10 });
    let failoverBaseline = $state('');
    let loaded = $state(false);
    let failoverSaveState = $state<SettingsSaveState>('idle');
    let failoverError: string | null = $state(null);

    async function loadFailover() {
        try {
            const res = await fetch('/api/config');
            const config = await res.json();
            if (config.api_failover) {
                failoverDraft = {
                    retries: config.api_failover.max_retries_per_call ?? 5,
                    delay: config.api_failover.retry_delay_seconds ?? 30,
                    max: config.api_failover.max_consecutive_failures ?? 30,
                };
                failoverBaseline = JSON.stringify(failoverDraft);
            }
            loaded = true;
        } catch (e) {
            console.error('Failed to load settings:', e);
        }
    }

    $effect(() => {
        if (section === 'settings' && !loaded) void loadFailover();
    });

    const failoverDirty = $derived(failoverBaseline !== '' && JSON.stringify(failoverDraft) !== failoverBaseline);

    $effect(() => {
        if (failoverDirty && failoverSaveState !== 'saving' && failoverSaveState !== 'error') failoverSaveState = 'dirty';
    });

    async function saveFailover() {
        if (failoverSaveState !== 'dirty' && failoverSaveState !== 'error') return;
        failoverError = null;
        failoverSaveState = 'saving';
        try {
            const res = await fetch('/api/config');
            const config = await res.json();
            config.api_failover = {
                max_retries_per_call: Number(failoverDraft.retries),
                retry_delay_seconds: Number(failoverDraft.delay),
                max_consecutive_failures: Number(failoverDraft.max),
            };
            const saveRes = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(config),
            });
            if (saveRes.ok) {
                failoverBaseline = JSON.stringify(failoverDraft);
                failoverSaveState = 'saved';
                setTimeout(() => { failoverSaveState = 'idle'; }, 2000);
            } else {
                failoverError = (await saveRes.text()) || 'Save failed';
                failoverSaveState = 'error';
            }
        } catch (e) {
            failoverError = e instanceof Error ? e.message : 'Save failed';
            failoverSaveState = 'error';
        }
    }
</script>

<div class={styles.profileLayout}>
    <header class={engine.unifiedHeader}>
        <div class={engine.headerTop}>
            <div class={engine.titleGroup}>
                <h2 class={engine.title}>{sectionTitles[section]}</h2>
            </div>
            <div class={engine.headerRight}>
                <span class={engine.tabLabel}>{sectionTabLabel}</span>
                {#if section === 'fee'}
                    <SettingsSaveButton state={feeSaveState} onsave={saveFee} />
                {:else if section === 'settings'}
                    <SettingsSaveButton state={failoverSaveState} onsave={saveFailover} />
                {/if}
            </div>
        </div>
    </header>

    <div class={styles.profileContent}>
        {#if section === 'fee'}
            {#if feeError}
                <div class="{engine.alertBanner} {engine.alertError}">{feeError}</div>
            {/if}

            <div class={engine.card}>
                <div class={engine.cardHead}>
                    <h3 class={engine.cardTitle}>Fees &amp; Leverage</h3>
                    <ConfigSourceChip source="[workspace.fees] · [workspace.leverage]" apply="LIVE" />
                </div>
                <p class={engine.infoLine}>
                    The single editor for economics — every engine surface (TAE / PME / PAE settings,
                    P&amp;L projections) reads these values.
                </p>
                <div class={engine.formRow}>
                    <div class={engine.field}>
                        <label class={engine.fieldLabel} for="fee-maker">Maker fee %</label>
                        <input class={engine.fieldInput} id="fee-maker" type="number" min="0" max="5" step="0.01" bind:value={feeDraft.maker} />
                    </div>
                    <div class={engine.field}>
                        <label class={engine.fieldLabel} for="fee-taker">Taker fee %</label>
                        <input class={engine.fieldInput} id="fee-taker" type="number" min="0" max="5" step="0.01" bind:value={feeDraft.taker} />
                    </div>
                    <div class={engine.field}>
                        <label class={engine.fieldLabel} for="fee-funding">Funding rate 8h %</label>
                        <input class={engine.fieldInput} id="fee-funding" type="number" min="0" max="2" step="0.01" bind:value={feeDraft.funding} />
                    </div>
                    <div class={engine.field}>
                        <label class={engine.fieldLabel} for="fee-lev">Cross leverage</label>
                        <input class={engine.fieldInput} id="fee-lev" type="number" min="1" max="150" step="1" bind:value={feeDraft.leverage} />
                    </div>
                </div>
            </div>

            <div class={engine.card}>
                <div class={engine.cardHead}>
                    <h3 class={engine.cardTitle}>Cost Projection</h3>
                    <ConfigSourceChip source="taker + funding from above" />
                </div>
                <p class={engine.infoLine}>
                    Round-trip fees plus the 8h funding drag a perpetual position pays while held.
                    Uses the saved taker fee and funding rate above.
                </p>
                <div class={styles.calcRow}>
                    <div class={styles.calcField}>
                        <label class={engine.fieldLabel} for="proj-capital">Capital ($)</label>
                        <input class={engine.fieldInput} id="proj-capital" type="number" min="1" step="100" bind:value={calcCapital} />
                    </div>
                    <div class={styles.calcField}>
                        <label class={engine.fieldLabel} for="proj-leverage">Leverage (what-if)</label>
                        <input class={engine.fieldInput} id="proj-leverage" type="number" min="1" max="150" step="1" bind:value={calcLeverage} />
                    </div>
                    <div class={styles.calcField}>
                        <label class={engine.fieldLabel} for="proj-hold">Expected hold (8h periods)</label>
                        <input class={engine.fieldInput} id="proj-hold" type="number" min="1" max="30" step="1" bind:value={holdPeriods} />
                    </div>
                </div>
                <div class={styles.calcResults}>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Notional Value</span>
                        <span class={styles.calcValue}>{formatUsd(projection.notional)}</span>
                    </div>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Round-Trip Fees</span>
                        <span class={styles.calcValue}>{formatUsd(projection.roundTripFees)}</span>
                    </div>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Funding Drag ({holdPeriods} × 8h)</span>
                        <span class={styles.calcValue}>{formatUsd(projection.fundingDrag)}</span>
                    </div>
                    <div class={styles.calcResultItem}>
                        <span class={styles.calcLabel}>Min Profit to Break Even</span>
                        <span class="{styles.calcValue} {projection.minProfitPct > 3 ? styles.feeWarn : ''}">
                            {formatPct(projection.minProfitPct)}
                            <span class={styles.calcResultSub}>(fees {formatUsd(projection.roundTripFees)} + funding {formatUsd(projection.fundingDrag)})</span>
                        </span>
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
            {#if failoverError}
                <div class="{engine.alertBanner} {engine.alertError}">{failoverError}</div>
            {/if}
            {#if !loaded}
                <div class={styles.loadingMsg}>Loading settings...</div>
            {:else}
                <div class={engine.card}>
                    <div class={engine.cardHead}>
                        <h3 class={engine.cardTitle}>API Failover</h3>
                        <ConfigSourceChip source="[workspace.api_failover]" apply="NEW_PIPELINES" />
                    </div>
                    <p class={engine.infoLine}>
                        REST call resilience policy. Read when pipelines are built — changes apply to
                        newly launched instances.
                    </p>
                    <div class={engine.formRow}>
                        <div class={engine.field}>
                            <label class={engine.fieldLabel} for="failover-retries">Max Retries Per Call</label>
                            <input class={engine.fieldInput} id="failover-retries" type="number" bind:value={failoverDraft.retries} min="1" max="20" />
                        </div>
                        <div class={engine.field}>
                            <label class={engine.fieldLabel} for="failover-delay">Retry Delay (seconds)</label>
                            <input class={engine.fieldInput} id="failover-delay" type="number" bind:value={failoverDraft.delay} min="1" max="300" />
                        </div>
                        <div class={engine.field}>
                            <label class={engine.fieldLabel} for="failover-max">Max Consecutive Failures</label>
                            <input class={engine.fieldInput} id="failover-max" type="number" bind:value={failoverDraft.max} min="1" max="50" />
                            <span class={styles.fieldHint}>halt workspace after this many</span>
                        </div>
                    </div>
                </div>
            {/if}
        {/if}
    </div>
</div>
