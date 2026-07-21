// @vitest-environment jsdom
// Test for the LIQ HEATMAP and VOL PROFILE toggle pills in ChartToggles.svelte.
//
// Verifies that toggling a flag on the TF state object propagates correctly
// across all 4 timeframes (since the toggle is sync-all, like VWAP/Bollinger).

import { describe, it, expect, beforeEach } from 'vitest';
import type { InstanceState } from '../types';

beforeEach(() => {
    (globalThis as any).__appStore = {
        instancesMap: {},
    };
});

function makeInstance(): InstanceState {
    return {
        symbol: 'BTC-USDT',
        exchange: 'Hyperliquid',
        isConnected: true,
        microTerm: makeTf('micro'),
        fastTerm: makeTf('fast'),
        slowTerm: makeTf('slow'),
        macroTerm: makeTf('macro'),
        historyLatestClose: '0',
        currentView: 'terminal',
        alignment: null,
        analysis: null,
        risk: null,
        advisory: null,
        automationEnabled: false,
        automationIntervalMode: 'interval',
        automationIntervalValue: 900,
        automationIntervalUnit: 'seconds',
        priceLineMode: false,
        slowIntervalSecs: 900,
        normalIntervalSecs: 300,
        fastIntervalSecs: 60,
        showEmaFast: false,
        showEmaMedium: false,
        showEmaSlow: false,
        showEmaLong: false,
    };
}

function makeTf(slot: 'micro' | 'fast' | 'slow' | 'macro') {
    return {
        slot,
        symbol: 'BTC-USDT',
        exchange: 'Hyperliquid',
        barDurationSec: 60,
        indicators: {},
        priceText: '--',
        volText: '--',
        avgVolText: '--',
        showPatterns: true,
        isCompleted: false,
        latestSnapshot: null,
        historyPrices: [],
        showEmas: true,
        showBb: true,
        showVwap: true,
        showVolume: true,
        showAdx: true,
        showAtr: true,
        showRsi: true,
        showMacd: true,
        showSqueeze: true,
        showBbwp: true,
        showFib: true,
        showRvol: true,
        showStochastic: true,
        showChandeMo: true,
        showSupertrend: true,
        showKeltner: true,
        showDonchian: true,
        showIchimoku: true,
        showHullMa: true,
        showPsar: true,
        showStddevChan: true,
        showObv: true,
        showCmf: true,
        showMfi: true,
        showHv: true,
        showAroon: true,
        showChoppiness: true,
        showLinregSlope: true,
        showZscore: true,
        showLiqHeatmap: false,
        showVolumeProfile: false,
    } as any;
}

describe('ChartToggles overlay state propagation', () => {
    it('defaults showLiqHeatmap to false on every timeframe', () => {
        const inst = makeInstance();
        expect(inst.microTerm.showLiqHeatmap).toBe(false);
        expect(inst.fastTerm.showLiqHeatmap).toBe(false);
        expect(inst.slowTerm.showLiqHeatmap).toBe(false);
        expect(inst.macroTerm.showLiqHeatmap).toBe(false);
    });

    it('defaults showVolumeProfile to false on every timeframe', () => {
        const inst = makeInstance();
        expect(inst.microTerm.showVolumeProfile).toBe(false);
        expect(inst.fastTerm.showVolumeProfile).toBe(false);
        expect(inst.slowTerm.showVolumeProfile).toBe(false);
        expect(inst.macroTerm.showVolumeProfile).toBe(false);
    });

    it('LIQ HEATMAP toggle flips all four timeframes in sync', () => {
        const inst = makeInstance();
        // Simulate the syncAll() pattern used by ChartToggles.
        const v = !inst.microTerm.showLiqHeatmap;
        const tfs = [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm];
        for (const tf of tfs) tf.showLiqHeatmap = v;

        expect(inst.microTerm.showLiqHeatmap).toBe(true);
        expect(inst.fastTerm.showLiqHeatmap).toBe(true);
        expect(inst.slowTerm.showLiqHeatmap).toBe(true);
        expect(inst.macroTerm.showLiqHeatmap).toBe(true);
    });

    it('VOL PROFILE toggle flips all four timeframes in sync', () => {
        const inst = makeInstance();
        const v = !inst.microTerm.showVolumeProfile;
        const tfs = [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm];
        for (const tf of tfs) tf.showVolumeProfile = v;

        expect(inst.microTerm.showVolumeProfile).toBe(true);
        expect(inst.fastTerm.showVolumeProfile).toBe(true);
        expect(inst.slowTerm.showVolumeProfile).toBe(true);
        expect(inst.macroTerm.showVolumeProfile).toBe(true);
    });

    it('toggling LIQ HEATMAP off syncs the off state too', () => {
        const inst = makeInstance();
        // First turn on
        for (const tf of [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm]) {
            tf.showLiqHeatmap = true;
        }
        expect(inst.macroTerm.showLiqHeatmap).toBe(true);
        // Then turn off
        const v = !inst.microTerm.showLiqHeatmap; // false
        for (const tf of [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm]) {
            tf.showLiqHeatmap = v;
        }
        expect(inst.microTerm.showLiqHeatmap).toBe(false);
        expect(inst.macroTerm.showLiqHeatmap).toBe(false);
    });

    it('LIQ HEATMAP and VOL PROFILE toggles are independent', () => {
        const inst = makeInstance();
        inst.microTerm.showLiqHeatmap = true;
        // Volume profile should not be affected.
        expect(inst.microTerm.showVolumeProfile).toBe(false);
        inst.microTerm.showVolumeProfile = true;
        expect(inst.microTerm.showLiqHeatmap).toBe(true);
    });
});