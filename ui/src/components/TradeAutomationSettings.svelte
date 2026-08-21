<script lang="ts">
    // TradeAutomationSettings — the TAE Settings tab (always present).
    // Every value is editable and config-driven: load `GET /api/config`,
    // edit drafts, save through the extended `POST /api/config` (validated
    // server-side, M8 ranges mirrored here). One save button in the header
    // with the shared idle → dirty → saving → saved | error state machine.
    import { onMount } from 'svelte';
    import SettingsSaveButton, { type SettingsSaveState } from './SettingsSaveButton.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import ConfigSourceChip from './ConfigSourceChip.svelte';
    import ModeChip from './ModeChip.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import type { ExecutionMode } from '../lib/modePresentation';
    import styles from '../styles/engine-dashboard.module.css';

    let { mode }: { mode?: ExecutionMode } = $props();

    interface MinimalTae {
        enabled?: boolean;
        allocation_pct?: number;
        min_net_rr?: number;
        max_position_size_usd?: number | null;
        max_open_positions?: number;
        entry_mode?: string;
        invalidate_on?: string;
    }
    interface ExecutionCfg { slippage_ceiling_pct?: number }
    interface ScoringCfg {
        base_allocation_pct?: number;
        micro_allocation_pct?: number;
        max_allocation_pct?: number;
        base_score_threshold?: number;
        micro_score_threshold?: number;
    }

    let cfg: { minimal_tae?: MinimalTae; execution?: ExecutionCfg; scoring?: ScoringCfg } | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);
    let saveError: string | null = $state(null);
    let saveState = $state<SettingsSaveState>('idle');

    // Drafts — seeded from the loaded config, compared for dirty state.
    let tae = $state<Required<Pick<MinimalTae, 'enabled' | 'allocation_pct' | 'min_net_rr' | 'max_position_size_usd' | 'max_open_positions' | 'entry_mode' | 'invalidate_on'>>>({
        enabled: false,
        allocation_pct: 10,
        min_net_rr: 1,
        max_position_size_usd: null,
        max_open_positions: 1,
        entry_mode: 'zone_midpoint',
        invalidate_on: 'direction_flip',
    });
    let exec = $state<ExecutionCfg>({ slippage_ceiling_pct: 0.5 });
    let scoring = $state<ScoringCfg>({
        base_allocation_pct: 1,
        micro_allocation_pct: 2,
        max_allocation_pct: 3,
        base_score_threshold: 40,
        micro_score_threshold: 60,
    });

    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            cfg = data;
            if (data.minimal_tae) {
                tae = {
                    enabled: data.minimal_tae.enabled ?? false,
                    allocation_pct: data.minimal_tae.allocation_pct ?? 10,
                    min_net_rr: data.minimal_tae.min_net_rr ?? 1,
                    max_position_size_usd: data.minimal_tae.max_position_size_usd ?? null,
                    max_open_positions: data.minimal_tae.max_open_positions ?? 1,
                    entry_mode: data.minimal_tae.entry_mode ?? 'zone_midpoint',
                    invalidate_on: data.minimal_tae.invalidate_on ?? 'direction_flip',
                };
            }
            if (data.execution) exec = { slippage_ceiling_pct: data.execution.slippage_ceiling_pct ?? 0.5 };
            if (data.scoring) {
                scoring = {
                    base_allocation_pct: data.scoring.base_allocation_pct ?? 1,
                    micro_allocation_pct: data.scoring.micro_allocation_pct ?? 2,
                    max_allocation_pct: data.scoring.max_allocation_pct ?? 3,
                    base_score_threshold: data.scoring.base_score_threshold ?? 40,
                    micro_score_threshold: data.scoring.micro_score_threshold ?? 60,
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
            JSON.stringify(tae) !== JSON.stringify({
                enabled: c.minimal_tae?.enabled ?? false,
                allocation_pct: c.minimal_tae?.allocation_pct ?? 10,
                min_net_rr: c.minimal_tae?.min_net_rr ?? 1,
                max_position_size_usd: c.minimal_tae?.max_position_size_usd ?? null,
                max_open_positions: c.minimal_tae?.max_open_positions ?? 1,
                entry_mode: c.minimal_tae?.entry_mode ?? 'zone_midpoint',
                invalidate_on: c.minimal_tae?.invalidate_on ?? 'direction_flip',
            }) ||
            JSON.stringify(exec) !== JSON.stringify({ slippage_ceiling_pct: c.execution?.slippage_ceiling_pct ?? 0.5 }) ||
            JSON.stringify(scoring) !== JSON.stringify({
                base_allocation_pct: c.scoring?.base_allocation_pct ?? 1,
                micro_allocation_pct: c.scoring?.micro_allocation_pct ?? 2,
                max_allocation_pct: c.scoring?.max_allocation_pct ?? 3,
                base_score_threshold: c.scoring?.base_score_threshold ?? 40,
                micro_score_threshold: c.scoring?.micro_score_threshold ?? 60,
            })
        );
    });

    $effect(() => {
        if (dirty && saveState !== 'saving' && saveState !== 'error') saveState = 'dirty';
    });

    function fmtPct(v: number | undefined): string {
        return v != null ? `${v.toFixed(2)}%` : '—';
    }

    async function save() {
        if (saveState !== 'dirty' && saveState !== 'error') return;
        saveError = null;
        saveState = 'saving';
        try {
            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    minimal_tae: {
                        enabled: tae.enabled,
                        allocation_pct: Number(tae.allocation_pct),
                        min_net_rr: Number(tae.min_net_rr),
                        max_position_size_usd: Number(tae.max_position_size_usd) > 0 ? Number(tae.max_position_size_usd) : null,
                        max_open_positions: Number(tae.max_open_positions),
                        entry_mode: tae.entry_mode,
                        invalidate_on: tae.invalidate_on,
                    },
                    execution: { slippage_ceiling_pct: Number(exec.slippage_ceiling_pct) },
                    scoring: {
                        base_allocation_pct: Number(scoring.base_allocation_pct),
                        micro_allocation_pct: Number(scoring.micro_allocation_pct),
                        max_allocation_pct: Number(scoring.max_allocation_pct),
                        base_score_threshold: Number(scoring.base_score_threshold),
                        micro_score_threshold: Number(scoring.micro_score_threshold),
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
        return buildEngineExport('trade_automation', 'settings', mode ?? null, {
            minimal_tae: tae,
            execution: exec,
            scoring,
        });
    }
</script>

<div style="display:flex; flex-direction:column; height:100%; background:#000">
    <header class={styles.unifiedHeader}>
        <div class={styles.headerTop}>
            <div class={styles.titleGroup}>
                <h2 class={styles.title}>Trade Automation Settings</h2>
            </div>
            <div class={styles.headerRight}>
                <span class={styles.tabLabel}>Settings</span>
                {#if mode}
                    <ModeChip {mode} />
                {/if}
                <SettingsSaveButton state={saveState} onsave={save} />
                <ExportDataButton onExport={buildExport} title="Copy the Trade Automation configuration as JSON" />
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
                    <h3 class={styles.cardTitle}>Setup Executor</h3>
                    <ConfigSourceChip source="[workspace.minimal_tae]" apply="LIVE" />
                </div>
                <p class={styles.infoLine}>How the executor accepts, sizes and manages setups. Applied to the next evaluation.</p>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-enabled">Enabled</label>
                        <select class={styles.select} id="tae-enabled" bind:value={tae.enabled}>
                            <option value={true}>On</option>
                            <option value={false}>Off</option>
                        </select>
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-alloc">Allocation % per position (1–100)</label>
                        <input class={styles.fieldInput} id="tae-alloc" type="number" min="1" max="100" step="1" bind:value={tae.allocation_pct} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-rr">Min net R:R</label>
                        <input class={styles.fieldInput} id="tae-rr" type="number" min="0" max="20" step="0.1" bind:value={tae.min_net_rr} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-maxsize">Max position size $</label>
                        <input class={styles.fieldInput} id="tae-maxsize" type="number" min="0" step="10" bind:value={tae.max_position_size_usd} />
                        <span class={styles.muted} style="font-size:10px">0 = no cap</span>
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-maxpos">Max open positions</label>
                        <input class={styles.fieldInput} id="tae-maxpos" type="number" min="1" max="100" step="1" bind:value={tae.max_open_positions} />
                    </div>
                </div>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-entry">Entry mode</label>
                        <input class={styles.fieldInput} id="tae-entry" type="text" bind:value={tae.entry_mode} spellcheck="false" />
                        <span class={styles.muted} style="font-size:10px">v7 supports zone_midpoint</span>
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-invalidate">Invalidate on</label>
                        <input class={styles.fieldInput} id="tae-invalidate" type="text" bind:value={tae.invalidate_on} spellcheck="false" />
                        <span class={styles.muted} style="font-size:10px">default: direction_flip</span>
                    </div>
                </div>
            </div>

            <div class={styles.card}>
                <div class={styles.cardHead}>
                    <h3 class={styles.cardTitle}>Execution</h3>
                    <ConfigSourceChip source="[workspace.execution]" apply="LIVE" />
                </div>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-slip">Slippage ceiling %</label>
                        <input class={styles.fieldInput} id="tae-slip" type="number" min="0" max="5" step="0.05" bind:value={exec.slippage_ceiling_pct} />
                        <span class={styles.muted} style="font-size:10px">worst-case fill tolerance (0.5 = 0.5%)</span>
                    </div>
                </div>
            </div>

            <div class={styles.card}>
                <div class={styles.cardHead}>
                    <h3 class={styles.cardTitle}>Allocation Scoring</h3>
                    <ConfigSourceChip source="[workspace.scoring]" apply="LIVE" />
                </div>
                <p class={styles.infoLine}>Confluence-score → allocation mapping for the position sizing protocol.</p>
                <div class={styles.formRow}>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-base-all">Base allocation %</label>
                        <input class={styles.fieldInput} id="tae-base-all" type="number" min="0.01" max="100" step="0.1" bind:value={scoring.base_allocation_pct} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-micro-all">Micro allocation %</label>
                        <input class={styles.fieldInput} id="tae-micro-all" type="number" min="0.01" max="100" step="0.1" bind:value={scoring.micro_allocation_pct} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-max-all">Max allocation %</label>
                        <input class={styles.fieldInput} id="tae-max-all" type="number" min="0.01" max="100" step="0.1" bind:value={scoring.max_allocation_pct} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-base-th">Base score threshold</label>
                        <input class={styles.fieldInput} id="tae-base-th" type="number" min="0" max="100" step="1" bind:value={scoring.base_score_threshold} />
                    </div>
                    <div class={styles.field}>
                        <label class={styles.fieldLabel} for="tae-micro-th">Micro score threshold</label>
                        <input class={styles.fieldInput} id="tae-micro-th" type="number" min="0" max="100" step="1" bind:value={scoring.micro_score_threshold} />
                    </div>
                </div>
                <p class={styles.infoLine}>Current saved values — {cfg.minimal_tae?.allocation_pct ?? 10}% allocation per position · {fmtPct(cfg.execution?.slippage_ceiling_pct)} slippage ceiling.</p>
            </div>
        {:else}
            <div class={styles.empty}>No configuration available.</div>
        {/if}
    </div>
</div>
