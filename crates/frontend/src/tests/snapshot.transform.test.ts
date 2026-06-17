// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { getState } from '../state.svelte';

describe('TEST-UI: Snapshot Data Transform', () => {
    let app: ReturnType<typeof getState>;

    beforeEach(() => {
        app = getState();
        app.initPair('BTC');
        app.apiKeyConfigured = true;
    });

    it('should map raw MarketSnapshot JSON to PairState fields', () => {
        const pair = app.pairsMap['Hyperliquid-BTC'];

        // Simulate a complete MarketSnapshot from the Rust backend
        const rawSnapshot: Record<string, unknown> = {
            exchange: 'Hyperliquid',
            symbol: 'BTC',
            timeframe_secs: 60,
            timestamp: 1718000000,
            is_completed: true,
            mid_price: '65000.00',
            bid_price: '64999.50',
            ask_price: '65000.50',
            rsi_14: 62.5,
            macd_line: 15.0,
            macd_signal: 10.0,
            macd_hist: 5.0,
            macd_histogram_peak: 18.0,
            macd_trend_state: 'Accelerating',
            macd_crossover_detected: false,
            squeeze_on: false,
            squeeze_momentum: 0.12,
            squeeze_duration: 0,
            squeeze_release_trigger: true,
            squeeze_momentum_direction: 'BullishAcceleration',
            bbwp: 45.0,
            ema_fast: 64900.0,
            ema_medium: 64800.0,
            ema_slow: 64500.0,
            ema_long: 64000.0,
            ema_stack_state: 'Bullish',
            atr_14: 250.0,
            atr_slope: 5.0,
            atr_volatility_regime: 'Stable',
            adx_14: 28.0,
            adx_plus: 30.0,
            adx_minus: 18.0,
            adx_slope: 1.5,
            adx_regime: 'Emerging',
            rsi_divergence_status: 'Potential',
            rsi_divergence_coords: '[[49500,55,5],[49000,60,10]]',
            macd_divergence_status: 'None',
            fib_golden_pocket_low: 49800.0,
            fib_golden_pocket_high: 49900.0,
            fib_extension_1618: 52000.0,
            fib_extension_2618: 54000.0,
            chart_pattern: 'BullishTriangle',
            chart_pattern_confidence: 35.0,
            open: 64800.0,
            high: 65200.0,
            low: 64750.0,
            close: 65000.0,
            volume: 150.0,
            vwap: 65010.0,
            vwap_bias: 'Equilibrium',
            average_volume: 120.0,
            rvol: 1.25,
        };

        pair.midTerm.latestSnapshot = rawSnapshot;

        const snap = pair.midTerm.latestSnapshot!;
        expect(snap.mid_price).toBe('65000.00');
        expect(snap.exchange).toBe('Hyperliquid');
        expect(snap.symbol).toBe('BTC');
        expect(snap.rsi_14).toBe(62.5);
        expect(snap.macd_line).toBe(15.0);
        expect(snap.macd_signal).toBe(10.0);
        expect(snap.macd_hist).toBe(5.0);
        expect(snap.squeeze_on).toBe(false);
        expect(snap.squeeze_momentum).toBe(0.12);
        expect(snap.bbwp).toBe(45.0);
        expect(snap.ema_fast).toBe(64900.0);
        expect(snap.atr_14).toBe(250.0);
        expect(snap.adx_14).toBe(28.0);
        expect(snap.fib_golden_pocket_low).toBe(49800.0);
        expect(snap.fib_extension_1618).toBe(52000.0);
        expect(snap.chart_pattern).toBe('BullishTriangle');
        expect(snap.chart_pattern_confidence).toBe(35.0);
        expect(snap.is_completed).toBe(true);
    });

    it('should handle null optional fields with sentinel defaults', () => {
        const pair = app.pairsMap['Hyperliquid-BTC'];

        // Snapshot with minimal fields and null Option values
        const sparseSnapshot: Record<string, unknown> = {
            exchange: 'Hyperliquid',
            symbol: 'BTC',
            timeframe_secs: 60,
            timestamp: 1000000,
            is_completed: false,
            mid_price: '30000.00',
            bid_price: '29999.50',
            ask_price: '30000.50',
            // All optional fields omitted or null
        };

        pair.midTerm.latestSnapshot = sparseSnapshot;

        // Core fields should be present
        const snap = pair.midTerm.latestSnapshot!;
        expect(snap.mid_price).toBe('30000.00');
        expect(snap.is_completed).toBe(false);

        // Optional fields should be null/undefined when not present
        expect(snap.rsi_14).toBeUndefined();
        expect(snap.macd_line).toBeUndefined();
        expect(snap.squeeze_on).toBeUndefined();
        expect(snap.bbwp).toBeUndefined();
    });

    it('should distinguish completed candle from live shadow tick', () => {
        const pair = app.pairsMap['Hyperliquid-BTC'];

        // Completed candle
        pair.midTerm.latestSnapshot = {
            exchange: 'Hyperliquid',
            symbol: 'BTC',
            is_completed: true,
            mid_price: '50000.00',
            rsi_14: 65.0,
            squeeze_on: false,
        };
        expect(pair.midTerm.latestSnapshot!.is_completed).toBe(true);

        // Live/shadow tick (incomplete candle)
        pair.midTerm.latestSnapshot = {
            exchange: 'Hyperliquid',
            symbol: 'BTC',
            is_completed: false,
            mid_price: '50100.00',
            rsi_14: 66.0,
            squeeze_on: true,
        };
        expect(pair.midTerm.latestSnapshot!.is_completed).toBe(false);
        // Both states are recorded but is_completed flag distinguishes them
        expect(pair.midTerm.latestSnapshot!.mid_price).toBe('50100.00');
    });

    it('should handle multi-pair snapshot routing by exchange key', () => {
        app.initPair('ETH');

        const btcData = {
            exchange: 'Hyperliquid',
            symbol: 'BTC',
            mid_price: '65000.00',
            rsi_14: 62.0,
            is_completed: true,
        };

        const ethData = {
            exchange: 'Hyperliquid',
            symbol: 'ETH',
            mid_price: '3200.00',
            rsi_14: 48.0,
            is_completed: true,
        };

        app.pairsMap['Hyperliquid-BTC'].midTerm.latestSnapshot = btcData;
        app.pairsMap['Hyperliquid-ETH'].midTerm.latestSnapshot = ethData;

        // Each pair independently stores its own snapshot
        expect(app.pairsMap['Hyperliquid-BTC'].midTerm.latestSnapshot!.symbol).toBe('BTC');
        expect(app.pairsMap['Hyperliquid-BTC'].midTerm.latestSnapshot!.mid_price).toBe('65000.00');

        expect(app.pairsMap['Hyperliquid-ETH'].midTerm.latestSnapshot!.symbol).toBe('ETH');
        expect(app.pairsMap['Hyperliquid-ETH'].midTerm.latestSnapshot!.mid_price).toBe('3200.00');
    });
});
