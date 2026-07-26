// Phase 2 tests — verify the export JSON includes all visible Metrics sections.

import { describe, it, expect } from 'vitest';
import { buildMetricsExportJson } from '../lib/metricsExport';

describe('metricsExport — covers all Metrics tab surfaces', () => {
    function makeTf() {
        return {
            slot: 'micro' as const,
            symbol: 'BTC-USDT',
            exchange: 'Hyperliquid',
            barDurationSec: 60,
            indicators: {
                rsi: {
                    raw_value: 28.5,
                    normalized: 0.75,
                    state_label: 'OVERSOLD_ACCUMULATION',
                    values: { period: 14 },
                    signals: [
                        {
                            kind: 'Crossover',
                            direction: 'Bullish',
                            status: 'Confirmed',
                            label: 'RSI bullish cross',
                            strength: 0.8,
                            age_bars: 2,
                        },
                    ],
                },
                fibonacci: {
                    raw_value: 65000,
                    normalized: 0,
                    state_label: 'GOLDEN_POCKET_TEST',
                    values: {
                        gp_top: 66800,
                        gp_bottom: 66400,
                        ext_1618: 69800,
                        ext_2618: 71200,
                        fib_0236: 67500,
                        fib_0382: 67200,
                        fib_0500: 66800,
                        fib_0618: 66500,
                        fib_0660: 66400,
                        fib_0786: 66000,
                    },
                    signals: [],
                },
            },
            priceText: '65000',
            volText: '120',
            avgVolText: '100',
            showPatterns: true,
            isCompleted: true,
            latestSnapshot: {} as Record<string, unknown>,
            historyPrices: [],
            context: {
                trend: { score: 0.5, confidence: 0.7, label: 'BULL' },
                momentum: { score: 0.4, confidence: 0.6, label: 'NEUTRAL' },
                regime: 'TRENDING_BULL',
                overall_score: 50,
                overall_label: 'BULL',
            } as Record<string, unknown>,
            volumeProfile: {
                symbol: 'BTC-USDT',
                timeframe_slot: 'micro',
                timeframe_secs: 60,
                bins: [
                    { price_low: 64500, price_high: 64600, volume: 100, buy_volume: 70, sell_volume: 30, is_poc: false, is_value_area: true },
                    { price_low: 64600, price_high: 64700, volume: 300, buy_volume: 200, sell_volume: 100, is_poc: true, is_value_area: true },
                ],
                poc_price: 64650,
                value_area_high: 64700,
                value_area_low: 64500,
                total_volume: 1500,
                range_low: 64000,
                range_high: 65000,
                num_bins: 30,
                timestamp_ms: 1700000000000,
            },
            liquidity: {
                long_liquidations_usd: 4000,
                short_liquidations_usd: 1500,
                net_liquidation_usd: 2500,
                event_count: 47,
                largest_event_usd: 1200,
                largest_event_price: 64800,
                largest_event_side: 'LONG',
                cascade_state: 'SUSTAINED',
                cascade_intensity: 82,
            },
            cluster: {
                symbol: 'BTC-USDT',
                generated_at_ms: 1700000000000,
                valid_until_ms: 1700000300000,
                mid_price: 65000,
                leverage_assumptions: {
                    source: 'DEFAULT_POWER_LAW',
                    buckets: [1, 5, 10, 20, 50, 100],
                    weights: [0.3, 0.25, 0.2, 0.15, 0.07, 0.03],
                    funding_extreme_pct: 0.05,
                    funding_modulation_active: true,
                },
                short_clusters: [
                    { price_low: 66000, price_high: 66200, peak_price: 66100, notional_usd: 1000000, dominant_leverage: 20, distance_from_mid_pct: 1.7, cluster_kind: 'ABOVE_CURRENT_PRICE', magnet_strength: 87 },
                ],
                long_clusters: [
                    { price_low: 64000, price_high: 64200, peak_price: 64100, notional_usd: 2000000, dominant_leverage: 20, distance_from_mid_pct: -1.4, cluster_kind: 'BELOW_CURRENT_PRICE', magnet_strength: 92 },
                ],
                cascade_asymmetry: 0.3,
                total_long_oi_usd: 50000000,
                total_short_oi_usd: 30000000,
                estimation_confidence: 0.85,
            },
            liquiditySignals: [
                { kind: 'CASCADE_DETECTED', direction: 'BEARISH', strength: 80, confidence: 0.78, evidence: ['cascade_above_threshold'] },
            ],
        } as any;
    }

    function makeAnalysis() {
        return {
            symbol: 'BTC-USDT',
            bias: 'Bullish',
            confidence: 0.8,
            state_confidence: 0.8,
            market_regime: 'TRENDING_BULL',
            trend_assessment: 'Strong',
            momentum_assessment: 'Stable',
            structure_assessment: 'Strong',
            volatility_assessment: 'Normal',
            volume_assessment: 'Strong',
            opportunity_analysis: 'TrendContinuation',
            market_quality: 'Excellent',
            market_quality_score: 90,
            market_interpretation: 'Trending up',
            rationale: 'Strong trend',
            supporting_signals: ['rsi', 'macd'],
            contradicting_signals: [],
            timeframes_considered: 4,
        } as any;
    }

    function makeRisk() {
        return {
            symbol: 'BTC-USDT',
            market_risk: { score: 35, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
            volatility_risk: { score: 45, level: 'MODERATE', state: 'STABLE', confidence: 80, evidence: [] },
            structure_risk: { score: 25, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
            momentum_risk: { score: 20, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
            signal_risk: { score: 30, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
            execution_risk: { score: 25, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
            execution_liquidity_risk: { score: 15, level: 'VERY_LOW', state: 'STABLE', confidence: 80, evidence: [] },
            cascade_risk: { score: 30, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
            overall_risk: { score: 28, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
        } as any;
    }

    function makeOpportunity() {
        return {
            symbol: 'BTC-USDT',
            primary_opportunity: 'TrendContinuation',
            opportunity_score: 82,
            setup_quality: 'PRIME',
            profiles: [],
            forecast_confidence: 0.72,
            contributing_signals: [],
            invalidation_note: 'Below VAL',
            entry_zone: { low: 64800, high: 65200 },
            target_zone: { low: 66000, high: 66500 },
            invalidation_level: 64200,
            expected_rr_internal: 2.5,
            time_horizon: 'SWING',
            confluent_entry_levels: [
                { price: 65000, confluence_count: 3, sources: ['FIBONACCI', 'VOLUME_PROFILE', 'SUPPORT_RESISTANCE'], strength: 0.86 },
            ],
            confluent_target_levels: [
                { price: 66200, confluence_count: 2, sources: ['VOLUME_PROFILE'], strength: 0.78 },
            ],
            confluent_invalidation_levels: [
                { price: 64200, confluence_count: 1, sources: ['PIVOT_POINTS'], strength: 0.55 },
            ],
        } as any;
    }

    function makeAdvisory() {
        return {
            symbol: 'BTC-USDT',
            directional_guidance: 'Long',
            market_stance: 'Constructive',
            strategy_environment: 'TrendFollowing',
            entry_guidance: 'Pullback',
            exit_guidance: 'TrendWeakening',
            protection_strategy: 'StructureBased',
            target_strategy: 'ResistanceBased',
            stop_loss_distance_pct: 0.0085,
            confidence_assessment: 73,
            trade_readiness: 'READY',
            entry_danger: { score: 30, level: 'LOW', state: 'STABLE', confidence: 80 },
            environment_favorability: { score: 30, level: 'LOW', state: 'STABLE', confidence: 80, evidence: [] },
            cascade_risk_score: 25,
            final_recommendation: 'Watch for pullback to GP zone',
        } as any;
    }

    function makeRegistry() {
        return [
            { key: 'rsi', display_name: 'RSI', group: 'Momentum', class: 'Hybrid', directional: true, supports_divergence: true, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'decimals2', value_source: 'sub:', color: '#fff', guide_section: '' },
            { key: 'fibonacci', display_name: 'Fibonacci', group: 'Structure', class: 'Hybrid', directional: true, supports_divergence: false, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'price', value_source: 'sub:gp_top', color: '#fff', guide_section: '' },
        ] as any;
    }

    it('includes all 7 top-level sections visible in the Metrics view', () => {
        const json = buildMetricsExportJson({
            symbol: 'BTC-USDT',
            tfLabel: 'Micro',
            tfSecs: 60,
            timestamp: 1700000000,
            markPrice: 65000,
            registry: makeRegistry(),
            tf: makeTf(),
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false },
            analysis: makeAnalysis(),
            risk: makeRisk(),
            alignment: null,
            opportunity: makeOpportunity(),
            advisory: makeAdvisory(),
            volumeProfile: makeTf().volumeProfile,
            liquidity: makeTf().liquidity,
            cluster: makeTf().cluster,
            liquiditySignals: makeTf().liquiditySignals,
            decisionContext: {
                score: 75.2,
                bias: 'Bullish',
                trade_readiness: 'READY',
                expected_reward_risk_ratio: 1.79,
                contributing_indicators: ['rsi', 'macd'],
                entry_danger: { score: 30, level: 'LOW', state: 'STABLE', confidence: 80 },
            },
        });
        const obj = JSON.parse(json);

        // Every section the user sees in the Metrics UI is in the JSON.
        expect(obj.timeframe.label).toBe('Micro');
        expect(obj.mark_price).toBe(65000);
        expect(Array.isArray(obj.indicators)).toBe(true);
        expect(obj.indicators.find((i: any) => i.key === 'rsi')).toBeDefined();
        expect(obj.indicators.find((i: any) => i.key === '__fibonacci_summary__')).toBeDefined();

        // Phase 2 — new sections
        expect(obj.opportunity.primary_opportunity).toBe('TrendContinuation');
        expect(obj.opportunity.entry_zone).toEqual({ low: 64800, high: 65200 });
        expect(obj.opportunity.target_zone).toEqual({ low: 66000, high: 66500 });
        expect(obj.opportunity.invalidation_level).toBe(64200);
        expect(obj.opportunity.confluent_entry_levels.length).toBe(1);

        expect(obj.advisory.directional_guidance).toBe('Long');
        expect(obj.advisory.protection_strategy).toBe('StructureBased');
        expect(obj.advisory.trade_readiness).toBe('READY');
        expect(obj.advisory.entry_danger.level).toBe('LOW');

        expect(obj.analysis.bias).toBe('Bullish');
        expect(obj.analysis.market_quality_score).toBe(90);

        expect(obj.risk.overall.score).toBe(28);
        expect(obj.risk.by_dimension.length).toBe(8);

        expect(obj.volume_profile.poc_price).toBe(64650);
        expect(obj.volume_profile.top_hvn.length).toBeGreaterThan(0);

        expect(obj.liquidity_flow.cascade_state).toBe('SUSTAINED');
        expect(obj.liquidity_flow.cascade_intensity).toBe(82);

        expect(obj.cluster_matrix.top_above.length).toBeGreaterThan(0);
        expect(obj.cluster_matrix.top_below.length).toBeGreaterThan(0);
        expect(obj.cluster_matrix.cascade_asymmetry).toBe(0.3);

        expect(obj.liquidity_signals.length).toBe(1);
        expect(obj.liquidity_signals[0].kind).toBe('CASCADE_DETECTED');
    });

    it('omits new sections gracefully when data is null', () => {
        const json = buildMetricsExportJson({
            symbol: 'BTC-USDT',
            tfLabel: 'Micro',
            tfSecs: 60,
            timestamp: null,
            markPrice: 0,
            registry: [],
            tf: { indicators: {} } as any,
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false },
            analysis: null,
            risk: null,
            alignment: null,
            opportunity: null,
            advisory: null,
            volumeProfile: null,
            liquidity: null,
            cluster: null,
            liquiditySignals: [],
            decisionContext: null,
        });
        const obj = JSON.parse(json);
        expect(obj.opportunity).toBeNull();
        expect(obj.advisory).toBeNull();
        expect(obj.risk).toBeNull();
        expect(obj.volume_profile).toBeNull();
        expect(obj.liquidity_flow).toBeNull();
        expect(obj.cluster_matrix).toBeNull();
    });

    it('extracts Fibonacci values from indicators.fibonacci.values even when an opportunity matrix exists', () => {
        const tf = makeTf();
        const json = buildMetricsExportJson({
            symbol: 'BTC-USDT',
            tfLabel: 'Micro',
            tfSecs: 60,
            timestamp: 1,
            markPrice: 65000,
            registry: makeRegistry(),
            tf,
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false },
            analysis: null,
            risk: null,
            alignment: null,
            opportunity: null,
            advisory: null,
            volumeProfile: null,
            liquidity: null,
            cluster: null,
            liquiditySignals: [],
            decisionContext: null,
        });
        const obj = JSON.parse(json);
        const fib = obj.indicators.find((i: any) => i.key === '__fibonacci_summary__');
        expect(fib).toBeDefined();
        expect(fib.sub_values.gp_top).toBe(66800);
        expect(fib.sub_values.gp_bottom).toBe(66400);
        expect(fib.sub_values.ext_1618).toBe(69800);
        expect(fib.sub_values.ext_2618).toBe(71200);
        expect(fib.sub_values.retracement_coefficients.fib_0618).toBe(66500);
    });
});
