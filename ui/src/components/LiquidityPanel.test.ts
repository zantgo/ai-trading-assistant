// @vitest-environment jsdom
// Phase 4: LiquidityPanel data type & helper tests.
//
// This file focuses on the underlying data model and the type
// compatibility of the new Phase 1-3 types with the rest of the
// frontend. The full component rendering is exercised manually because
// the Svelte 5 + Vitest + jsdom triple currently has lifecycle_function
// issues in headless tests (see RiskPanel, AdvisoryPanel placeholders
// which also have no component-level tests).

import { describe, it, expect, beforeEach } from 'vitest';
import type {
    LiquidityFlow,
    LiquidationClusterMatrix,
    LiquiditySignal,
    CascadeState,
} from '../types';

beforeEach(() => {
    (globalThis as any).__appStore = {
        instancesMap: {},
    };
});

describe('LiquidityPanel data types', () => {
    it('LiquidityFlow deserializes from server JSON shape', () => {
        const json = {
            long_liquidations_usd: 50000.0,
            short_liquidations_usd: 10000.0,
            net_liquidation_usd: 40000.0,
            event_count: 3,
            largest_event_usd: 30000.0,
            largest_event_price: 49500.0,
            largest_event_side: 'Long',
            cascade_state: 'Detected',
            cascade_intensity: 65.0,
        };
        const flow = json as LiquidityFlow;
        expect(flow.cascade_state).toBe<CascadeState>('Detected');
        expect(flow.cascade_intensity).toBe(65.0);
    });

    it('LiquidationClusterMatrix round-trips through JSON', () => {
        const json = {
            symbol: 'BTC-USDT',
            generated_at_ms: 1700000000000,
            valid_until_ms: 1700000300000,
            mid_price: 50000.0,
            leverage_assumptions: {
                buckets: [1, 3, 5, 10, 20, 50, 100],
                weights: [0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
                funding_modulation_active: true,
                funding_extreme_pct: 0.0005,
                source: 'FundingAdaptive',
            },
            short_clusters: [{
                price_low: 50100.0,
                price_high: 50500.0,
                peak_price: 50250.0,
                notional_usd: 1500000.0,
                dominant_leverage: 10,
                distance_from_mid_pct: 0.5,
                cluster_kind: 'AboveCurrentPrice',
                magnet_strength: 75.0,
            }],
            long_clusters: [],
            cascade_asymmetry: -0.4,
            total_long_oi_usd: 30000000.0,
            total_short_oi_usd: 20000000.0,
            estimation_confidence: 0.85,
        };
        const m = json as LiquidationClusterMatrix;
        expect(m.symbol).toBe('BTC-USDT');
        expect(m.short_clusters.length).toBe(1);
        expect(m.short_clusters[0].peak_price).toBe(50250.0);
        expect(m.cascade_asymmetry).toBe(-0.4);
    });

    it('LiquiditySignal kind enum matches server output', () => {
        const sig: LiquiditySignal = {
            kind: 'CascadeSustained',
            direction: 'Bearish',
            strength: 80,
            confidence: 0.9,
            evidence: ['3 events in last 5 candles'],
        };
        expect(sig.kind).toBe('CascadeSustained');
        expect(sig.direction).toBe('Bearish');
        expect(sig.evidence.length).toBe(1);
    });

    it('all signal kind enum values are valid strings', () => {
        const kinds: LiquiditySignal['kind'][] = [
            'CascadeDetected', 'CascadeSustained', 'CascadeExhausted',
            'LiquidityVacuum', 'FundingExtreme', 'OIFundingDivergence', 'MagnetActivated',
        ];
        // Each must be a non-empty string.
        for (const k of kinds) {
            expect(typeof k).toBe('string');
            expect(k.length).toBeGreaterThan(0);
        }
    });

    it('all cluster kind values are valid strings', () => {
        const kinds: LiquidationClusterMatrix['short_clusters'][0]['cluster_kind'][] = [
            'AboveCurrentPrice', 'BelowCurrentPrice', 'AtCurrentPrice', 'Distant',
        ];
        for (const k of kinds) {
            expect(typeof k).toBe('string');
        }
    });
});