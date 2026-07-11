<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { RiskMatrix, RiskLevel, TrendStability } from '../types';

    let { pairKey }: { pairKey: string } = $props();
    const app = useAppStore();

    const pair = $derived(app.instancesMap[pairKey]);
    const risk = $derived(pair?.risk ?? null);

    function riskColor(level: RiskLevel): string {
        switch (level) {
            case 'VeryLow': return '#22c55e';
            case 'Low': return '#4ade80';
            case 'Moderate': return '#fbbf24';
            case 'High': return '#f97316';
            case 'Extreme': return '#ef4444';
            default: return '#78909c';
        }
    }

    function trendStabilityColor(ts: TrendStability): string {
        switch (ts) {
            case 'Strong': return '#22c55e';
            case 'Healthy': return '#4ade80';
            case 'Developing': return '#fbbf24';
            case 'Weak': return '#f97316';
            case 'Exhausted': return '#ef4444';
            default: return '#78909c';
        }
    }

    function reliabilityColor(sr: string): string {
        switch (sr) {
            case 'Excellent': return '#22c55e';
            case 'Good': return '#4ade80';
            case 'Fair': return '#fbbf24';
            case 'Poor': return '#ef4444';
            default: return '#78909c';
        }
    }

    function formatRiskLabel(r: string): string {
        return r.replace(/([A-Z])/g, ' $1').trim();
    }

    function stopMethodLabel(m: string): string {
        return m.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
    }
</script>

<div class="risk-panel">
    {#if !risk}
        <div class="empty-state">
            <div class="empty-icon">⚡</div>
            <h3>No Risk Data</h3>
            <p>Awaiting completed candles across multiple timeframes. The Risk Matrix requires at least two timeframes with data to assess market risk.</p>
        </div>
    {:else}
        <div class="panel-header">
            <h2>Risk Assessment — {risk.symbol}</h2>
            <div class="header-badges">
                <span class="badge" style:background={riskColor(risk.overall_market_risk)}>
                    {formatRiskLabel(risk.overall_market_risk)}
                </span>
            </div>
        </div>

        <div class="risk-grid">
            <div class="risk-card">
                <div class="card-label">Volatility Risk</div>
                <div class="card-badge" style:background={riskColor(risk.volatility_risk)}>
                    {formatRiskLabel(risk.volatility_risk)}
                </div>
            </div>
            <div class="risk-card">
                <div class="card-label">Liquidity Risk</div>
                <div class="card-badge" style:background={riskColor(risk.liquidity_risk)}>
                    {formatRiskLabel(risk.liquidity_risk)}
                </div>
            </div>
            <div class="risk-card">
                <div class="card-label">Trend Stability</div>
                <div class="card-badge" style:background={trendStabilityColor(risk.trend_stability)}>
                    {risk.trend_stability}
                </div>
            </div>
            <div class="risk-card">
                <div class="card-label">Structural Risk</div>
                <div class="card-badge" style:background={riskColor(risk.structural_risk)}>
                    {formatRiskLabel(risk.structural_risk)}
                </div>
            </div>
            <div class="risk-card">
                <div class="card-label">Signal Reliability</div>
                <div class="card-badge" style:background={reliabilityColor(risk.signal_reliability)}>
                    {risk.signal_reliability}
                </div>
            </div>
        </div>

        <div class="guidance-section">
            <h3>Stop & Target Guidance</h3>
            <div class="guidance-grid">
                <div class="guidance-card">
                    <div class="guidance-label">Stop Method</div>
                    <div class="guidance-value">{stopMethodLabel(risk.suggested_stop_method)}</div>
                </div>
                <div class="guidance-card">
                    <div class="guidance-label">Stop Distance</div>
                    <div class="guidance-value">{risk.suggested_stop_distance.toFixed(1)}x ATR</div>
                </div>
                <div class="guidance-card">
                    <div class="guidance-label">Target Method</div>
                    <div class="guidance-value">{stopMethodLabel(risk.suggested_target_method)}</div>
                </div>
                <div class="guidance-card">
                    <div class="guidance-label">Expected R:R</div>
                    <div class="guidance-value rr-value">{risk.expected_rr.toFixed(1)}</div>
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .risk-panel {
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
    .empty-icon { font-size: 48px; margin-bottom: 16px; opacity: 0.4; }
    .empty-state h3 { font-size: 18px; margin-bottom: 8px; color: #b0b0b0; }
    .empty-state p { font-size: 13px; color: #6b7280; max-width: 400px; margin: 0 auto; line-height: 1.5; }

    .panel-header {
        display: flex; justify-content: space-between; align-items: center;
        margin-bottom: 24px; flex-wrap: wrap; gap: 12px;
    }
    .panel-header h2 { font-size: 16px; font-weight: 600; margin: 0; color: #d4d4d8; }
    .badge {
        padding: 6px 16px; border-radius: 6px; font-size: 13px;
        font-weight: 700; color: #111; white-space: nowrap;
    }

    .risk-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
        gap: 12px;
        margin-bottom: 24px;
    }
    .risk-card {
        background: #1a1a2e; border: 1px solid #2d2d3d;
        border-radius: 8px; padding: 14px; text-align: center;
    }
    .card-label {
        font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px;
        color: #8b8b9b; margin-bottom: 8px;
    }
    .card-badge {
        display: inline-block; padding: 4px 14px; border-radius: 6px;
        font-size: 13px; font-weight: 700; color: #111;
    }

    .guidance-section { margin-bottom: 20px; }
    .guidance-section h3 {
        font-size: 14px; font-weight: 600; color: #d4d4d8; margin-bottom: 12px;
    }
    .guidance-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
        gap: 12px;
    }
    .guidance-card {
        background: #1a1a2e; border: 1px solid #2d2d3d;
        border-radius: 8px; padding: 14px;
    }
    .guidance-label {
        font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px;
        color: #8b8b9b; margin-bottom: 6px;
    }
    .guidance-value {
        font-size: 15px; font-weight: 700; color: #d4d4d8;
    }
    .rr-value { color: #4ade80; }
</style>
