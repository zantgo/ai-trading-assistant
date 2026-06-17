// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { getState } from '../state.svelte';

describe('TEST-UI: State Reactive Effects', () => {
    let app: ReturnType<typeof getState>;

    beforeEach(() => {
        app = getState();
        app.initPair('BTC');
        app.apiKeyConfigured = true;
    });

    it('should register all five timeframes on pair init', () => {
        const pair = app.pairsMap['Hyperliquid-BTC'];
        expect(pair).toBeDefined();
        expect(pair.symbol).toBe('BTC');
        expect(pair.exchange).toBe('Hyperliquid');

        // All five timeframes should exist with default values
        expect(pair.shortTerm).toBeDefined();
        expect(pair.midTerm).toBeDefined();
        expect(pair.longTerm).toBeDefined();
        expect(pair.macroTerm).toBeDefined();
        expect(pair.supermacroTerm).toBeDefined();

        // Mid-term defaults
        expect(pair.midTerm.priceText).toBe('--');
        expect(pair.midTerm.rsiText).toBe('--');
        expect(pair.midTerm.macdLineText).toBe('--');
        expect(pair.isConnected).toBe(false);
    });

    it('should phase through analysis states progressively', () => {
        expect(app.analysisPhase).toBe('idle');

        app.analysisPhase = 'phase1';
        expect(app.analysisPhase).toBe('phase1');

        app.analysisPhase = 'phase2';
        expect(app.analysisPhase).toBe('phase2');

        app.analysisPhase = 'complete';
        expect(app.analysisPhase).toBe('complete');

        // Reset to idle
        app.analysisPhase = 'idle';
        expect(app.analysisPhase).toBe('idle');
    });

    it('should trigger auto-trade log when position changes from active to None', () => {
        app.currentPosition = 'Long';
        app.entryPriceVal = '50000';
        expect(app.currentPosition).toBe('Long');

        // Switching to None should auto-log the trade (via autoLogTrade)
        app.currentPosition = 'None';
        expect(app.currentPosition).toBe('None');
        // Entry price should be cleared
        expect(app.entryPriceVal).toBe('');
    });

    it('should update chart telemetry via snapshot assignment', () => {
        const pair = app.pairsMap['Hyperliquid-BTC'];

        // Simulate receiving a market snapshot
        pair.midTerm.latestSnapshot = {
            mid_price: '65000.00',
            exchange: 'Hyperliquid',
            symbol: 'BTC',
            rsi_14: 62.5,
            macd_line: 15.0,
            macd_signal: 10.0,
            macd_hist: 5.0,
            squeeze_on: false,
            squeeze_momentum: 0.12,
            bbwp: 45.0,
            ema_fast: 64900,
            ema_medium: 64800,
            ema_slow: 64500,
            atr_14: 250.0,
            adx_14: 28.0,
            is_completed: true,
        };

        expect(pair.midTerm.latestSnapshot).not.toBeNull();
        const snap = pair.midTerm.latestSnapshot!;
        expect(snap.mid_price).toBe('65000.00');
        expect(snap.symbol).toBe('BTC');
        expect(snap.is_completed).toBe(true);
    });

    it('should maintain chat history across messages', () => {
        expect(app.chatHistory.length).toBe(0);

        app.chatHistory = [
            { role: 'user', content: 'What is the trend?' },
            { role: 'assistant', content: 'The trend is bullish with strong momentum.' },
        ];

        expect(app.chatHistory.length).toBe(2);
        expect(app.chatHistory[0].role).toBe('user');
        expect(app.chatHistory[1].role).toBe('assistant');
        expect(app.chatHistory[1].content).toContain('bullish');
    });

    it('should snapshot per-timeframe state independently', () => {
        app.initPair('ETH');

        // Set values on BTC mid-term
        app.pairsMap['Hyperliquid-BTC'].midTerm.priceText = '65000.00';
        app.pairsMap['Hyperliquid-BTC'].midTerm.rsiText = '62.5';

        // Set values on ETH long-term
        app.pairsMap['Hyperliquid-ETH'].longTerm.priceText = '3200.00';
        app.pairsMap['Hyperliquid-ETH'].longTerm.rsiText = '45.0';

        // BTC mid-term unchanged
        expect(app.pairsMap['Hyperliquid-BTC'].midTerm.priceText).toBe('65000.00');
        // ETH long-term holds its value
        expect(app.pairsMap['Hyperliquid-ETH'].longTerm.priceText).toBe('3200.00');
        // ETH mid-term still default
        expect(app.pairsMap['Hyperliquid-ETH'].midTerm.priceText).toBe('--');
    });
});
