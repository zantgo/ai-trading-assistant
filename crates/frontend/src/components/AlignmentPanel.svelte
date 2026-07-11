<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { AlignmentMatrix, TfAlignment } from '../types';

    let { pairKey }: { pairKey: string } = $props();
    const app = useAppStore();

    const pair = $derived(app.instancesMap[pairKey]);
    const alignment = $derived(pair?.alignment ?? null);

    function biasColor(score: number): string {
        if (score > 20) return '#26a69a';
        if (score < -20) return '#ef5350';
        return '#78909c';
    }

    function agreementColor(pct: number): string {
        if (pct >= 75) return '#26a69a';
        if (pct >= 50) return '#ffa726';
        return '#ef5350';
    }

    function regimeColor(regime: string): string {
        switch (regime) {
            case 'TRENDING': return '#26a69a';
            case 'EXPANSION': return '#42a5f5';
            case 'COMPRESSION': return '#ffa726';
            case 'RANGE': return '#78909c';
            default: return '#78909c';
        }
    }

    function gaugeWidth(score: number): string {
        return `${((score + 1) / 2 * 100).toFixed(1)}%`;
    }
</script>

<div class="alignment-panel">
    {#if !alignment || alignment.timeframes_present === 0}
        <div class="empty-state">
            <div class="empty-icon">⟷</div>
            <h3>No Confluence Data</h3>
            <p>Awaiting completed candles across multiple timeframes. The Confluence Matrix requires at least two timeframes with data to compute multi-timeframe alignment.</p>
        </div>
    {:else}
        <!-- Header -->
        <div class="panel-header">
            <h2>Timeframe Alignment — {alignment.symbol}</h2>
            <div class="header-badges">
                <span class="badge" style:background={biasColor(alignment.mtf_overall_score)}>
                    {alignment.mtf_overall_label.replace(/_/g, ' ')}
                </span>
                <span class="badge badge-count">
                    {alignment.timeframes_present}/4 TFs active
                </span>
            </div>
        </div>

        <!-- MTF Overall Score -->
        <div class="score-section">
            <div class="score-label">MTF Overall Score</div>
            <div class="score-bar-container">
                <div class="score-bar-bg">
                    <div class="score-bar-fill" style:width={gaugeWidth(alignment.mtf_overall_score / 100)} style:background={biasColor(alignment.mtf_overall_score)}></div>
                </div>
                <span class="score-value" style:color={biasColor(alignment.mtf_overall_score)}>
                    {alignment.mtf_overall_score.toFixed(0)}
                </span>
            </div>
        </div>

        <!-- MTF Alignment per Group -->
        <div class="alignment-grid">
            <div class="alignment-card">
                <div class="card-label">Trend</div>
                <div class="card-bar-bg">
                    <div class="card-bar-fill" style:width={gaugeWidth(alignment.mtf_trend_alignment)} style:background={biasColor(alignment.mtf_trend_alignment * 100)}></div>
                </div>
                <div class="card-value" style:color={biasColor(alignment.mtf_trend_alignment * 100)}>
                    {(alignment.mtf_trend_alignment * 100).toFixed(0)}
                </div>
            </div>
            <div class="alignment-card">
                <div class="card-label">Momentum</div>
                <div class="card-bar-bg">
                    <div class="card-bar-fill" style:width={gaugeWidth(alignment.mtf_momentum_alignment)} style:background={biasColor(alignment.mtf_momentum_alignment * 100)}></div>
                </div>
                <div class="card-value" style:color={biasColor(alignment.mtf_momentum_alignment * 100)}>
                    {(alignment.mtf_momentum_alignment * 100).toFixed(0)}
                </div>
            </div>
            <div class="alignment-card">
                <div class="card-label">Volume</div>
                <div class="card-bar-bg">
                    <div class="card-bar-fill" style:width={gaugeWidth(alignment.mtf_volume_alignment)} style:background={biasColor(alignment.mtf_volume_alignment * 100)}></div>
                </div>
                <div class="card-value" style:color={biasColor(alignment.mtf_volume_alignment * 100)}>
                    {(alignment.mtf_volume_alignment * 100).toFixed(0)}
                </div>
            </div>
            <div class="alignment-card">
                <div class="card-label">Volatility</div>
                <div class="card-bar-bg">
                    <div class="card-bar-fill" style:width={gaugeWidth(alignment.mtf_volatility_alignment)} style:background={biasColor(alignment.mtf_volatility_alignment * 100)}></div>
                </div>
                <div class="card-value" style:color={biasColor(alignment.mtf_volatility_alignment * 100)}>
                    {(alignment.mtf_volatility_alignment * 100).toFixed(0)}
                </div>
            </div>
        </div>

        <!-- Trend Agreement -->
        <div class="agreement-section">
            <div class="agreement-label">
                Trend Agreement
                <span class="agreement-pct" style:color={agreementColor(alignment.trend_agreement_pct)}>
                    {alignment.trend_agreement_pct.toFixed(0)}%
                </span>
            </div>
            <div class="agreement-desc">
                {#if alignment.trend_agreement_pct >= 75}
                    Strong multi-timeframe directional alignment
                {:else if alignment.trend_agreement_pct >= 50}
                    Moderate agreement — some timeframe divergence
                {:else}
                    Weak agreement — timeframes are conflicting
                {/if}
            </div>
        </div>

        <!-- Cross-TF Signals -->
        {#if alignment.signal_cross_tf_count > 0}
        <div class="cross-signals">
            <span class="signal-badge">⚡ {alignment.signal_cross_tf_count} signals across ≥2 timeframes</span>
        </div>
        {/if}

        <!-- Per-Timeframe Breakdown Table -->
        <div class="breakdown-section">
            <h3>Timeframe Breakdown</h3>
            <table class="breakdown-table">
                <thead>
                    <tr>
                        <th>Timeframe</th>
                        <th>Trend</th>
                        <th>Momentum</th>
                        <th>Overall</th>
                        <th>Regime</th>
                        <th>Signals</th>
                    </tr>
                </thead>
                <tbody>
                    {#each alignment.timeframe_alignments as tf (tf.timeframe)}
                        <tr>
                            <td class="tf-name">{tf.timeframe}<br/><span class="tf-price">${tf.price.toFixed(0)}</span></td>
                            <td style:color={biasColor(tf.trend_score * 100)}>{(tf.trend_score * 100).toFixed(0)}</td>
                            <td style:color={biasColor(tf.momentum_score * 100)}>{(tf.momentum_score * 100).toFixed(0)}</td>
                            <td><span class="badge" style:background={biasColor(tf.overall_score)}>{tf.overall_score}</span></td>
                            <td><span class="badge" style:background={regimeColor(tf.regime)}>{tf.regime}</span></td>
                            <td>{tf.active_signals}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>

<style>
    .alignment-panel {
        padding: 20px;
        height: 100%;
        overflow-y: auto;
        color: #e0e0e0;
        font-family: 'Inter', system-ui, sans-serif;
    }

    .empty-state {
        text-align: center;
        padding: 60px 20px;
    }
    .empty-icon {
        font-size: 48px;
        margin-bottom: 16px;
        opacity: 0.4;
    }
    .empty-state h3 {
        font-size: 18px;
        margin-bottom: 8px;
        color: #b0b0b0;
    }
    .empty-state p {
        font-size: 13px;
        color: #6b7280;
        max-width: 400px;
        margin: 0 auto;
        line-height: 1.5;
    }

    .panel-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 24px;
        flex-wrap: wrap;
        gap: 12px;
    }
    .panel-header h2 {
        font-size: 16px;
        font-weight: 600;
        margin: 0;
        color: #d4d4d8;
    }
    .header-badges {
        display: flex;
        gap: 8px;
    }

    .badge {
        padding: 4px 10px;
        border-radius: 6px;
        font-size: 11px;
        font-weight: 600;
        color: #111;
        white-space: nowrap;
    }
    .badge-count {
        background: #2d2d3d;
        color: #a0a0b0;
    }

    .score-section {
        margin-bottom: 24px;
    }
    .score-label {
        font-size: 12px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: #8b8b9b;
        margin-bottom: 8px;
    }
    .score-bar-container {
        display: flex;
        align-items: center;
        gap: 12px;
    }
    .score-bar-bg {
        flex: 1;
        height: 8px;
        background: #2d2d3d;
        border-radius: 4px;
        overflow: hidden;
    }
    .score-bar-fill {
        height: 100%;
        border-radius: 4px;
        transition: width 0.5s ease;
    }
    .score-value {
        font-size: 20px;
        font-weight: 700;
        min-width: 48px;
        text-align: right;
    }

    .alignment-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 12px;
        margin-bottom: 24px;
    }
    .alignment-card {
        background: #1a1a2e;
        border: 1px solid #2d2d3d;
        border-radius: 8px;
        padding: 14px;
    }
    .card-label {
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: #8b8b9b;
        margin-bottom: 8px;
    }
    .card-bar-bg {
        height: 6px;
        background: #2d2d3d;
        border-radius: 3px;
        overflow: hidden;
        margin-bottom: 6px;
    }
    .card-bar-fill {
        height: 100%;
        border-radius: 3px;
        transition: width 0.5s ease;
    }
    .card-value {
        font-size: 14px;
        font-weight: 700;
    }

    .agreement-section {
        background: #1a1a2e;
        border: 1px solid #2d2d3d;
        border-radius: 8px;
        padding: 14px;
        margin-bottom: 20px;
    }
    .agreement-label {
        font-size: 13px;
        font-weight: 600;
        color: #d4d4d8;
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 4px;
    }
    .agreement-pct {
        font-size: 18px;
        font-weight: 700;
    }
    .agreement-desc {
        font-size: 12px;
        color: #6b7280;
    }

    .cross-signals {
        margin-bottom: 20px;
    }
    .signal-badge {
        background: #1a1a2e;
        border: 1px solid #ffa726;
        color: #ffa726;
        padding: 6px 14px;
        border-radius: 8px;
        font-size: 12px;
        font-weight: 600;
    }

    .breakdown-section h3 {
        font-size: 14px;
        font-weight: 600;
        color: #d4d4d8;
        margin-bottom: 12px;
    }
    .breakdown-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 12px;
    }
    .breakdown-table th {
        text-align: left;
        padding: 8px 10px;
        color: #8b8b9b;
        font-weight: 600;
        text-transform: uppercase;
        font-size: 10px;
        letter-spacing: 0.5px;
        border-bottom: 1px solid #2d2d3d;
    }
    .breakdown-table td {
        padding: 10px;
        border-bottom: 1px solid #1a1a2e;
        color: #c0c0c8;
    }
    .tf-name {
        font-weight: 600;
    }
    .tf-price {
        font-size: 10px;
        color: #6b7280;
    }
</style>
