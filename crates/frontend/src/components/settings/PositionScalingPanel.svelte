<script lang="ts">
    import type { PositionScalingConfig, AllocationCurveModel } from '../../types';
    import styles from './settings.module.css';

    let {
        onchange,
        initial,
    }: { onchange?: (c: PositionScalingConfig) => void; initial?: PositionScalingConfig | null } = $props();

    let sizingModel = $state<AllocationCurveModel>(initial?.allocation_curve?.model ?? 'Stepped');
    let basePct = $state(initial?.allocation_curve?.base_allocation_pct ?? 1.0);
    let maxPct = $state(initial?.allocation_curve?.max_allocation_pct ?? 3.0);
    let leverageMode = $state<'Fixed' | 'VolatilityScaled'>(initial?.leverage_mode ?? 'Fixed');
    let leverageCap = $state(initial?.leverage_cap ?? 20);
    let targetMargin = $state(initial?.target_margin ?? 0.02);
    let exponent = $state(initial?.allocation_curve?.exponent ?? 2.0);

    function emit() {
        onchange?.({
            allocation_curve: {
                model: sizingModel,
                base_allocation_pct: basePct,
                max_allocation_pct: maxPct,
                base_score_threshold: 40,
                micro_score_threshold: 60,
                ...(sizingModel === 'Exponential' ? { exponent } : {}),
            },
            leverage_mode: leverageMode,
            leverage_cap: leverageCap,
            target_margin: targetMargin,
        });
    }
</script>

<div class={styles.panel}>
    <h4 class={styles.panelTitle}>Position Sizing &amp; Risk</h4>
    <p class={styles.panelDesc}>Configure how confluence scores map to capital allocation and leverage.</p>

    <div class={styles.fieldGroup}>
        <label class={styles.fieldLabel}>Sizing Model</label>
        <select class={styles.select} value={sizingModel} onchange={(e) => { sizingModel = e.currentTarget.value as AllocationCurveModel; emit(); }}>
            <option value="Stepped">Stepped (Thresholds)</option>
            <option value="Linear">Linear Interpolation</option>
            <option value="Exponential">Exponential (Front-loaded)</option>
        </select>
    </div>

    <div class={styles.fieldRow}>
        <div class={styles.fieldGroup}>
            <label class={styles.fieldLabel}>Base Allocation %</label>
            <input type="number" min="0.1" max="100" step="0.1" value={basePct} oninput={(e) => { basePct = parseFloat(e.currentTarget.value) || 1; emit(); }} class={styles.input} />
        </div>
        <div class={styles.fieldGroup}>
            <label class={styles.fieldLabel}>Max Allocation %</label>
            <input type="number" min="0.1" max="100" step="0.1" value={maxPct} oninput={(e) => { maxPct = parseFloat(e.currentTarget.value) || 3; emit(); }} class={styles.input} />
        </div>
    </div>

    {#if sizingModel === 'Exponential'}
        <div class={styles.fieldGroup}>
            <label class={styles.fieldLabel}>Exponent ({exponent})</label>
            <input type="range" min="1" max="5" step="0.5" value={exponent} oninput={(e) => { exponent = parseFloat(e.currentTarget.value); emit(); }} class={styles.weightSlider} />
        </div>
    {/if}

    <div class={styles.fieldGroup}>
        <label class={styles.fieldLabel}>Leverage Mode</label>
        <select class={styles.select} value={leverageMode} onchange={(e) => { leverageMode = e.currentTarget.value as 'Fixed' | 'VolatilityScaled'; emit(); }}>
            <option value="Fixed">Fixed</option>
            <option value="VolatilityScaled">Volatility-Scaled (ATR)</option>
        </select>
    </div>

    <div class={styles.fieldRow}>
        <div class={styles.fieldGroup}>
            <label class={styles.fieldLabel}>Leverage Cap</label>
            <input type="number" min="1" max="100" step="1" value={leverageCap} oninput={(e) => { leverageCap = parseInt(e.currentTarget.value) || 20; emit(); }} class={styles.input} />
        </div>
        {#if leverageMode === 'VolatilityScaled'}
            <div class={styles.fieldGroup}>
                <label class={styles.fieldLabel}>Target Margin</label>
                <input type="number" min="0.001" max="0.5" step="0.001" value={targetMargin} oninput={(e) => { targetMargin = parseFloat(e.currentTarget.value) || 0.02; emit(); }} class={styles.input} />
            </div>
        {/if}
    </div>
</div>
