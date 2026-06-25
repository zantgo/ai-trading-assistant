import type { ScaleInPortion, TakeProfitTarget } from '../types';

export class PaperTradingStore {
    paperCashBalance = $state(10000);
    paperInitialUSD = $state(10000);
    paperAllocationPct = $state(10);
    paperAutoExecute = $state(false);
    activePaperPosition = $state<Record<string, unknown> | null>(null);
    paperUnrealizedPnl = $state(0);
    paperUnrealizedRoi = $state(0);
    paperTotalAccountValue = $state(10000);
    paperMarginUsed = $state(0);
    paperMaxTrades = $state(10);
    paperActiveTrades = $state(0);
    paperAvailableTrades = $state(10);
    paperHistory = $state<Record<string, unknown>[]>([]);
    paperLoading = $state(false);
    paperScaleInPortions = $state<ScaleInPortion[]>([]);
    paperTakeProfitTargets = $state<TakeProfitTarget[]>([]);
    paperAvgEntryPrice = $state(0);
    paperInvalidationLevel = $state(0);
    paperFilledPortions = $state(0);
    paperMaxRiskPct = $state(2.0);
    paperLeverage = $state(20);
    paperAutoExecuteIntervals = $state(15);
    paperLookbackTrades = $state(10);

    async fetchPaperStatus(pairKey: string) {
        try {
            const res = await fetch(`/api/paper/status?symbol=${encodeURIComponent(pairKey)}`);
            if (!res.ok) return;
            const data = await res.json();
            this.paperCashBalance = data.current_cash ?? 10000;
            this.paperInitialUSD = data.initial_usd ?? 10000;
            this.paperAllocationPct = data.allocation_pct ?? 10;
            this.paperAutoExecute = data.auto_execute ?? false;
            this.activePaperPosition = data.active_position ?? null;
            this.paperUnrealizedPnl = data.unrealized_pnl ?? 0;
            this.paperUnrealizedRoi = data.unrealized_roi_pct ?? 0;
            this.paperTotalAccountValue = data.total_account_value ?? 10000;
            this.paperMarginUsed = data.margin_used ?? 0;
            this.paperMaxTrades = data.max_trades ?? 10;
            this.paperActiveTrades = data.active_trades ?? 0;
            this.paperAvailableTrades = data.available_trades ?? 10;
            this.paperScaleInPortions = data.scale_in_portions ?? [];
            this.paperTakeProfitTargets = data.take_profit_targets ?? [];
            this.paperAvgEntryPrice = data.active_position?.average_entry_price ?? data.active_position?.entry_price ?? 0;
            this.paperInvalidationLevel = data.active_position?.final_invalidation_level ?? 0;
            this.paperFilledPortions = data.active_position?.current_portions ?? 0;
            this.paperMaxRiskPct = data.max_risk_pct ?? 2.0;
            this.paperLeverage = data.leverage ?? 20;
            this.paperAutoExecuteIntervals = data.auto_execute_intervals ?? 15;
            this.paperLookbackTrades = data.lookback_trades ?? 10;
        } catch (_) {}
    }

    async openPaperPosition(pairKey: string, direction: 'LONG' | 'SHORT') {
        this.paperLoading = true;
        try {
            const res = await fetch('/api/paper/order', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, direction, action: 'OPEN' }),
            });
            if (res.ok) await this.fetchPaperStatus(pairKey);
        } catch (_) {} finally { this.paperLoading = false; }
    }

    async closePaperPosition(pairKey: string) {
        this.paperLoading = true;
        try {
            const res = await fetch('/api/paper/order', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, direction: '', action: 'CLOSE' }),
            });
            if (res.ok) await this.fetchPaperStatus(pairKey);
        } catch (_) {} finally { this.paperLoading = false; }
    }

    async resetPaperAccount(pairKey: string) {
        try {
            await fetch('/api/paper/reset', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ symbol: pairKey }) });
            await this.fetchPaperStatus(pairKey);
        } catch (_) {}
    }

    async savePaperConfig(pairKey: string, initialUSD: number, allocationPct: number, autoExecute: boolean) {
        try {
            await fetch('/api/paper/config', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    symbol: pairKey, initial_usd: initialUSD, allocation_pct: allocationPct,
                    auto_execute: autoExecute, max_risk_pct: this.paperMaxRiskPct,
                    leverage: this.paperLeverage, auto_execute_intervals: this.paperAutoExecuteIntervals,
                    lookback_trades: this.paperLookbackTrades,
                })
            });
            await this.fetchPaperStatus(pairKey);
        } catch (_) {}
    }

    async fetchPaperHistory(pairKey: string, symbol?: string) {
        try {
            const url = symbol ? `/api/paper/performance?symbol=${encodeURIComponent(symbol)}` : '/api/paper/performance';
            const res = await fetch(url);
            if (res.ok) { this.paperHistory = (await res.json()).trades || []; }
        } catch (_) {}
    }
}
