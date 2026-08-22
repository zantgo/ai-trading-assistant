// Tests for deriveTradePlan — pure helpers for the L4/L6 trade plan math.

import { deriveTradePlan } from '../lib/tradePlan';
import type { AnalysisMatrix, OpportunityMatrix, AdvisoryMatrix } from '../types';

describe('deriveTradePlan', () => {
    function makeArgs() {
        // mirror around 68000 for the SHORT side so per-side fields
        // produce a consistent SHORT bracket. The default plan under
        // test is LONG, so the SHORT fields are loaded only to validate
        // the direction-aware gates elsewhere.
        const mirror = (price: number): number => 2 * 68000 - price;
        return {
            symbol: 'BTC-USDT',
            markPrice: 68000,
            opportunity: {
                symbol: 'BTC-USDT',
                primary_opportunity: 'TrendContinuation',
                opportunity_score: 82,
                setup_quality: 'PRIME',
                profiles: [],
                forecast_confidence: 0.72,
                contributing_signals: [],
                invalidation_note: '',
                entry_zone: { low: 67800, high: 68200 },
                target_zone: { low: 69000, high: 69500 },
                invalidation_level: 67400,
                long_entry_zone: { low: 67800, high: 68200 },
                long_target_zone: { low: 69000, high: 69500 },
                long_invalidation_level: 67400,
                long_expected_rr_internal: 2.5,
                short_entry_zone: { low: mirror(68200), high: mirror(67800) },
                short_target_zone: { low: mirror(69500), high: mirror(69000) },
                short_invalidation_level: mirror(67400),
                short_expected_rr_internal: 2.5,
                time_horizon: 'SWING',
                confluent_entry_levels: [
                    { price: 68000, confluence_count: 3, sources: ['FIBONACCI', 'VOLUME_PROFILE', 'SUPPORT_RESISTANCE'], strength: 0.86 },
                ],
                confluent_target_levels: [
                    { price: 69200, confluence_count: 2, sources: ['VOLUME_PROFILE'], strength: 0.78 },
                    { price: 71000, confluence_count: 1, sources: ['FIBONACCI'], strength: 0.6 },
                ],
                confluent_invalidation_levels: [
                    { price: 67200, confluence_count: 1, sources: ['PIVOT_POINTS'], strength: 0.55 },
                ],
            },
            advisory: {
                symbol: 'BTC-USDT',
                directional_guidance: 'Long',
                market_stance: 'Constructive',
                strategy_environment: 'TrendFollowing',
                entry_guidance: 'Pullback',
                exit_guidance: 'TrendWeakening',
                protection_strategy: 'StructureBased',
                target_strategy: 'ResistanceBased',
                stop_loss_distance_pct: 2.5,
                confidence_assessment: 73,
                trade_readiness: 'READY',
                entry_danger: { level: 'Low' },
                environment_favorability: { score: 30, level: 'Low', state: 'Stable', confidence: 50, evidence: [] },
                cascade_risk_score: 25,
                final_recommendation: 'Watch for pullback to GP zone',
            } as any,
            analysis: null as any,
            decisionContext: {
                score: 75.2,
                bias: 'Bullish',
                trade_readiness: 'READY',
                expected_reward_risk_ratio: 1.79,
                contributing_indicators: ['rsi', 'macd', 'vwap', 'fibonacci'],
            },
            tf: undefined,
            microTf: undefined,
            overallRisk: 28.3,
        };
    }

    it('builds a LONG plan with TP1/TP2/TP3 ladder', () => {
        const plan = deriveTradePlan(makeArgs() as any);
        expect(plan.direction).toBe('LONG');
        expect(plan.setupType).toBe('TrendContinuation');
        expect(plan.timeHorizon).toBe('SWING');
        expect(plan.targets.length).toBeGreaterThanOrEqual(1);

        const tp1 = plan.targets[0];
        expect(tp1.label).toBe('TP1');
        expect(tp1.price).toBeGreaterThan(plan.entryMid);
        expect(tp1.sizePct).toBe(40);

        expect(plan.stop).not.toBeNull();
        expect(plan.stop!.price).toBe(67400);
    });

    it('sets actionable=true when readiness is READY and SL present', () => {
        const args = makeArgs();
        args.analysis = {
            symbol: 'BTC-USDT',
            bias: 'Bullish',
            confidence: 0.8,
            market_regime: 'TrendingBull',
            trend_assessment: 'Strong',
            momentum_assessment: 'Stable',
            structure_assessment: 'Strong',
            volatility_assessment: 'Normal',
            volume_assessment: 'Strong',
            market_quality: 'Excellent',
            market_quality_score: 90,
            market_interpretation: '',
            rationale: '',
            supporting_signals: [],
            contradicting_signals: [],
            timeframes_considered: 4,
        } as any;
        const plan = deriveTradePlan(args as any);
        expect(plan.readiness).toBe('READY');
        expect(plan.actionable).toBe(true);
        expect(plan.actionabilityReason).toBe('Actionable setup');
    });

    it('sets actionable=false when readiness is STAND_ASIDE', () => {
        const args = makeArgs();
        args.decisionContext = { ...args.decisionContext, trade_readiness: 'STAND_ASIDE' };
        args.analysis = {
            symbol: 'BTC-USDT',
            bias: 'Bullish',
            confidence: 0.8,
            market_regime: 'TrendingBull',
            trend_assessment: 'Strong',
            momentum_assessment: 'Stable',
            structure_assessment: 'Strong',
            volatility_assessment: 'Normal',
            volume_assessment: 'Strong',
            market_quality: 'Excellent',
            market_quality_score: 90,
            market_interpretation: '',
            rationale: '',
            supporting_signals: [],
            contradicting_signals: [],
            timeframes_considered: 4,
        } as any;
        const plan = deriveTradePlan(args as any);
        expect(plan.actionable).toBe(false);
        expect(plan.actionabilityReason).toMatch(/STAND ASIDE/);
    });

    it('marks plan as not actionable when stop is missing', () => {
        const args = makeArgs();
        if (args.opportunity) {
            // Zero out both the legacy AND per-side invalidation so the
            // direction-aware stop gate (LONG: inval < entry_mid) fires
            // on missing data.
            args.opportunity.invalidation_level = 0;
            args.opportunity.long_invalidation_level = 0;
            args.opportunity.short_invalidation_level = 0;
            args.opportunity.confluent_invalidation_levels = [];
        }
        const plan = deriveTradePlan(args as any);
        expect(plan.stop).toBeNull();
        expect(plan.actionable).toBe(false);
    });

    it('aggregates entry sources from confluent_entry_levels', () => {
        const plan = deriveTradePlan(makeArgs() as any);
        expect(plan.entrySources.length).toBe(1);
        expect(plan.entrySources[0].tag).toBe('FIB');
    });

    it('caps target count by time_horizon', () => {
        const args = makeArgs();
        args.opportunity = args.opportunity && { ...args.opportunity, time_horizon: 'SCALP' };
        const plan = deriveTradePlan(args as any);
        expect(plan.timeHorizon).toBe('SCALP');
        expect(plan.targets.length).toBe(1);
    });

    it('derives R:R for each target', () => {
        const plan = deriveTradePlan(makeArgs() as any);
        for (const t of plan.targets) {
            expect(t.rrRatio).not.toBeNull();
            expect(t.rrRatio as number).toBeGreaterThan(0);
        }
    });

    it('handles NEUTRAL direction when advisory guidance is unknown', () => {
        const args = makeArgs();
        args.advisory = { ...args.advisory, directional_guidance: 'AvoidDirectionalExposure' } as any;
        const plan = deriveTradePlan(args as any);
        expect(plan.direction).toBe('NEUTRAL');
    });

    it('uses the percent-scale advisory stop for the fallback price (no negative)', () => {
        // Regression: stop_loss_distance_pct is percent-scale on the wire
        // (2.5 = 2.5%). The old fraction-scale ×100 produced a LONG
        // fallback stop of entryMid × (1 − 2.5) = a NEGATIVE price.
        const args = makeArgs();
        const entryMid = 68000;
        args.advisory = { ...args.advisory, stop_loss_distance_pct: 2.5 } as any;

        const plan = deriveTradePlan(args as any);
        expect(plan.stop).not.toBeNull();
        expect(plan.stop!.fallbackPrice).toBeDefined();
        const fallback = Number(String(plan.stop!.fallbackPrice).replace(/[^0-9.]/g, ''));
        expect(fallback).toBeGreaterThan(0);
        // entryMid × (1 − 0.025) ≈ 66300.
        expect(Math.abs(fallback - entryMid * 0.975)).toBeLessThan(50);
    });
});
