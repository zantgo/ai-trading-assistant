// @vitest-environment jsdom
//
// LevelsView — liquidation-magnet distance formatting (audit fix):
//   LV-1: `distance_from_mid_pct` is an absolute percentage on the wire
//         (0.6 = 0.6%), so the magnet row must render "0.60%" — never a
//         100× inflated "60.00%".
//   LV-2: non-finite distances render as an em dash.

import { describe, expect, it } from 'vitest';
import { render } from '@testing-library/svelte';
import LevelsView from './LevelsView.svelte';
import type { IndicatorMeta, TimeframeTelemetry } from '../../types';

function makeTf(overrides: Partial<TimeframeTelemetry> = {}): TimeframeTelemetry {
    return {
        symbol: 'BTC-USDT',
        slot: 'micro',
        timeframeSecs: 60,
        barDurationSec: 60,
        priceText: '50000',
        indicators: {},
        latestSnapshot: null,
        context: null,
        liquidity: null,
        cluster: null,
        volumeProfile: null,
        liquiditySignals: [],
        indicatorLifecycle: {},
        pipelineState: 'Live',
        isCompleted: false,
        exchange: null,
        ...overrides,
    } as TimeframeTelemetry;
}

describe('LevelsView liquidation-magnet distance', () => {
    it('renders distance_from_mid_pct as an absolute percentage (0.6 → "0.60%")', () => {
        const tf = makeTf({
            cluster: {
                symbol: 'BTC-USDT',
                generated_at_ms: 1700000000000,
                valid_until_ms: 1700000300000,
                mid_price: 50000,
                leverage_assumptions: null,
                short_clusters: [{
                    price_low: 50100,
                    price_high: 50500,
                    peak_price: 50250,
                    notional_usd: 1500000,
                    dominant_leverage: 10,
                    distance_from_mid_pct: 0.6,
                    magnet_strength: 80,
                    kind: 'SHORT_CLUSTER',
                }],
                long_clusters: [],
                cascade_asymmetry: -0.4,
                total_long_oi_usd: 1e8,
                total_short_oi_usd: 1e8,
                estimation_confidence: 0.7,
            } as unknown as TimeframeTelemetry['cluster'],
        });
        const { container } = render(LevelsView, { tf, registry: [] as IndicatorMeta[] });
        const text = container.textContent ?? '';
        expect(text).toContain('0.60%');
        expect(text).not.toContain('60.00%');
    });

    it('renders an em dash for non-finite distances', () => {
        const tf = makeTf({
            cluster: {
                symbol: 'BTC-USDT',
                generated_at_ms: 1700000000000,
                valid_until_ms: 1700000300000,
                mid_price: 50000,
                leverage_assumptions: null,
                short_clusters: [{
                    price_low: 50100,
                    price_high: 50500,
                    peak_price: 50250,
                    notional_usd: 1500000,
                    dominant_leverage: 10,
                    distance_from_mid_pct: Number.NaN,
                    magnet_strength: 80,
                    kind: 'SHORT_CLUSTER',
                }],
                long_clusters: [],
                cascade_asymmetry: -0.4,
                total_long_oi_usd: 1e8,
                total_short_oi_usd: 1e8,
                estimation_confidence: 0.7,
            } as unknown as TimeframeTelemetry['cluster'],
        });
        const { container } = render(LevelsView, { tf, registry: [] as IndicatorMeta[] });
        const text = container.textContent ?? '';
        expect(text).toContain('—');
        expect(text).not.toMatch(/\d+\.\d{2}%/);
    });
});
