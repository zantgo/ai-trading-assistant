import type { OpenOrder, PlaceOrderPayload, ScaleInPortion, TakeProfitTarget, SlotState, PositionSlot, EquitySnapshot } from '../types';

export class PaperTradingStore {
    paperCashBalance = $state(0);
    paperInitialUSD = $state(0);
    paperAllocationPct = $state(10);
    paperAutoExecute = $state(false);
    activePaperPosition = $state<Record<string, unknown> | null>(null);
    paperUnrealizedPnl = $state(0);
    paperUnrealizedRoi = $state(0);
    paperTotalAccountValue = $state(0);
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
    paperBreakEvenTrailEnabled = $state(false);

    // Percentage-based position tracking
    paperPositionPct = $state(0);
    paperFreeBalancePct = $state(100);
    paperDirection = $state<'LONG' | 'SHORT' | ''>('');
    openOrders = $state<OpenOrder[]>([]);

    // Slot-based state (4-Portion Dynamic Margin)
    activeSlots = $state<SlotState[]>([]);
    positionSlots = $state<PositionSlot[]>([]);
    equitySnapshots = $state<EquitySnapshot[]>([]);
    paperInitialAllocatedMargin = $state(0);
    paperRealizedPnlAccumulator = $state(0);

    // Derived runes for slot button counters
    get activeLongs(): number {
        if (this.paperDirection !== 'LONG') return 0;
        return this.activeSlots.filter(s => s.is_active).length;
    }
    get activeShorts(): number {
        if (this.paperDirection !== 'SHORT') return 0;
        return this.activeSlots.filter(s => s.is_active).length;
    }

    async fetchPaperStatus(pairKey: string) {
        try {
            const res = await fetch(`/api/paper/status?symbol=${encodeURIComponent(pairKey)}`);
            if (!res.ok) return;
            const data = await res.json();
            this.paperCashBalance = data.current_cash ?? 0;
            this.paperInitialUSD = data.initial_usd ?? 0;
            this.paperAllocationPct = data.allocation_pct ?? 10;
            this.paperAutoExecute = data.auto_execute ?? false;
            this.activePaperPosition = data.active_position ?? null;
            this.paperUnrealizedPnl = data.unrealized_pnl ?? 0;
            this.paperUnrealizedRoi = data.unrealized_roi_pct ?? 0;
            this.paperTotalAccountValue = data.total_account_value ?? 0;
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
            this.paperBreakEvenTrailEnabled = data.break_even_trail_enabled ?? false;
            this.paperInitialAllocatedMargin = data.initial_allocated_margin ?? 0;
            this.paperRealizedPnlAccumulator = data.realized_pnl_accumulator ?? 0;

            // Slot data
            if (data.position_slots) {
                this.positionSlots = data.position_slots;
                this.activeSlots = data.position_slots.map((s: PositionSlot) => ({
                    slot_index: s.slot_index,
                    is_active: s.is_active,
                    entry_price: s.entry_price,
                    size: s.size,
                    allocated_usd: s.allocated_usd,
                }));
            }

            // Compute percentage-based state
            const pos = data.active_position;
            const init = data.initial_usd ?? 0;
            if (pos && init > 0) {
                this.paperPositionPct = Math.round((pos.allocated_usd / init) * 100);
                this.paperFreeBalancePct = 100 - this.paperPositionPct;
                this.paperDirection = pos.direction ?? '';
            } else {
                this.paperPositionPct = 0;
                this.paperFreeBalancePct = 100;
                this.paperDirection = '';
            }
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

    /** Open position with percentage of balance. */
    async openPositionPct(pairKey: string, direction: 'LONG' | 'SHORT', pct: number) {
        this.paperLoading = true;
        try {
            const res = await fetch('/api/paper/position', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, direction, pct }),
            });
            const data = await res.json();
            await this.fetchPaperStatus(pairKey);
            return data;
        } catch (_) { return { success: false, message: 'Network error' }; }
        finally { this.paperLoading = false; }
    }

    /** Close percentage of current position. */
    async closePositionPct(pairKey: string, pct: number) {
        this.paperLoading = true;
        try {
            const res = await fetch('/api/paper/close', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, direction: '', pct }),
            });
            const data = await res.json();
            await this.fetchPaperStatus(pairKey);
            return data;
        } catch (_) { return { success: false, message: 'Network error' }; }
        finally { this.paperLoading = false; }
    }

    /** Set take-profit targets. */
    async setTpTargets(pairKey: string, targets: { pct: number; price: number }[]) {
        try {
            const res = await fetch('/api/paper/tp', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, targets }),
            });
            await this.fetchPaperStatus(pairKey);
            return await res.json();
        } catch (_) { return { success: false, message: 'Network error' }; }
    }

    /** Set stop-loss levels. */
    async setSlLevels(pairKey: string, stops: { pct: number; price: number }[]) {
        try {
            const res = await fetch('/api/paper/sl', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, targets: stops }),
            });
            await this.fetchPaperStatus(pairKey);
            return await res.json();
        } catch (_) { return { success: false, message: 'Network error' }; }
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
                    break_even_trail_enabled: this.paperBreakEvenTrailEnabled,
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

    async fetchOpenOrders(pairKey: string) {
        try {
            const res = await fetch(`/api/paper/open-orders?symbol=${encodeURIComponent(pairKey)}`);
            if (res.ok) { this.openOrders = await res.json(); }
        } catch (_) {}
    }

    async fetchSlotStates(pairKey: string) {
        try {
            const res = await fetch(`/api/paper/slot-states?symbol=${encodeURIComponent(pairKey)}`);
            if (res.ok) {
                const data = await res.json();
                this.activeSlots = data.slots || [];
                if (this.activeSlots.length > 0) {
                    const active = this.activeSlots.filter((s: SlotState) => s.is_active);
                    this.paperDirection = active.length > 0 ? (active[0].entry_price > 0 ? this.paperDirection || 'LONG' : this.paperDirection) : '';
                } else {
                    this.paperDirection = '';
                }
            }
        } catch (_) {}
    }

    async fetchEquityHistory(pairKey: string) {
        try {
            const res = await fetch(`/api/paper/equity-history?symbol=${encodeURIComponent(pairKey)}&limit=200`);
            if (res.ok) {
                const data = await res.json();
                this.equitySnapshots = data.snapshots || [];
            }
        } catch (_) {}
    }

    async openSlot(pairKey: string, direction: 'LONG' | 'SHORT') {
        this.paperLoading = true;
        try {
            const res = await fetch('/api/paper/portion/open', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, direction }),
            });
            const data = await res.json();
            await this.fetchPaperStatus(pairKey);
            await this.fetchSlotStates(pairKey);
            return data;
        } catch (_) { return { success: false, message: 'Network error' }; }
        finally { this.paperLoading = false; }
    }

    async closeSlot(pairKey: string) {
        this.paperLoading = true;
        try {
            const res = await fetch('/api/paper/portion/close', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey }),
            });
            const data = await res.json();
            await this.fetchPaperStatus(pairKey);
            await this.fetchSlotStates(pairKey);
            return data;
        } catch (_) { return { success: false, message: 'Network error' }; }
        finally { this.paperLoading = false; }
    }

    async placeOrder(pairKey: string, order: PlaceOrderPayload) {
        try {
            const res = await fetch('/api/paper/order/place', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, ...order }),
            });
            const data = await res.json();
            await this.fetchOpenOrders(pairKey);
            return data;
        } catch (_) { return { success: false, message: 'Network error' }; }
    }

    async cancelOrder(pairKey: string, orderId: number) {
        try {
            const res = await fetch('/api/paper/order/cancel', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ symbol: pairKey, order_id: orderId }),
            });
            const data = await res.json();
            await this.fetchOpenOrders(pairKey);
            return data;
        } catch (_) { return { success: false, message: 'Network error' }; }
    }
}

export function calcLiqPrice(entryPrice: number, direction: 'LONG' | 'SHORT', leverage: number): number {
    if (entryPrice <= 0 || leverage <= 0) return 0;
    const invLeverage = 1 / leverage;
    return direction === 'LONG'
        ? entryPrice * (1 - invLeverage)
        : entryPrice * (1 + invLeverage);
}

export function calcSizeUnits(payAmountUsd: number, leverage: number, markPrice: number): number {
    if (markPrice <= 0 || leverage <= 0) return 0;
    return (payAmountUsd * leverage) / markPrice;
}

export function calcEstFees(sizeUnits: number, markPrice: number, takerFeePct: number = 0.04): number {
    return sizeUnits * markPrice * (takerFeePct / 100);
}
