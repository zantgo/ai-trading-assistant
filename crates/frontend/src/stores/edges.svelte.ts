import type {
    SavedEdge, EdgeConfig, EdgeAnalysisResponse, EdgeSaveRequest, EdgeAnalyzeRequest,
    EdgeArchetype, RegimeGates, SizingConfig, StopLossConfig, TakeProfitConfig,
    ExecutionConfig, IndicatorConfig, TriggerRule, SizingModel, StopLossModel, TriggerPhase,
} from '../types';
import { fetchEdgesCall, saveEdgeCall, analyzeEdgeCall, deleteEdgeCall } from '../lib/api.svelte';

export function createDefaultEdgeConfig(): EdgeConfig {
    return {
        archetype: 'trend_following',
        regime_gates: { trending: true, compression: false, expansion: false, range: false },
        quorum_threshold: 60,
        mtf_quorum: ['micro', 'fast'],
        indicators: [
            { name: 'rsi', weight: 10, trigger_rule: 'overbought_oversold', enabled: true },
            { name: 'macd', weight: 20, trigger_rule: 'crossover', enabled: true },
            { name: 'adx', weight: 15, trigger_rule: 'threshold_above', enabled: true },
            { name: 'ema', weight: 10, trigger_rule: 'slope_direction', enabled: true },
            { name: 'bbwp', weight: 10, trigger_rule: 'threshold_below', enabled: false },
            { name: 'squeeze', weight: 10, trigger_rule: 'release', enabled: false },
            { name: 'atr', weight: 5, trigger_rule: 'slope_direction', enabled: false },
            { name: 'vwap', weight: 5, trigger_rule: 'threshold_above', enabled: false },
            { name: 'rvol', weight: 5, trigger_rule: 'threshold_above', enabled: false },
        ],
        sizing: { model: 'fixed', daily_vol_target_pct: 2.0, max_leverage: 20 },
        stop_loss: { model: 'atr_volatility_stop', atr_multiplier: 2.0 },
        take_profit: { tp1_multiplier: 2.5, tp2_multiplier: 5.0, tp3_multiplier: 8.0 },
        execution: { min_rvol: 1.5, climax_rvol: 3.0, trigger_phase: 'execute_on_confirmed_close', vwap_filter: false },
        backtest_depth: 10000,
    };
}

export const AVAILABLE_INDICATORS: { name: string; label: string; defaultTrigger: TriggerRule }[] = [
    { name: 'rsi', label: 'RSI (Relative Strength)', defaultTrigger: 'overbought_oversold' },
    { name: 'macd', label: 'MACD', defaultTrigger: 'crossover' },
    { name: 'adx', label: 'ADX (Trend Strength)', defaultTrigger: 'threshold_above' },
    { name: 'ema', label: 'EMA Stack', defaultTrigger: 'slope_direction' },
    { name: 'bbwp', label: 'BBWP (Volatility)', defaultTrigger: 'threshold_below' },
    { name: 'squeeze', label: 'Squeeze Momentum', defaultTrigger: 'release' },
    { name: 'atr', label: 'ATR (Volatility)', defaultTrigger: 'slope_direction' },
    { name: 'vwap', label: 'VWAP', defaultTrigger: 'threshold_above' },
    { name: 'rvol', label: 'Relative Volume', defaultTrigger: 'threshold_above' },
];

export class EdgeStore {
    savedEdges: SavedEdge[] = $state([]);
    activeEdgeId: number | null = $state(null);
    draftConfig: EdgeConfig = $state(createDefaultEdgeConfig());
    draftName: string = $state('');
    draftDescription: string = $state('');
    simulationResults: EdgeAnalysisResponse | null = $state(null);
    isSimulating: boolean = $state(false);
    error: string | null = $state(null);
    saveStatus: string | null = $state(null);

    resetDraft() {
        this.draftConfig = createDefaultEdgeConfig();
        this.draftName = '';
        this.draftDescription = '';
        this.saveStatus = null;
        this.error = null;
    }

    loadConfig(edge: SavedEdge) {
        this.draftConfig = { ...createDefaultEdgeConfig(), ...edge.config };
        this.draftName = edge.name;
        this.draftDescription = edge.description || '';
        this.activeEdgeId = edge.id;
    }

    async fetchEdges(pairKey: string) {
        try {
            this.savedEdges = await fetchEdgesCall(pairKey);
        } catch (e: any) {
            this.error = `Failed to fetch edges: ${e.message}`;
        }
    }

    async saveEdge(pairKey: string, creatorName?: string): Promise<boolean> {
        if (!this.draftName.trim()) {
            this.error = 'Edge name is required';
            return false;
        }
        this.saveStatus = 'saving';
        try {
            const payload: EdgeSaveRequest = {
                name: this.draftName.trim(),
                pair_key: pairKey,
                description: this.draftDescription,
                config: this.draftConfig,
                creator_name: creatorName || undefined,
            };
            const result = await saveEdgeCall(payload);
            if (result.success) {
                this.activeEdgeId = result.id || null;
                this.saveStatus = 'saved';
                await this.fetchEdges(pairKey);
                return true;
            } else {
                this.error = result.error || 'Failed to save edge';
                this.saveStatus = null;
                return false;
            }
        } catch (e: any) {
            this.error = `Failed to save edge: ${e.message}`;
            this.saveStatus = null;
            return false;
        }
    }

    async deleteEdge(id: number, pairKey: string) {
        try {
            const ok = await deleteEdgeCall(id);
            if (ok) {
                if (this.activeEdgeId === id) {
                    this.activeEdgeId = null;
                    this.resetDraft();
                }
                await this.fetchEdges(pairKey);
            }
        } catch (e: any) {
            this.error = `Failed to delete edge: ${e.message}`;
        }
    }

    async runAnalysis(symbol: string, timeframeSecs: number) {
        if (!this.activeEdgeId) {
            this.error = 'No edge selected. Please save the edge first.';
            return;
        }
        this.isSimulating = true;
        this.error = null;
        try {
            const payload: EdgeAnalyzeRequest = {
                edge_id: this.activeEdgeId,
                symbol,
                timeframe_secs: timeframeSecs,
            };
            this.simulationResults = await analyzeEdgeCall(payload);
        } catch (e: any) {
            this.error = `Analysis failed: ${e.message}`;
        } finally {
            this.isSimulating = false;
        }
    }

    exportConfig() {
        const json = JSON.stringify(this.draftConfig, null, 2);
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${this.draftName || 'edge_config'}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
}

let instance: EdgeStore | null = null;

export function useEdgeStore(): EdgeStore {
    if (!instance) {
        instance = new EdgeStore();
    }
    return instance;
}
