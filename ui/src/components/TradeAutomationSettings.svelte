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
    import { fetchStrategies } from '../lib/api.svelte';
    import styles from '../styles/engine-dashboard.module.css';

    let { mode }: { mode?: ExecutionMode } = $props();

    interface MinimalTae {
        enabled?: boolean;
        allocation_pct?: number;
        min_net_rr?: number;
        max_position_size_pct_of_equity?: number | null;
        max_open_positions?: number;
        entry_mode?: string;
        invalidate_on?: string;
    }
    interface ExecutionCfg { slippage_ceiling_pct?: number }

    let cfg: { minimal_tae?: MinimalTae; execution?: ExecutionCfg } | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);
    let saveError: string | null = $state(null);
    let saveState = $state<SettingsSaveState>('idle');
    // v9: instance-scoped — bound strategy + vol scale (auto).
    let strategies = $state<{ name: string }[]>([]);
    let boundStrategy = $state<string | null>(null);
    let strategyDirty = $state(false);
    let strategySaving = $state(false);
    let strategyFlash = $state<string | null>(null);

    // Drafts — seeded from the loaded config, compared for dirty state.
    let tae = $state<Required<Pick<MinimalTae, 'enabled' | 'allocation_pct' | 'min_net_rr' | 'max_position_size_pct_of_equity' | 'max_open_positions' | 'entry_mode' | 'invalidate_on'>>>({
        enabled: false,
        allocation_pct: 10,
        min_net_rr: 1,
        max_position_size_pct_of_equity: null,
        max_open_positions: 1,
        entry_mode: 'zone_midpoint',
        invalidate_on: 'direction_flip',
    });
    let exec = $state<ExecutionCfg>({ slippage_ceiling_pct: 0.5 });

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
                    max_position_size_pct_of_equity: data.minimal_tae.max_position_size_pct_of_equity ?? null,
                    max_open_positions: data.minimal_tae.max_open_positions ?? 1,
                    entry_mode: data.minimal_tae.entry_mode ?? 'zone_midpoint',
                    invalidate_on: data.minimal_tae.invalidate_on ?? 'direction_flip',
                };
            }
            if (data.execution) exec = { slippage_ceiling_pct: data.execution.slippage_ceiling_pct ?? 0.5 };
            const instances: { id?: string; strategy?: string | null }[] = data.instances ?? [];
            boundStrategy = instances[0]?.strategy ?? null;
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        void fetchConfig();
        void fetchStrategies()
            .then((list) => (strategies = list))
            .catch(() => {});
    });

    const dirty = $derived.by(() => {
        const c = cfg;
        if (!c) return false;
        return (
            JSON.stringify(tae) !== JSON.stringify({
                enabled: c.minimal_tae?.enabled ?? false,
                allocation_pct: c.minimal_tae?.allocation_pct ?? 10,
                min_net_rr: c.minimal_tae?.min_net_rr ?? 1,
                max_position_size_pct_of_equity: c.minimal_tae?.max_position_size_pct_of_equity ?? null,
                max_open_positions: c.minimal_tae?.max_open_positions ?? 1,
                entry_mode: c.minimal_tae?.entry_mode ?? 'zone_midpoint',
                invalidate_on: c.minimal_tae?.invalidate_on ?? 'direction_flip',
            }) ||
            JSON.stringify(exec) !== JSON.stringify({ slippage_ceiling_pct: c.execution?.slippage_ceiling_pct ?? 0.5 })
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
                        max_position_size_pct_of_equity: Number(tae.max_position_size_pct_of_equity) > 0 ? Number(tae.max_position_size_pct_of_equity) : null,
                        max_open_positions: Number(tae.max_open_positions),
                        entry_mode: tae.entry_mode,
                        invalidate_on: tae.invalidate_on,
                    },
                    execution: { slippage_ceiling_pct: Number(exec.slippage_ceiling_pct) },
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

    async function saveStrategyBinding() {
        if (!boundStrategy) return;
        strategySaving = true;
        const res = await fetch('/api/config', { method: 'POST', headers: { 'Content-Type': 'application/json' } });
        if (!res.ok) {
            strategyFlash = 'Failed to read instance context.';
            strategySaving = false;
            return;
        }
        const data = await res.json();
        const instId = (data.instances ?? [])[0]?.id as string | undefined;
        if (!instId) {
            strategyFlash = 'No instance to bind.';
            strategySaving = false;
            return;
        }
        const post = await fetch(`/api/instances/${encodeURIComponent(instId)}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ strategy: boundStrategy }),
        });
        strategySaving = false;
        if (!post.ok) {
            strategyFlash = (await post.text()) || 'Bind failed';
            return;
        }
        strategyFlash = `Bound to '${boundStrategy}' — full recharge at the next candle boundary; open positions keep their entry params.`;
        strategyDirty = false;
        setTimeout(() => (strategyFlash = null), 5000);
    }

    function buildExport(): string {
        return buildEngineExport('trade_automation', 'settings', mode ?? null, {
            minimal_tae: tae,
            execution: exec,
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
                        <label class={styles.fieldLabel} for="tae-maxsize">Max position size % of equity</label>
                        <input class={styles.fieldInput} id="tae-maxsize" type="number" min="0" max="100" step="1" bind:value={tae.max_position_size_pct_of_equity} />
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
                    <h3 class={styles.cardTitle}>Strategy Binding</h3>
                    <ConfigSourceChip source="[instances.*.strategy]" apply="LIVE" />
                </div>
                <p class={styles.infoLine}>
                    The instance's bound strategy (JSON). Changing it recharges the
                    pipeline fully at the next candle boundary; open positions keep
                    the params they entered with.
                </p>
                <div class={styles.formRow}>
                    <div class={styles.field} style="flex:2">
                        <label class={styles.fieldLabel} for="tae-strategy">Bound strategy</label>
                        <select id="tae-strategy" class={styles.select} bind:value={boundStrategy} onchange={() => (strategyDirty = true)}>
                            <option value={null}></option>
                            {#each strategies as stg (stg.name)}
                                <option value={stg.name}>{stg.name}</option>
                            {/each}
                        </select>
                    </div>
                    <div class={styles.field}>
                        <button class="{styles.btn} {styles.btnPrimary}" disabled={!strategyDirty || strategySaving} onclick={() => void saveStrategyBinding()}>
                            {strategySaving ? 'Binding…' : 'Bind'}
                        </button>
                    </div>
                </div>
                {#if strategyFlash}
                    <p class={styles.infoLine}>{strategyFlash}</p>
                {/if}
                <p class={styles.infoLine}>
                    Volatility scaling (vol_scale) is auto-computed per instance from ATR
                    history; a manual override lands with the strategy editor.
                </p>
            </div>
        {:else}
            <div class={styles.empty}>No configuration available.</div>
        {/if}
    </div>
</div>
