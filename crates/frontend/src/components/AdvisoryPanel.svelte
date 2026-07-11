<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { AdvisoryMatrix, DirectionalGuidance, MarketStance, EntryGuidance, ExitGuidance } from '../types';

    let { pairKey }: { pairKey: string } = $props();
    const app = useAppStore();

    const pair = $derived(app.instancesMap[pairKey]);
    const advisory = $derived(pair?.advisory ?? null);

    function directionColor(d: DirectionalGuidance): string {
        if (d === 'StrongLong' || d === 'Long') return '#26a69a';
        if (d === 'StrongShort' || d === 'Short') return '#ef5350';
        return '#78909c';
    }

    function stanceColor(s: MarketStance): string {
        if (s === 'Aggressive') return '#22c55e';
        if (s === 'Constructive') return '#4ade80';
        if (s === 'Cautious') return '#fbbf24';
        if (s === 'Avoid') return '#ef5350';
        return '#78909c';
    }

    function entryColor(e: EntryGuidance): string {
        if (e === 'Immediate') return '#22c55e';
        if (e === 'WaitForConfirmation') return '#fbbf24';
        if (e === 'NoEntryContext') return '#ef5350';
        return '#78909c';
    }

    function exitColor(e: ExitGuidance): string {
        if (e === 'NoWarning') return '#22c55e';
        if (e === 'MomentumExhaustion') return '#ef5350';
        if (e === 'StructureBreakdown') return '#ef5350';
        return '#fbbf24';
    }

    function formatLabel(s: string): string {
        return s.replace(/([A-Z])/g, ' $1').trim().replace(/_/g, ' ');
    }
</script>

<div class="advisory-panel">
    {#if !advisory}
        <div class="empty-state">
            <div class="empty-icon">📋</div>
            <h3>No Advisory Data</h3>
            <p>Awaiting analysis and risk assessment. The Advisory Matrix requires an Analysis Matrix and Risk Matrix to produce guidance.</p>
        </div>
    {:else}
        <div class="panel-header">
            <h2>Market Advisory — {advisory.symbol}</h2>
            <div class="header-badges">
                <span class="badge" style:background={directionColor(advisory.directional_guidance)}>
                    {formatLabel(advisory.directional_guidance)}
                </span>
            </div>
        </div>

        <div class="guidance-grid">
            <div class="guidance-card">
                <div class="card-label">Directional Guidance</div>
                <div class="card-badge" style:background={directionColor(advisory.directional_guidance)}>
                    {formatLabel(advisory.directional_guidance)}
                </div>
            </div>
            <div class="guidance-card">
                <div class="card-label">Market Stance</div>
                <div class="card-badge" style:background={stanceColor(advisory.market_stance)}>
                    {advisory.market_stance}
                </div>
            </div>
            <div class="guidance-card">
                <div class="card-label">Opportunity</div>
                <div class="card-value">{formatLabel(advisory.opportunity_classification)}</div>
            </div>
            <div class="guidance-card">
                <div class="card-label">Strategy Environment</div>
                <div class="card-value">{formatLabel(advisory.strategy_environment)}</div>
            </div>
        </div>

        <div class="entry-exit-grid">
            <div class="ee-card">
                <div class="ee-label">Entry Guidance</div>
                <div class="ee-badge" style:background={entryColor(advisory.entry_guidance)}>
                    {formatLabel(advisory.entry_guidance)}
                </div>
            </div>
            <div class="ee-card">
                <div class="ee-label">Exit Warning</div>
                <div class="ee-badge" style:background={exitColor(advisory.exit_guidance)}>
                    {formatLabel(advisory.exit_guidance)}
                </div>
            </div>
            <div class="ee-card">
                <div class="ee-label">Protection Strategy</div>
                <div class="ee-value">{formatLabel(advisory.protection_strategy)}</div>
            </div>
            <div class="ee-card">
                <div class="ee-label">Target Strategy</div>
                <div class="ee-value">{formatLabel(advisory.target_strategy)}</div>
            </div>
        </div>

        <div class="confidence-section">
            <div class="conf-label">Confidence Assessment</div>
            <div class="conf-bar-bg">
                <div class="conf-bar-fill" style:width={(advisory.confidence_assessment).toFixed(0) + '%'} style:background={advisory.confidence_assessment > 60 ? '#26a69a' : advisory.confidence_assessment > 30 ? '#fbbf24' : '#ef5350'}></div>
            </div>
            <span class="conf-value">{advisory.confidence_assessment.toFixed(0)}%</span>
        </div>

        <div class="recommendation-section">
            <h3>Final Recommendation</h3>
            <p>{advisory.final_recommendation}</p>
        </div>
    {/if}
</div>

<style>
    .advisory-panel {
        padding: 20px; height: 100%; overflow-y: auto;
        color: #e0e0e0; font-family: 'Inter', system-ui, sans-serif;
    }
    .empty-state { text-align: center; padding: 60px 20px; }
    .empty-icon { font-size: 48px; margin-bottom: 16px; opacity: 0.4; }
    .empty-state h3 { font-size: 18px; margin-bottom: 8px; color: #b0b0b0; }
    .empty-state p { font-size: 13px; color: #6b7280; max-width: 400px; margin: 0 auto; line-height: 1.5; }
    .panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; flex-wrap: wrap; gap: 12px; }
    .panel-header h2 { font-size: 16px; font-weight: 600; margin: 0; color: #d4d4d8; }
    .badge { padding: 6px 16px; border-radius: 6px; font-size: 13px; font-weight: 700; color: #111; }

    .guidance-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 20px; }
    .guidance-card { background: #1a1a2e; border: 1px solid #2d2d3d; border-radius: 8px; padding: 14px; text-align: center; }
    .card-label { font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: #8b8b9b; margin-bottom: 8px; }
    .card-badge { display: inline-block; padding: 4px 14px; border-radius: 6px; font-size: 13px; font-weight: 700; color: #111; }
    .card-value { font-size: 13px; font-weight: 600; color: #d4d4d8; }

    .entry-exit-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 20px; }
    .ee-card { background: #1a1a2e; border: 1px solid #2d2d3d; border-radius: 8px; padding: 14px; text-align: center; }
    .ee-label { font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: #8b8b9b; margin-bottom: 8px; }
    .ee-badge { display: inline-block; padding: 4px 14px; border-radius: 6px; font-size: 13px; font-weight: 700; color: #111; }
    .ee-value { font-size: 13px; font-weight: 600; color: #d4d4d8; }

    .confidence-section { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; background: #1a1a2e; border: 1px solid #2d2d3d; border-radius: 8px; padding: 14px; }
    .conf-label { font-size: 12px; font-weight: 600; color: #d4d4d8; min-width: 160px; }
    .conf-bar-bg { flex: 1; height: 8px; background: #2d2d3d; border-radius: 4px; overflow: hidden; }
    .conf-bar-fill { height: 100%; border-radius: 4px; transition: width 0.5s; }
    .conf-value { font-size: 16px; font-weight: 700; color: #d4d4d8; }

    .recommendation-section { background: #1a1a2e; border: 1px solid #2d2d3d; border-radius: 8px; padding: 16px; }
    .recommendation-section h3 { font-size: 14px; font-weight: 600; color: #d4d4d8; margin: 0 0 8px 0; }
    .recommendation-section p { font-size: 13px; color: #a0a0b0; line-height: 1.6; margin: 0; }
</style>
