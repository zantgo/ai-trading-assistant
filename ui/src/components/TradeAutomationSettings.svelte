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

    // v10: lifecycle-hardening dials — edited on the bound strategy JSON
    // via GET/PUT /api/strategies/:name (partial tae patch).
    interface V10TaeDraft {
        setup_gone_policy: string;
        replace_policy: string;
        min_reprice_delta_atr: number;
        pending_entry_expiry_bars: number | null;
        entry_mode: string;
        chase_max_atr: number;
        chase_score_floor: number;
        instant_fill_policy: string;
        spread_gate_bps: number | null;
        max_setup_age_bars: number | null;
        tp_placement: string;
        sl_mode: string;
        sl_padding_atr: number;
        atr_anchor_mult: number;
        min_sl_atr: number | null;
        tp_refresh_min_rr_delta: number;
        confidence_drop_pct: number | null;
    }
    const V10_DEFAULTS: V10TaeDraft = {
        setup_gone_policy: 'balanced',
        replace_policy: 'cancel_and_adopt',
        min_reprice_delta_atr: 0.25,
        pending_entry_expiry_bars: null,
        entry_mode: 'zone_midpoint',
        chase_max_atr: 0.5,
        chase_score_floor: 75,
        instant_fill_policy: 'take_better',
        spread_gate_bps: null,
        max_setup_age_bars: null,
        tp_placement: 'zone_midpoint',
        sl_mode: 'invalidation',
        sl_padding_atr: 0,
        atr_anchor_mult: 1.5,
        min_sl_atr: null,
        tp_refresh_min_rr_delta: 0.3,
        confidence_drop_pct: null,
    };
    let v10 = $state<V10TaeDraft>({ ...V10_DEFAULTS });
    let v10Loaded = $state<V10TaeDraft | null>(null);
    let v10Saving = $state(false);
    let v10Flash = $state<string | null>(null);

    function isNullNum(v: unknown): number | null {
        const n = Number(v);
        return Number.isFinite(n) && n !== 0 ? n : null;
    }

    async function loadStrategyDials(name: string) {
        v10Flash = null;
        try {
            const res = await fetch(`/api/strategies/${encodeURIComponent(name)}`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const json = await res.json();
            const t = json.tae ?? {};
            const intake = t.intake ?? {};
            const life = t.lifecycle ?? {};
            const exec = t.execution ?? {};
            const risk = t.risk ?? {};
            v10 = {
                setup_gone_policy: risk.setup_gone_policy ?? 'balanced',
                replace_policy: life.replace_policy ?? 'cancel_and_adopt',
                min_reprice_delta_atr: Number(life.min_reprice_delta_atr ?? 0.25),
                pending_entry_expiry_bars: life.pending_entry_expiry_bars ?? null,
                entry_mode: exec.entry_mode ?? 'zone_midpoint',
                chase_max_atr: Number(exec.chase_max_atr ?? 0.5),
                chase_score_floor: Number(exec.chase_score_floor ?? 75),
                instant_fill_policy: exec.instant_fill_policy ?? 'take_better',
                spread_gate_bps: exec.spread_gate_bps ?? null,
                max_setup_age_bars: intake.max_setup_age_bars ?? null,
                tp_placement: exec.tp_placement ?? 'zone_midpoint',
                sl_mode: risk.sl_mode ?? 'invalidation',
                sl_padding_atr: Number(risk.sl_padding_atr ?? 0),
                atr_anchor_mult: Number(risk.atr_anchor_mult ?? 1.5),
                min_sl_atr: risk.min_sl_atr ?? null,
                tp_refresh_min_rr_delta: Number(risk.tp_refresh_min_rr_delta ?? 0.3),
                confidence_drop_pct: risk.confidence_drop_pct ?? null,
            };
            v10Loaded = { ...v10 };
        } catch (e) {
            v10Flash = e instanceof Error ? e.message : String(e);
        }
    }

    const v10Dirty = $derived(
        v10Loaded !== null && JSON.stringify(v10) !== JSON.stringify(v10Loaded),
    );

    async function saveStrategyDials() {
        if (!boundStrategy || v10Saving) return;
        v10Saving = true;
        v10Flash = null;
        try {
            const res = await fetch(`/api/strategies/${encodeURIComponent(boundStrategy)}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name: boundStrategy,
                    strategy: {
                        tae: {
                            intake: {
                                max_setup_age_bars: v10.max_setup_age_bars,
                            },
                            lifecycle: {
                                replace_policy: v10.replace_policy,
                                min_reprice_delta_atr: Number(v10.min_reprice_delta_atr),
                                pending_entry_expiry_bars: v10.pending_entry_expiry_bars,
                            },
                            execution: {
                                entry_mode: v10.entry_mode,
                                chase_max_atr: Number(v10.chase_max_atr),
                                chase_score_floor: Number(v10.chase_score_floor),
                                instant_fill_policy: v10.instant_fill_policy,
                                spread_gate_bps: v10.spread_gate_bps,
                                tp_placement: v10.tp_placement,
                            },
                            risk: {
                                setup_gone_policy: v10.setup_gone_policy,
                                tp_refresh_min_rr_delta: Number(v10.tp_refresh_min_rr_delta),
                                sl_mode: v10.sl_mode,
                                sl_padding_atr: Number(v10.sl_padding_atr),
                                atr_anchor_mult: Number(v10.atr_anchor_mult),
                                min_sl_atr: v10.min_sl_atr,
                                confidence_drop_pct: v10.confidence_drop_pct,
                            },
                        },
                    },
                }),
            });
            if (res.ok) {
                v10Loaded = { ...v10 };
                v10Flash = `Saved to '${boundStrategy}' — applied at the next candle boundary; open positions keep their entry params.`;
                setTimeout(() => (v10Flash = null), 5000);
            } else {
                v10Flash = (await res.text()) || 'Save failed';
            }
        } catch (e) {
            v10Flash = e instanceof Error ? e.message : 'Save failed';
        } finally {
            v10Saving = false;
        }
    }

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
            if (boundStrategy) void loadStrategyDials(boundStrategy);
            else v10Loaded = null;
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
        void loadStrategyDials(boundStrategy);
    }

    function buildExport(): string {
        return buildEngineExport('trade_automation', 'settings', mode ?? null, {
            minimal_tae: tae,
            execution: exec,
            v10_dials: v10,
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
                        <span class={styles.muted} style="font-size:10px">workspace fallback — the strategy dial wins</span>
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

            {#if boundStrategy && v10Loaded !== null}
                <div class={styles.card}>
                    <div class={styles.cardHead}>
                        <h3 class={styles.cardTitle}>Lifecycle Posture</h3>
                        <ConfigSourceChip source="[tae.risk.setup_gone_policy]" apply="LIVE" />
                    </div>
                    <p class={styles.infoLine}>
                        What happens when an actionable setup disappears — while a pending
                        entry waits, and while a position is open. TP always closes the
                        full position.
                    </p>
                    <div class={styles.formRow}>
                        {#each [
                            { v: 'balanced', l: 'BALANCED', d: 'Pending expires after N bars; open holds behind SL/TP/time-stop.' },
                            { v: 'strict', l: 'STRICT', d: 'Pending cancelled; open closed at market (setup_gone).' },
                            { v: 'risky', l: 'RISKY', d: 'Pending immortal; open holds until an opposite flip.' },
                        ] as p (p.v)}
                            <label class="{styles.field}" style="display:flex; flex-direction:row; gap:6px; align-items:center">
                                <input type="radio" name="v10-posture" value={p.v} bind:group={v10.setup_gone_policy} />
                                <span style="font-weight:600">{p.l}</span>
                                <span class={styles.muted} style="font-size:10px">{p.d}</span>
                            </label>
                        {/each}
                    </div>
                    <div class={styles.formRow}>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-expiry">Pending expiry bars (null = posture default)</label>
                            <input class={styles.fieldInput} id="v10-expiry" type="number" min="1" max="1000" step="1" bind:value={v10.pending_entry_expiry_bars} />
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-confdrop">Confidence drop exit (pts, null = off)</label>
                            <input class={styles.fieldInput} id="v10-confdrop" type="number" min="0" max="100" step="1" bind:value={v10.confidence_drop_pct} />
                        </div>
                    </div>
                </div>

                <div class={styles.card}>
                    <div class={styles.cardHead}>
                        <h3 class={styles.cardTitle}>Entry Policy</h3>
                        <ConfigSourceChip source="[tae.execution / tae.intake / tae.lifecycle]" apply="LIVE" />
                    </div>
                    <p class={styles.infoLine}>
                        How strictly the executor prices and accepts entries — from
                        best-price waiting to conditional chasing.
                    </p>
                    <div class={styles.formRow}>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-entrymode">Entry mode</label>
                            <select class={styles.select} id="v10-entrymode" bind:value={v10.entry_mode}>
                                <option value="zone_midpoint">zone_midpoint — zone center limit</option>
                                <option value="zone_edge">zone_edge — best-price edge (strict)</option>
                                <option value="zone_any">zone_any — first-touch edge</option>
                                <option value="market_on_ready">market_on_ready — market order</option>
                                <option value="chase">chase — market if within tolerance + high score</option>
                            </select>
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-fill">Instant-fill policy</label>
                            <select class={styles.select} id="v10-fill" bind:value={v10.instant_fill_policy}>
                                <option value="take_better">take_better — take the better fill</option>
                                <option value="cancel">cancel — refuse beyond-zone dispatch</option>
                            </select>
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-replace">Replacement policy</label>
                            <select class={styles.select} id="v10-replace" bind:value={v10.replace_policy}>
                                <option value="cancel_and_adopt">cancel_and_adopt — adopt same tick</option>
                                <option value="cancel">cancel — v9 behavior</option>
                            </select>
                        </div>
                    </div>
                    <div class={styles.formRow}>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-chaseatr">Chase tolerance (× ATR)</label>
                            <input class={styles.fieldInput} id="v10-chaseatr" type="number" min="0.05" max="10" step="0.05" bind:value={v10.chase_max_atr} />
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-chasescore">Chase score floor</label>
                            <input class={styles.fieldInput} id="v10-chasescore" type="number" min="0" max="100" step="1" bind:value={v10.chase_score_floor} />
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-reprice">Min re-price delta (× ATR)</label>
                            <input class={styles.fieldInput} id="v10-reprice" type="number" min="0" max="10" step="0.05" bind:value={v10.min_reprice_delta_atr} />
                        </div>
                    </div>
                    <div class={styles.formRow}>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-spread">Spread gate (bps, null = off)</label>
                            <input class={styles.fieldInput} id="v10-spread" type="number" min="0" max="1000" step="1" bind:value={v10.spread_gate_bps} />
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-age">Max setup age (bars, null = off)</label>
                            <input class={styles.fieldInput} id="v10-age" type="number" min="0" max="1000" step="1" bind:value={v10.max_setup_age_bars} />
                        </div>
                    </div>
                </div>

                <div class={styles.card}>
                    <div class={styles.cardHead}>
                        <h3 class={styles.cardTitle}>Exit Policy</h3>
                        <ConfigSourceChip source="[tae.risk / tae.execution.tp_placement]" apply="LIVE" />
                    </div>
                    <p class={styles.infoLine}>
                        SL/TP strictness. The SL never widens on a bracket refresh
                        (asymmetric ratchet); the TP closes the full position.
                    </p>
                    <div class={styles.formRow}>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-slmode">SL mode</label>
                            <select class={styles.select} id="v10-slmode" bind:value={v10.sl_mode}>
                                <option value="invalidation">invalidation — exact level (strict)</option>
                                <option value="invalidation_padded">invalidation_padded — level + ATR buffer</option>
                                <option value="atr_anchored">atr_anchored — entry ∓ N × ATR</option>
                            </select>
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-tpplace">TP placement</label>
                            <select class={styles.select} id="v10-tpplace" bind:value={v10.tp_placement}>
                                <option value="zone_near_edge">zone_near_edge — conservative</option>
                                <option value="zone_midpoint">zone_midpoint — balanced</option>
                                <option value="zone_far_edge">zone_far_edge — aggressive</option>
                            </select>
                        </div>
                    </div>
                    <div class={styles.formRow}>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-slpad">SL padding (× ATR)</label>
                            <input class={styles.fieldInput} id="v10-slpad" type="number" min="0" max="10" step="0.05" bind:value={v10.sl_padding_atr} />
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-anchor">ATR anchor (× ATR)</label>
                            <input class={styles.fieldInput} id="v10-anchor" type="number" min="0.1" max="20" step="0.1" bind:value={v10.atr_anchor_mult} />
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-minsl">Min SL distance (× ATR, null = off)</label>
                            <input class={styles.fieldInput} id="v10-minsl" type="number" min="0" max="20" step="0.1" bind:value={v10.min_sl_atr} />
                        </div>
                        <div class={styles.field}>
                            <label class={styles.fieldLabel} for="v10-tprefresh">TP refresh min RR delta</label>
                            <input class={styles.fieldInput} id="v10-tprefresh" type="number" min="0" max="10" step="0.05" bind:value={v10.tp_refresh_min_rr_delta} />
                        </div>
                    </div>
                    <div class={styles.formRow}>
                        <div class={styles.field}>
                            <button class="{styles.btn} {styles.btnPrimary}" disabled={!v10Dirty || v10Saving} onclick={() => void saveStrategyDials()}>
                                {v10Saving ? 'Saving…' : 'Save dials'}
                            </button>
                        </div>
                        {#if v10Flash}
                            <p class={styles.infoLine}>{v10Flash}</p>
                        {/if}
                    </div>
                </div>
            {:else}
                <div class={styles.card}>
                    <div class={styles.cardHead}>
                        <h3 class={styles.cardTitle}>Lifecycle Hardening (v10)</h3>
                        <ConfigSourceChip source="[tae.*]" apply="LIVE" />
                    </div>
                    <p class={styles.infoLine}>
                        Posture, entry and exit dials are per-strategy. Bind a strategy
                        below to edit them here — or open the strategy editor for the
                        raw JSON.
                    </p>
                </div>
            {/if}

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
