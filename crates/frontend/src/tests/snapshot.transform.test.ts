// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { useAppStore } from '../state.svelte';
import { applySnapshotToTimeframe } from '../lib/websocket.svelte';
import { iRaw, iSub, emaStackState, vwapBias, adxRegime, divStatus, isSqueezeOn } from '../lib/telemetry';
import type { TimeframeTelemetry } from '../types';

/** Wrap a nested snapshot into a JSON-RPC broadcast MessageEvent. */
function wsEvent(snapshot: Record<string, unknown>): MessageEvent {
    return {
        data: JSON.stringify({
            jsonrpc: '2.0',
            method: 'broadcast.market_snapshot',
            params: { symbol: 'BTC', timeframe_secs: 60, snapshot },
        }),
    } as MessageEvent;
}

function nestedSnapshot(): Record<string, unknown> {
    return {
        symbol: 'BTC',
        timeframe_secs: 60,
        is_completed: true,
        mid_price: '65000.00',
        volume: 150.0,
        average_volume: 120.0,
        indicators: {
            rsi: { raw_value: 28.5, normalized: 0.75, state_label: 'OVERSOLD_ACCUMULATION' },
            macd: {
                raw_value: 5.2,
                normalized: 0.85,
                state_label: 'BULLISH_CROSSOVER_ACCELERATING',
                values: { line: -12.4, signal: -17.6, histogram: 5.2, histogram_peak: 8.0 },
            },
            squeeze: { raw_value: 0.12, normalized: 0.65, state_label: 'BULLISH_EXPANSION_ACCELERATING' },
            ema_stack: {
                raw_value: 65000,
                normalized: 1.0,
                state_label: 'ESTABLISHED_BULLISH_STACK',
                values: { fast: 64900, medium: 64800, slow: 64500, long: 64000 },
            },
            bbwp: { raw_value: 45.0, normalized: 0.5, state_label: 'NORMAL_VOLATILITY_BULL_CYCLE' },
            rvol: { raw_value: 1.25, normalized: 0.2, state_label: 'NORMAL_PARTICIPATION_VOLUME' },
            adx: {
                raw_value: 28.0,
                normalized: 0.6,
                state_label: 'STRONG_BULL_TREND',
                values: { adx: 28.0, plus_di: 30.0, minus_di: 18.0, adx_slope: 1.5 },
            },
            vwap: {
                raw_value: 65010,
                normalized: 0.0,
                state_label: 'INTRA_DAY_VALUE_EQUILIBRIUM',
                values: { vwap: 65010, price: 65000 },
            },
            rsi_divergence: { raw_value: 0.5, normalized: 0.5, state_label: 'POTENTIAL_BULLISH_DIVERGENCE' },
        },
    };
}

describe('TEST-UI: Nested Snapshot Transform (v2.0)', () => {
    let app: ReturnType<typeof useAppStore>;

    beforeEach(() => {
        app = useAppStore();
        app.initInstance('BTC');
    });

    it('parses the nested indicators map into the state rune', () => {
        const tf: TimeframeTelemetry = app.instancesMap['BTC-USDT'].microTerm;
        applySnapshotToTimeframe(tf, wsEvent(nestedSnapshot()));

        // Nested map is the source of truth.
        expect(tf.indicators['rsi'].normalized).toBe(0.75);
        expect(tf.indicators['rsi'].state_label).toBe('OVERSOLD_ACCUMULATION');
        expect(tf.indicators['macd'].state_label).toBe('BULLISH_CROSSOVER_ACCELERATING');
        expect(tf.indicators['macd'].values!.line).toBe(-12.4);
        expect(tf.isCompleted).toBe(true);
    });

    it('exposes indicator values via the shared telemetry accessors', () => {
        const tf = app.instancesMap['BTC-USDT'].microTerm;
        applySnapshotToTimeframe(tf, wsEvent(nestedSnapshot()));

        // Core (non-indicator) market data stays as flat text, price-scaled.
        expect(tf.priceText).toBe('65000.0');

        // All indicator-derived values come from the nested map (single source
        // of truth) — no legacy flat fields remain on TimeframeTelemetry.
        const m = tf.indicators;
        expect(iRaw(m, 'rsi')).toBe(28.5);
        expect(iSub(m, 'macd', 'line')).toBe(-12.4);
        expect(emaStackState(m)).toBe('bullish');
        expect(iSub(m, 'ema_stack', 'fast')).toBe(64900);
        expect(vwapBias(m)).toBe('equilibrium');
        expect(adxRegime(m)).toBe('strong');
        expect(divStatus(m, 'rsi_divergence')).toBe('potential');
        expect(isSqueezeOn(m)).toBe(false);
        expect(iRaw(m, 'rvol')).toBe(1.25);
    });

    it('renders the backend state_label verbatim (no client re-derivation)', () => {
        const tf = app.instancesMap['BTC-USDT'].microTerm;
        applySnapshotToTimeframe(tf, wsEvent(nestedSnapshot()));
        // The TelemetryTable binds directly to these labels.
        expect(tf.indicators['squeeze'].state_label).toBe('BULLISH_EXPANSION_ACCELERATING');
        expect(tf.indicators['bbwp'].state_label).toBe('NORMAL_VOLATILITY_BULL_CYCLE');
        expect(tf.indicators['rvol'].state_label).toBe('NORMAL_PARTICIPATION_VOLUME');
    });

    it('falls back to safe sentinels when indicators are absent', () => {
        const tf = app.instancesMap['BTC-USDT'].microTerm;
        applySnapshotToTimeframe(
            tf,
            wsEvent({ symbol: 'BTC', is_completed: false, mid_price: '30000.00' }),
        );
        expect(tf.indicators).toEqual({});
        expect(tf.priceText).toBe('30000.0');
        expect(tf.isCompleted).toBe(false);
        // Accessing a missing indicator is safe via optional chaining.
        expect(tf.indicators['rsi']?.state_label ?? 'UNKNOWN').toBe('UNKNOWN');
        expect(tf.indicators['rsi']?.normalized ?? 0).toBe(0);
    });

    it('routes nested snapshots independently per pair', () => {
        app.initInstance('ETH');
        const btc = app.instancesMap['BTC-USDT'].microTerm;
        const eth = app.instancesMap['ETH-USDT'].microTerm;

        applySnapshotToTimeframe(btc, wsEvent(nestedSnapshot()));
        applySnapshotToTimeframe(
            eth,
            wsEvent({
                symbol: 'ETH',
                is_completed: true,
                mid_price: '3200.00',
                indicators: {
                    rsi: { raw_value: 72.0, normalized: -0.75, state_label: 'OVERBOUGHT_DISTRIBUTION' },
                },
            }),
        );

        expect(btc.indicators['rsi'].state_label).toBe('OVERSOLD_ACCUMULATION');
        expect(eth.indicators['rsi'].state_label).toBe('OVERBOUGHT_DISTRIBUTION');
        expect(eth.priceText).toBe('3200.00');
    });

    it('caches decision_context from completed candles and holds it across shadow snapshots', () => {
        const tf = app.instancesMap['BTC-USDT'].microTerm;
        expect(tf.decisionContext).toBeNull();

        const completed = nestedSnapshot();
        completed.decision_context = {
            bullish_probability: 0.72, bearish_probability: 0.28, directional_bias: 0.44,
            consensus: 0.6, expected_range_1bar: 0.01, expected_range_5bar: 0.02,
            expected_range_20bar: 0.04, expected_volatility: 25, confluence: 0.5,
            risk_level: 0.3, reward_risk_ratio: 2.5, recommended_stop: 64000,
            trade_quality: 0.7, market_quality: 0.65, regime_confidence: 0.8,
            trend_persistence: 0.55, trade_readiness: 0.68,
        };
        applySnapshotToTimeframe(tf, wsEvent(completed));
        expect(tf.decisionContext).not.toBeNull();
        expect(tf.decisionContext!.trade_readiness).toBe(0.68);

        // Shadow (flicker) snapshot omits decision_context → cached value persists.
        applySnapshotToTimeframe(
            tf,
            wsEvent({ symbol: 'BTC', is_completed: false, mid_price: '65100.00' }),
        );
        expect(tf.isCompleted).toBe(false);
        expect(tf.decisionContext!.trade_readiness).toBe(0.68);
    });
});
