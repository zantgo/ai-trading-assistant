<script lang="ts">
    // PortfolioSettings — the PME Settings tab (always present).
    // Editable, config-driven: load `GET /api/config`, edit the safety
    // ladder and risk-limit drafts, save through the extended, validated
    // `POST /api/config`. One header save button (shared state machine).
    import { onMount } from 'svelte';
    import SettingsSaveButton, { type SettingsSaveState } from './SettingsSaveButton.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import ConfigSourceChip from './ConfigSourceChip.svelte';
    import ModeChip from './ModeChip.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import type { ExecutionMode } from '../lib/modePresentation';
    import styles from '../styles/engine-dashboard.module.css';

    let { mode }: { mode?: ExecutionMode } = $props();

    interface SafetyCfg {
        consecutive_loss_caution?: number;
        consecutive_loss_dropout?: number;
        dropout_duration_hours?: number;
        drawdown_limit_pct?: number;
        max_daily_drawdown_pct?: number;
        systemic_risk_threshold?: number;
    }
    interface RiskLimitsCfg {
        max_single_pair_exposure_pct?: number;
        max_portfolio_exposure_pct?: number;
        max_correlation?: number;
    }

    let cfg: { safety?: SafetyCfg; risk_limits?: RiskLimitsCfg } | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);
    let saveError: string | null = $state(null);
    let saveState = $state<SettingsSaveState>('idle');

    let safety = $state<SafetyCfg>({
        consecutive_loss_caution: 3,
        consecutive_loss_dropout: 5,
        dropout_duration_hours: 8,
        drawdown_limit_pct: 30,
        max_daily_drawdown_pct: 5,
        systemic_risk_threshold: 80,
    });
    let limits = $state<RiskLimitsCfg>({
        max_single_pair_exposure_pct: 20,
        max_portfolio_exposure_pct: 50,
        max_correlation: 0.8,
    });

    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            cfg = data;
            if (data.safety) {
                safety = {
                    consecutive_loss_caution: data.safety.consecutive_loss_caution ?? 3,
                    consecutive_loss_dropout: data.safety.consecutive_loss_dropout ?? 5,
                    dropout_duration_hours: data.safety.dropout_duration_hours ?? 8,
                    drawdown_limit_pct: data.safety.drawdown_limit_pct ?? 30,
                    max_daily_drawdown_pct: data.safety.max_daily_drawdown_pct ?? 5,
                    systemic_risk_threshold: data.safety.systemic_risk_threshold ?? 80,
                };
            }
            if (data.risk_limits) {
                limits = {
                    max_single_pair_exposure_pct: data.risk_limits.max_single_pair_exposure_pct ?? 20,
                    max_portfolio_exposure_pct: data.risk_limits.max_portfolio_exposure_pct ?? 50,
                    max_correlation: data.risk_limits.max_correlation ?? 0.8,
                };
            }
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    onMount(fetchConfig);

    const dirty = $derived.by(() => {
        const c = cfg;
        if (!c) return false;
        return (
            JSON.stringify(safety) !== JSON.stringify({
                consecutive_loss_caution: c.safety?.consecutive_loss_caution ?? 3,
                consecutive_loss_dropout: c.safety?.consecutive_loss_dropout ?? 5,
                dropout_duration_hours: c.safety?.dropout_duration_hours ?? 8,
                drawdown_limit_pct: c.safety?.drawdown_limit_pct ?? 30,
                max_daily_drawdown_pct: c.safety?.max_daily_drawdown_pct ?? 5,
                systemic_risk_threshold: c.safety?.systemic_risk_threshold ?? 80,
            }) ||
            JSON.stringify(limits) !== JSON.stringify({
                max_single_pair_exposure_pct: c.risk_limits?.max_single_pair_exposure_pct ?? 20,
                max_portfolio_exposure_pct: c.risk_limits?.max_portfolio_exposure_pct ?? 50,
                max_correlation: c.risk_limits?.max_correlation ?? 0.8,
            })
        );
    });

    $effect(() => {
        if (dirty && saveState !== 'saving' && saveState !== 'error') saveState = 'dirty';
    });

    async function save() {
        if (saveState !== 'dirty' && saveState !== 'error') return;
        saveError = null;
        saveState = 'saving';
        try {
            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    safety: {
                        consecutive_loss_caution: Number(safety.consecutive_loss_caution),
                        consecutive_loss_dropout: Number(safety.consecutive_loss_dropout),
                        dropout_duration_hours: Number(safety.dropout_duration_hours),
                        drawdown_limit_pct: Number(safety.drawdown_limit_pct),
                        max_daily_drawdown_pct: Number(safety.max_daily_drawdown_pct),
                        systemic_risk_threshold: Number(safety.systemic_risk_threshold),
                    },
                    risk_limits: {
                        max_single_pair_exposure_pct: Number(limits.max_single_pair_exposure_pct),
                        max_portfolio_exposure_pct: Number(limits.max_portfolio_exposure_pct),
                        max_correlation: Number(limits.max_correlation),
                    },
                }),
            });
            if (res.ok) {
                await fetchConfig();
                saveState = 'saved';
                setTimeout(() => { saveState = 'idle'; }, 2000);
            } else {
                saveError = (await res.text()) || 'Save failed';
                saveState = 'error';
            }
        } catch (e) {
            saveError = e instanceof Error ? e.message : 'Save failed';
            saveState = 'error';
        }
    }

    function buildExport(): string {
        return buildEngineExport('portfolio', 'settings', mode ?? null, {
            safety,
            risk_limits: limits,
        });
    }
</script>

<div style="display:flex; flex-direction:column; height:100%; background:#000">
    <header class={styles.unifiedHeader}>
        <div class={styles.headerTop}>
            <div class={styles.titleGroup}>
                <h2 class={styles.title}>Portfolio Settings</h2>
            </div>
            <div class={styles.headerRight}>
                <span class={styles.tabLabel}>Settings</span>
                {#if mode}
                    <ModeChip {mode} />
                {/if}
                <SettingsSaveButton state={saveState} onsave={save} />
                <ExportDataButton onExport={buildExport} title="Copy the Portfolio Management configuration as JSON" />
            </div>
        </div>
    </header>

    <div class={styles.content}>
        {#if saveError}
            <div class="{styles.alertBanner} {styles.alertError}">{saveError}</div>
        {/if}
        {#if loading}
            <div class={styles.empty}>Loading…</div>
        {:else if error}
            <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
        {:else if cfg}
            <div class={styles.card}>
                <div class={styles.cardHead}>
                    <h3 class={styles.cardTitle}>Safety Ladder</h3>
                    <ConfigSourceChip source="[workspace.safety]" apply="LIVE" />
                </div>
                <p class={styles.infoLine}>The protection ladder the PME arms — the same thresholds the Safety tab renders. The executor blocks new entries on SUSPENDED / DRAWDOWN_STOP.</p>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-caution">Losses → CAUTIOUS</label>
                        <input class={styles.fieldInput} id="pme-caution" type="number" min="1" max="20" step="1" bind:value={safety.consecutive_loss_caution} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-dropout">Losses → SUSPENDED</label>
                        <input class={styles.fieldInput} id="pme-dropout" type="number" min="2" max="20" step="1" bind:value={safety.consecutive_loss_dropout} />
                        <span class={styles.muted} style="font-size:10px">must exceed the CAUTIOUS rung</span>
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-cooldown">Dropout cooldown (h)</label>
                        <input class={styles.fieldInput} id="pme-cooldown" type="number" min="1" max="168" step="1" bind:value={safety.dropout_duration_hours} />
                    </div>
                </div>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-dd">Drawdown limit %</label>
                        <input class={styles.fieldInput} id="pme-dd" type="number" min="1" max="100" step="1" bind:value={safety.drawdown_limit_pct} />
                        <span class={styles.muted} style="font-size:10px">equity from peak → DRAWDOWN_STOP</span>
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-dd-daily">Max daily drawdown %</label>
                        <input class={styles.fieldInput} id="pme-dd-daily" type="number" min="0.1" max="50" step="0.1" bind:value={safety.max_daily_drawdown_pct} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-systemic">Systemic risk threshold</label>
                        <input class={styles.fieldInput} id="pme-systemic" type="number" min="0" max="100" step="1" bind:value={safety.systemic_risk_threshold} />
                    </div>
                </div>
            </div>

            <div class={styles.card}>
                <div class={styles.cardHead}>
                    <h3 class={styles.cardTitle}>Risk Limits</h3>
                    <ConfigSourceChip source="[workspace.risk_limits]" apply="LIVE" />
                </div>
                <p class={styles.infoLine}>Concentration / exposure / correlation caps the Exposure tab renders and the backend enforces.</p>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-single">Max single-pair exposure %</label>
                        <input class={styles.fieldInput} id="pme-single" type="number" min="1" max="100" step="1" bind:value={limits.max_single_pair_exposure_pct} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-port">Max portfolio exposure %</label>
                        <input class={styles.fieldInput} id="pme-port" type="number" min="1" max="100" step="1" bind:value={limits.max_portfolio_exposure_pct} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="pme-corr">Max correlation</label>
                        <input class={styles.fieldInput} id="pme-corr" type="number" min="0" max="1" step="0.05" bind:value={limits.max_correlation} />
                    </div>
                </div>
            </div>
        {:else}
            <div class={styles.empty}>No configuration available.</div>
        {/if}
    </div>
</div>
