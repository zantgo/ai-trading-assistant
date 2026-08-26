// @vitest-environment jsdom
import { describe, expect, it, beforeEach, vi } from 'vitest';
import { buildCandleDebugPayload } from './candleDebug';
import { clearHistoryCache, ingestLiveSnapshot } from './indicatorHistory';
import type { AppStore } from '../state.svelte';

function makeApp(instances: Record<string, { exchange: string; isConnected: boolean; microSec: number; fastSec: number; slowSec: number; macroSec: number }>): AppStore {
    const map: Record<string, any> = {};
    for (const [pairKey, cfg] of Object.entries(instances)) {
        map[pairKey] = {
            symbol: pairKey,
            exchange: cfg.exchange,
            isConnected: cfg.isConnected,
            instanceId: `inst_${pairKey.replace('-', '_')}`,
            microTerm: { slot: 'micro', barDurationSec: cfg.microSec, pipelineState: 'LIVE', latestSnapshot: null, indicators: {}, historyPrices: [] },
            fastTerm: { slot: 'fast', barDurationSec: cfg.fastSec, pipelineState: 'LIVE', latestSnapshot: null, indicators: {}, historyPrices: [] },
            slowTerm: { slot: 'slow', barDurationSec: cfg.slowSec, pipelineState: 'LIVE', latestSnapshot: null, indicators: {}, historyPrices: [] },
            macroTerm: { slot: 'macro', barDurationSec: cfg.macroSec, pipelineState: 'LIVE', latestSnapshot: null, indicators: {}, historyPrices: [] },
        };
    }
    return { instancesMap: map } as unknown as AppStore;
}

function makeSnapshot(ts: number, close: number, opts: { is_completed?: boolean; open?: number; high?: number; low?: number; volume?: number; indicators?: Record<string, unknown> } = {}): Record<string, unknown> {
    return {
        timestamp: ts,
        close,
        open: opts.open ?? close,
        high: opts.high ?? close,
        low: opts.low ?? close,
        volume: opts.volume ?? 10,
        is_completed: opts.is_completed ?? true,
        indicators: opts.indicators ?? {
            rsi: { raw_value: 55.5, normalized: 0.1, state_label: 'Live', values: null },
            ema_stack: { raw_value: close, normalized: 0.2, state_label: 'Live', values: { fast: close - 10, medium: close - 20 } },
        },
        indicator_lifecycle: { rsi: { state: 'Live', bars_seen: 500, bars_required: 14 } },
    };
}

describe('candleDebug', () => {
    beforeEach(() => {
        clearHistoryCache();
        // Reset debug flag
        if (typeof window !== 'undefined') (window as unknown as Record<string, unknown>).__CANDLE_DEBUG_ENABLED__ = undefined;
        try { localStorage.removeItem('candleDebug'); } catch {}
    });

    it('payload contains all instances × 4 slots (background TFs included)', () => {
        const app = makeApp({
            'BTC-USDT': { exchange: 'Hyperliquid', isConnected: true, microSec: 60, fastSec: 180, slowSec: 300, macroSec: 900 },
            'ETH-USDT': { exchange: 'Bitget', isConnected: true, microSec: 60, fastSec: 180, slowSec: 300, macroSec: 900 },
        });
        // Seed 5 completed candles per TF via ingestLiveSnapshot (mutates historyData)
        for (let i = 0; i < 5; i++) {
            const ts = 1_700_000_000 + i * 60;
            const snap = makeSnapshot(ts, 50000 + i * 10);
            ingestLiveSnapshot('BTC-USDT', 60, 'micro', snap);
            ingestLiveSnapshot('BTC-USDT', 180, 'fast', snap);
            ingestLiveSnapshot('BTC-USDT', 300, 'slow', snap);
            ingestLiveSnapshot('BTC-USDT', 900, 'macro', snap);
            ingestLiveSnapshot('ETH-USDT', 60, 'micro', snap);
            ingestLiveSnapshot('ETH-USDT', 180, 'fast', snap);
        }

        const triggerSnap = makeSnapshot(1_700_000_300, 50100);
        const payload = buildCandleDebugPayload(app, { pairKey: 'BTC-USDT', slot: 'micro', timeframe_secs: 60, snapshot: triggerSnap });

        expect(payload.trigger.pairKey).toBe('BTC-USDT');
        expect(payload.trigger.slot).toBe('micro');
        expect(payload.instances.length).toBe(2);
        expect(payload.summary.totalInstances).toBe(2);
        expect(payload.summary.totalTimeframes).toBe(8); // 2 instances × 4
        // Each instance should have 4 timeframes entries
        for (const inst of payload.instances) {
            expect(inst.timeframes.length).toBe(4);
            expect(inst.timeframes.map(t => t.slot)).toEqual(['micro', 'fast', 'slow', 'macro']);
        }
        // BTC micro should have 5 candles seeded
        const btcMicro = payload.instances.find(i => i.pairKey === 'BTC-USDT')!.timeframes.find(t => t.slot === 'micro')!;
        expect(btcMicro.candleCount).toBe(5);
        expect(btcMicro.candles.length).toBe(5);
        expect(btcMicro.candles[0].close).toBe(50000);
        // Indicator overlays should be present for BTC micro
        expect(Object.keys(btcMicro.indicatorOverlays).length).toBeGreaterThan(0);
        expect(btcMicro.indicatorOverlays['rsi']).toBeDefined();
        expect(btcMicro.indicatorOverlays['rsi'].length).toBe(5);
        expect(btcMicro.lastOverlayValues['rsi']).toBe(55.5);
        expect(btcMicro.historyTimes.length).toBe(5);
        expect(btcMicro.alignmentOk).toBe(true);
    });

    it('caps at 1000 — oldest evicted (FIFO)', () => {
        const app = makeApp({
            'BTC-USDT': { exchange: 'Hyperliquid', isConnected: true, microSec: 60, fastSec: 60, slowSec: 60, macroSec: 60 },
        });
        // Push 1100 candles — historyData should trim to 1000
        for (let i = 0; i < 1100; i++) {
            const ts = 1_700_000_000 + i * 60;
            ingestLiveSnapshot('BTC-USDT', 60, 'micro', makeSnapshot(ts, 50000 + i));
        }
        const payload = buildCandleDebugPayload(app, { pairKey: 'BTC-USDT', slot: 'micro', timeframe_secs: 60, snapshot: makeSnapshot(1_700_066_000, 51000) });
        const micro = payload.instances[0].timeframes.find(t => t.slot === 'micro')!;
        expect(micro.candleCount).toBe(1000);
        expect(micro.candles.length).toBe(1000);
        expect(micro.timesCount).toBe(1000);
        expect(micro.bufferLen).toBe(1000);
        // Oldest 100 should have been evicted — first timestamp should be the 101st pushed (i=100)
        expect(micro.candles[0].time).toBe(1_700_000_000 + 100 * 60);
        expect(micro.historyTimes[0]).toBe(1_700_000_000 + 100 * 60);
        expect(payload.summary.cappedAt_1000).toBe(true);
        expect(payload.summary.maxCandlesPerTf).toBe(1000);
    });

    it('includes exchange-aware payload for both Hyperliquid and Bitget', () => {
        const app = makeApp({
            'BTC-USDT': { exchange: 'Hyperliquid', isConnected: true, microSec: 60, fastSec: 180, slowSec: 300, macroSec: 900 },
            'BTC-USDT:bitget': { exchange: 'Bitget', isConnected: true, microSec: 60, fastSec: 180, slowSec: 300, macroSec: 900 },
        });
        ingestLiveSnapshot('BTC-USDT', 60, 'micro', makeSnapshot(1_700_000_000, 50000));
        ingestLiveSnapshot('BTC-USDT:bitget', 60, 'micro', makeSnapshot(1_700_000_000, 50000));

        const payload = buildCandleDebugPayload(app, { pairKey: 'BTC-USDT', slot: 'micro', timeframe_secs: 60, snapshot: makeSnapshot(1_700_000_060, 50010) });
        const exchanges = payload.instances.map(i => i.exchange).sort();
        expect(exchanges).toEqual(['Bitget', 'Hyperliquid']);
    });

    it('summary warmupOk_300 and bootstrapOk_500 for ≥60s TFs', () => {
        const app = makeApp({
            'BTC-USDT': { exchange: 'Hyperliquid', isConnected: true, microSec: 60, fastSec: 180, slowSec: 300, macroSec: 900 },
        });
        // Only 10 candles seeded — below 300 and 500
        for (let i = 0; i < 10; i++) {
            ingestLiveSnapshot('BTC-USDT', 60, 'micro', makeSnapshot(1_700_000_000 + i * 60, 50000 + i));
            ingestLiveSnapshot('BTC-USDT', 180, 'fast', makeSnapshot(1_700_000_000 + i * 180, 50000 + i));
        }
        let payload = buildCandleDebugPayload(app, { pairKey: 'BTC-USDT', slot: 'micro', timeframe_secs: 60, snapshot: makeSnapshot(1_700_000_600, 50010) });
        expect(payload.summary.warmupOk_300).toBe(false);
        expect(payload.summary.bootstrapOk_500).toBe(false);

        // Now seed to 500 for all ≥60s TFs
        clearHistoryCache();
        for (let i = 0; i < 500; i++) {
            const ts = 1_700_000_000 + i * 60;
            ingestLiveSnapshot('BTC-USDT', 60, 'micro', makeSnapshot(ts, 50000 + i));
            ingestLiveSnapshot('BTC-USDT', 180, 'fast', makeSnapshot(ts, 50000 + i));
            ingestLiveSnapshot('BTC-USDT', 300, 'slow', makeSnapshot(ts, 50000 + i));
            ingestLiveSnapshot('BTC-USDT', 900, 'macro', makeSnapshot(ts, 50000 + i));
        }
        payload = buildCandleDebugPayload(app, { pairKey: 'BTC-USDT', slot: 'micro', timeframe_secs: 60, snapshot: makeSnapshot(1_700_030_000, 50500) });
        expect(payload.summary.warmupOk_300).toBe(true);
        expect(payload.summary.bootstrapOk_500).toBe(true);
        expect(payload.summary.cappedAt_1000).toBe(true);
    });
});
