<script lang="ts">
    import { onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import { useEdgeStore, AVAILABLE_INDICATORS } from '../stores/edges.svelte';
    import Icon from '../lib/Icon.svelte';
    import type { IconName } from '../lib/icons';
    import type { SizingModel, StopLossModel, TriggerPhase, EdgeArchetype, TriggerRule } from '../types';
    import styles from './EdgeBuilder.module.css';

    let { paradigm = 'rule' }: { paradigm?: 'rule' | 'ai' } = $props();

    const app = useAppStore();
    const edge = useEdgeStore();

    let activeIndicatorDropdown: string | null = $state(null);
    let strategyNameInput = $state('');
    let strategyDescriptionInput = $state('');

    const pair = app.activeInstance();

    $effect(() => {
        strategyNameInput = edge.draftName;
        strategyDescriptionInput = edge.draftDescription;
    });

    function setName() {
        edge.draftName = strategyNameInput;
    }

    function setDescription() {
        edge.draftDescription = strategyDescriptionInput;
    }

    function toggleRegime(key: keyof typeof edge.draftConfig.regime_gates) {
        edge.draftConfig.regime_gates[key] = !edge.draftConfig.regime_gates[key];
    }

    function regimeIcon(r: string): IconName {
        if (r === 'trending') return 'trending-up';
        if (r === 'compression') return 'refresh';
        if (r === 'expansion') return 'zap';
        return 'minus';
    }

    function addIndicator(indName: string) {
        const existing = edge.draftConfig.indicators.find(i => i.name === indName);
        if (existing) {
            existing.enabled = !existing.enabled;
        } else {
            const info = AVAILABLE_INDICATORS.find(i => i.name === indName);
            edge.draftConfig.indicators.push({
                name: indName,
                weight: 10,
                trigger_rule: info?.defaultTrigger || 'threshold_above',
                enabled: true,
            });
        }
        activeIndicatorDropdown = null;
    }

    function removeIndicator(indName: string) {
        edge.draftConfig.indicators = edge.draftConfig.indicators.filter(i => i.name !== indName);
    }

    function updateIndicatorWeight(indName: string, weight: number) {
        const ind = edge.draftConfig.indicators.find(i => i.name === indName);
        if (ind) ind.weight = Math.max(0, Math.min(50, weight));
    }

    function updateIndicatorRule(indName: string, rule: TriggerRule) {
        const ind = edge.draftConfig.indicators.find(i => i.name === indName);
        if (ind) ind.trigger_rule = rule;
    }

    function addMtfQuorum(tf: string) {
        if (edge.draftConfig.mtf_quorum.includes(tf)) {
            edge.draftConfig.mtf_quorum = edge.draftConfig.mtf_quorum.filter(t => t !== tf);
        } else {
            edge.draftConfig.mtf_quorum.push(tf);
        }
    }

    async function handleSave() {
        const ok = await edge.saveEdge(app.pairKeyFor(pair.symbol), app.sessionUserName || undefined);
        if (ok) {
            await edge.fetchEdges(app.pairKeyFor(pair.symbol));
        }
    }

    async function handleLoad(id: number) {
        const saved = edge.savedEdges.find(e => e.id === id);
        if (saved) {
            edge.loadConfig(saved);
            strategyNameInput = edge.draftName;
            strategyDescriptionInput = edge.draftDescription;
        }
    }

    async function handleDelete(id: number) {
        if (confirm('Delete this strategy?')) {
            await edge.deleteEdge(id, app.pairKeyFor(pair.symbol));
        }
    }

    function handleNew() {
        edge.resetDraft();
        strategyNameInput = '';
        strategyDescriptionInput = '';
    }

    onMount(() => {
        edge.fetchEdges(app.pairKeyFor(pair.symbol));
    });
</script>

<div class={styles.edgeBuilder}>
    <div class={styles.header}>
        <h2>Edge Builder <span class={styles.paradigmBadge}>{paradigm === 'ai' ? 'AI-Driven' : 'Rule-Based'}</span></h2>
        <div class={styles.headerActions}>
            <button class={styles.btnOutline} onclick={handleNew}>+ New</button>
            <button class={styles.btnPrimary} onclick={handleSave}>{edge.saveStatus === 'saving' ? 'Saving...' : 'Save Strategy'}</button>
            <button class={styles.btnOutline} onclick={() => edge.exportConfig()}>Export JSON</button>
        </div>
    </div>

    {#if paradigm === 'ai'}
        <div class={styles.paradigmNote}>
            <Icon name="bot" size={14} /> AI-Driven configuration: evaluator prompts, agent confidence weights, and context-memory depth augment the indicator thresholds below.
        </div>
    {:else}
        <div class={styles.paradigmNote}>
            <Icon name="tool" size={14} /> Rule-Based configuration: strategy executes on deterministic indicator thresholds and regime gates below.
        </div>
    {/if}

    {#if edge.error}
        <div class={styles.errorBanner}>{edge.error}</div>
    {/if}
    {#if edge.saveStatus === 'saved'}
        <div class={styles.successBanner}>Strategy saved successfully!</div>
    {/if}

    {#if edge.savedEdges.length > 0}
        <div class={styles.savedList}>
            <span class={styles.sectionLabel}>Saved Strategies:</span>
            {#each edge.savedEdges as saved (saved.id)}
                <div class={styles.savedItem} class:active={edge.activeEdgeId === saved.id}>
                    <button type="button" class={styles.savedName} onclick={() => handleLoad(saved.id)}>{saved.name}</button>
                    <button class={styles.deleteBtn} onclick={() => handleDelete(saved.id)} title="Delete">x</button>
                </div>
            {/each}
        </div>
    {/if}

    <div class={styles.columns}>
        <!-- Column 1: Alpha Matrix -->
        <div class={styles.column}>
            <h3>Alpha Matrix & Regime</h3>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Archetype</span>
                <div class={styles.segmentedGroup}>
                    <button
                        class={styles.segmentBtn}
                        class:active={edge.draftConfig.archetype === 'trend_following'}
                        onclick={() => edge.draftConfig.archetype = 'trend_following'}
                    >Trend Following</button>
                    <button
                        class={styles.segmentBtn}
                        class:active={edge.draftConfig.archetype === 'mean_reversion'}
                        onclick={() => edge.draftConfig.archetype = 'mean_reversion'}
                    >Mean Reversion</button>
                </div>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Regime Gates</span>
                <div class={styles.regimeGrid}>
                    {#each (['trending', 'compression', 'expansion', 'range'] as const) as regime}
                        <button
                            class={styles.regimeCard}
                            class:active={edge.draftConfig.regime_gates[regime]}
                            onclick={() => toggleRegime(regime)}
                        >
                            <span class={styles.regimeIcon}>
                                <Icon name={regimeIcon(regime)} size={16} />
                            </span>
                            <span>{regime.charAt(0).toUpperCase() + regime.slice(1)}</span>
                        </button>
                    {/each}
                </div>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>
                    Confluence Quorum Threshold: <strong>{edge.draftConfig.quorum_threshold}</strong> pts
                </span>
                <input
                    type="range" min="0" max="100" value={edge.draftConfig.quorum_threshold}
                    oninput={(e: Event) => edge.draftConfig.quorum_threshold = parseFloat((e.target as HTMLInputElement).value)}
                />
                <div class={styles.rangeLabels}>
                    <span>0</span><span>50</span><span>100</span>
                </div>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Multi-Timeframe Quorum</span>
                <div class={styles.checkboxRow}>
                    {#each ['micro', 'fast', 'slow', 'macro'] as tf}
                        <label>
                            <input
                                type="checkbox"
                                checked={edge.draftConfig.mtf_quorum.includes(tf)}
                                onchange={() => addMtfQuorum(tf)}
                            />
                            {tf}
                        </label>
                    {/each}
                </div>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Indicators</span>
                <div class={styles.indicatorList}>
                    {#each AVAILABLE_INDICATORS as indInfo}
                        {@const active = edge.draftConfig.indicators.find(i => i.name === indInfo.name)}
                        <div class={styles.indicatorRow} class:enabled={active?.enabled}>
                            <label class={styles.indicatorCheck}>
                                <input
                                    type="checkbox"
                                    checked={active?.enabled || false}
                                    onchange={() => addIndicator(indInfo.name)}
                                />
                                {indInfo.label}
                            </label>
                            {#if active}
                                <div class={styles.indicatorControls}>
                                    <input
                                        type="number" min="0" max="50"
                                        value={active.weight}
                                        oninput={(e: Event) => updateIndicatorWeight(indInfo.name, parseInt((e.target as HTMLInputElement).value) || 0)}
                                        class={styles.weightInput}
                                    />
                                    <span class={styles.weightLabel}>pts</span>
                                    <select
                                        value={active.trigger_rule}
                                        onchange={(e: Event) => updateIndicatorRule(indInfo.name, (e.target as HTMLSelectElement).value as TriggerRule)}
                                        class={styles.ruleSelect}
                                    >
                                        <option value="crossover">Crossover</option>
                                        <option value="overbought_oversold">Overbought/Oversold</option>
                                        <option value="divergence">Divergence</option>
                                        <option value="slope_direction">Slope Direction</option>
                                        <option value="threshold_above">Threshold Above</option>
                                        <option value="threshold_below">Threshold Below</option>
                                        <option value="release">Release</option>
                                    </select>
                                </div>
                            {/if}
                        </div>
                    {/each}
                </div>
            </div>
        </div>

        <!-- Column 2: Execution & Risk -->
        <div class={styles.column}>
            <h3>Execution & Risk</h3>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Sizing Model</span>
                <div class={styles.segmentedGroup}>
                    <button
                        class={styles.segmentBtn}
                        class:active={edge.draftConfig.sizing.model === 'fixed'}
                        onclick={() => edge.draftConfig.sizing.model = 'fixed'}
                    >Fixed (25%)</button>
                    <button
                        class={styles.segmentBtn}
                        class:active={edge.draftConfig.sizing.model === 'volatility_targeting'}
                        onclick={() => edge.draftConfig.sizing.model = 'volatility_targeting'}
                    >Vol Targeting</button>
                </div>
            </div>

            {#if edge.draftConfig.sizing.model === 'volatility_targeting'}
                <div class={styles.section}>
                    <span class={styles.sectionLabel}>
                        Daily Vol Target: <strong>{edge.draftConfig.sizing.daily_vol_target_pct.toFixed(1)}%</strong>
                    </span>
                    <input
                        type="range" min="0.1" max="5.0" step="0.1"
                        value={edge.draftConfig.sizing.daily_vol_target_pct}
                        oninput={(e: Event) => edge.draftConfig.sizing.daily_vol_target_pct = parseFloat((e.target as HTMLInputElement).value)}
                    />
                </div>

                <div class={styles.section}>
                    <span class={styles.sectionLabel}>
                        Max Leverage: <strong>{edge.draftConfig.sizing.max_leverage}x</strong>
                    </span>
                    <input
                        type="range" min="1" max="20" step="1"
                        value={edge.draftConfig.sizing.max_leverage}
                        oninput={(e: Event) => edge.draftConfig.sizing.max_leverage = parseInt((e.target as HTMLInputElement).value)}
                    />
                </div>
            {/if}

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Stop-Loss Model</span>
                <select
                    value={edge.draftConfig.stop_loss.model}
                    onchange={(e: Event) => edge.draftConfig.stop_loss.model = (e.target as HTMLSelectElement).value as StopLossModel}
                    class={styles.selectFull}
                >
                    <option value="atr_volatility_stop">ATR Volatility Stop</option>
                    <option value="structural_pivot">Structural Pivot Low/High</option>
                    <option value="fixed_percentage">Fixed Percentage</option>
                </select>
            </div>

            {#if edge.draftConfig.stop_loss.model === 'atr_volatility_stop'}
                <div class={styles.section}>
                    <span class={styles.sectionLabel}>
                        ATR Multiplier: <strong>{edge.draftConfig.stop_loss.atr_multiplier.toFixed(1)}x</strong>
                    </span>
                    <input
                        type="range" min="1.0" max="5.0" step="0.1"
                        value={edge.draftConfig.stop_loss.atr_multiplier}
                        oninput={(e: Event) => edge.draftConfig.stop_loss.atr_multiplier = parseFloat((e.target as HTMLInputElement).value)}
                    />
                </div>
            {/if}

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Take-Profit (ATR Multipliers)</span>
                <div class={styles.tpRow}>
                    <label>TP1 <input type="number" min="0" max="20" step="0.5" value={edge.draftConfig.take_profit.tp1_multiplier} oninput={(e: Event) => edge.draftConfig.take_profit.tp1_multiplier = parseFloat((e.target as HTMLInputElement).value) || 0} class={styles.numInput} /></label>
                    <label>TP2 <input type="number" min="0" max="20" step="0.5" value={edge.draftConfig.take_profit.tp2_multiplier} oninput={(e: Event) => edge.draftConfig.take_profit.tp2_multiplier = parseFloat((e.target as HTMLInputElement).value) || 0} class={styles.numInput} /></label>
                    <label>TP3 <input type="number" min="0" max="20" step="0.5" value={edge.draftConfig.take_profit.tp3_multiplier} oninput={(e: Event) => edge.draftConfig.take_profit.tp3_multiplier = parseFloat((e.target as HTMLInputElement).value) || 0} class={styles.numInput} /></label>
                </div>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Execution Gates</span>
                <div class={styles.gateRow}>
                    <label for="eb-min-rvol">Min RVOL</label>
                    <input type="number" min="0" max="10" step="0.1" id="eb-min-rvol" value={edge.draftConfig.execution.min_rvol} oninput={(e: Event) => edge.draftConfig.execution.min_rvol = parseFloat((e.target as HTMLInputElement).value) || 0} class={styles.numInput} />
                </div>
                <div class={styles.gateRow}>
                    <label for="eb-climax-rvol">Climax RVOL Block</label>
                    <input type="number" min="0" max="10" step="0.1" id="eb-climax-rvol" value={edge.draftConfig.execution.climax_rvol} oninput={(e: Event) => edge.draftConfig.execution.climax_rvol = parseFloat((e.target as HTMLInputElement).value) || 0} class={styles.numInput} />
                </div>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Trigger Phase</span>
                <div class={styles.segmentedGroup}>
                    <button class={styles.segmentBtn} class:active={edge.draftConfig.execution.trigger_phase === 'execute_on_trigger'} onclick={() => edge.draftConfig.execution.trigger_phase = 'execute_on_trigger'}>On Trigger</button>
                    <button class={styles.segmentBtn} class:active={edge.draftConfig.execution.trigger_phase === 'execute_on_confirmed_close'} onclick={() => edge.draftConfig.execution.trigger_phase = 'execute_on_confirmed_close'}>Confirmed Close</button>
                </div>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Backtest Depth</span>
                <select value={edge.draftConfig.backtest_depth} onchange={(e: Event) => edge.draftConfig.backtest_depth = parseInt((e.target as HTMLSelectElement).value)} class={styles.selectFull}>
                    <option value="1000">1,000 candles</option>
                    <option value="5000">5,000 candles</option>
                    <option value="10000">10,000 candles</option>
                    <option value="25000">25,000 candles</option>
                </select>
            </div>
        </div>

        <!-- Column 3: Strategy Meta -->
        <div class={styles.column}>
            <h3>Strategy Meta</h3>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Strategy Name</span>
                <input
                    type="text" class={styles.textInput}
                    placeholder="e.g. GP_Reversal_V2"
                    bind:value={strategyNameInput}
                    oninput={setName}
                />
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Description</span>
                <textarea
                    class={styles.textArea}
                    placeholder="Design notes, assumptions, constraints..."
                    bind:value={strategyDescriptionInput}
                    oninput={setDescription}
                    rows={4}
                ></textarea>
            </div>

            <div class={styles.section}>
                <span class={styles.sectionLabel}>Live JSON Schema</span>
                <pre class={styles.jsonInspector}>{JSON.stringify(edge.draftConfig, null, 2)}</pre>
            </div>
        </div>
    </div>
</div>
