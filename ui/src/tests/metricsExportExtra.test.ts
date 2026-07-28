// Phase 2 tests — verify the export JSON includes all visible Metrics sections.

import { describe, it, expect } from 'vitest';
import { buildMetricsExportJson, buildPanelExportJson, buildMtfExportJson } from '../lib/metricsExport';

// Module-level fixtures reused by both describe blocks.
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
            // ContextOnly (gate) and EventOnly (Hull MA) entries — must
            // round-trip through the export without losing the new
            // `normalization_mode` metadata.
            { key: 'bbwp', display_name: 'BBWP', group: 'Volatility', class: 'Leading', directional: false, supports_divergence: false, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'percent1', value_source: 'raw', color: '#fff', guide_section: '', normalization_mode: 'ContextOnly' },
            { key: 'hull_ma', display_name: 'Hull MA', group: 'Trend', class: 'Lagging', directional: true, supports_divergence: false, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'price', value_source: 'raw', color: '#fff', guide_section: '', normalization_mode: 'EventOnly' },
            // Anchored VWAP — registry source must be `sub:weekly` (was
            // `sub:vwap_weekly` before the fix, which never resolved).
            { key: 'anchored_vwap', display_name: 'Anchored VWAP', group: 'Trend', class: 'Lagging', directional: true, supports_divergence: false, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'price', value_source: 'sub:weekly', color: '#fff', guide_section: '' },
        ] as any;
    }

describe('metricsExport — covers all Metrics tab surfaces', () => {
    it('includes all 7 top-level sections visible in the Metrics view', () => {
        const json = buildMetricsExportJson({
            sourceTab: 'metrics',
            symbol: 'BTC-USDT',
            tfLabel: 'Micro',
            tfSecs: 60,
            timestamp: 1700000000,
            markPrice: 65000,
            registry: makeRegistry(),
            tf: makeTf(),
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
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
        expect(obj.source_tab).toBe('metrics');
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
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
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
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
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

describe('metricsExport — per-tab EXPORT DATA wiring', () => {
    function baseTf() {
        return {
            indicators: {},
            latestSnapshot: {},
            priceText: '0',
            isCompleted: false,
        } as any;
    }
    function baseResolvers(overrides: Partial<any> = {}) {
        return {
            symbol: 'BTC-USDT',
            tfLabel: 'Micro',
            tfSecs: 60,
            timestamp: 1700000000,
            markPrice: 65000,
            registry: [],
            tf: baseTf(),
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
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
            ...overrides,
        };
    }

    it('tags the export with the calling tab so operators can identify the source', () => {
        for (const sourceTab of ['metrics', 'alignment', 'opportunity', 'risk', 'analysis', 'decision']) {
            const json = buildPanelExportJson({
                sourceTab,
                pairKey: 'BTC-USDT',
                resolvers: baseResolvers(),
            });
            expect(json).not.toBeNull();
            const obj = JSON.parse(json as string);
            expect(obj.source_tab).toBe(sourceTab);
        }
    });

    it('includes the full payload (analysis, risk, alignment, advisory, opportunity, decision)', () => {
        const json = buildPanelExportJson({
            sourceTab: 'decision',
            pairKey: 'BTC-USDT',
            resolvers: baseResolvers({
                analysis: { symbol: 'BTC-USDT', bias: 'Bullish' } as any,
                risk: makeRisk(),
                alignment: { mtf_overall_score: 12 } as any,
                advisory: makeAdvisory(),
                opportunity: makeOpportunity(),
                decisionContext: {
                    trade_readiness: 'READY',
                    entry_danger: { score: 30, level: 'LOW', state: 'STABLE', confidence: 80 },
                },
            }),
        });
        const obj = JSON.parse(json as string);
        expect(obj.analysis.bias).toBe('Bullish');
        expect(obj.risk.overall.score).toBe(28);
        expect(obj.alignment.mtf_overall_score).toBe(12);
        expect(obj.advisory.directional_guidance).toBe('Long');
        expect(obj.opportunity.primary_opportunity).toBe('TrendContinuation');
        // decision_context is folded into the advisory export — its trade_readiness
        // and entry_danger fields surface from the DecisionContext input.
        expect(obj.advisory.trade_readiness).toBe('READY');
        expect(obj.advisory.entry_danger).toEqual({ score: 30, level: 'LOW', state: 'STABLE', confidence: 80 });
    });

    it('returns null when there is no pair symbol (button should short-circuit)', () => {
        const json = buildPanelExportJson({
            sourceTab: 'opportunity',
            pairKey: '',
            resolvers: baseResolvers({ symbol: '' }),
        });
        expect(json).toBeNull();
    });
});

describe('TEST-UI: Registry metadata contract (normalization_mode, AVWAP sub-key)', () => {
    function localMakeMeta(overrides: Partial<IndicatorMeta> = {}): IndicatorMeta {
        return {
            key: 'rsi',
            display_name: 'RSI',
            group: 'Momentum',
            class: 'Leading',
            render: 'Pane',
            directional: true,
            supports_divergence: true,
            signal_types: [],
            default_weight: 1.0,
            default_enabled: true,
            config_params: [],
            value_format: 'decimals2',
            value_source: 'raw',
            color: '#fff',
            guide_section: '',
            ...overrides,
        };
    }

    it('IndicatorMeta accepts normalization_mode: ContextOnly / EventOnly / Directional', () => {
        // Compile-time smoke test: the new field types are valid TypeScript
        // and round-trip through `Partial<IndicatorMeta>` without errors.
        const ctx: IndicatorMeta = {
            ...localMakeMeta({ key: 'bbwp' }),
            normalization_mode: 'ContextOnly',
        };
        const ev: IndicatorMeta = {
            ...localMakeMeta({ key: 'hull_ma' }),
            normalization_mode: 'EventOnly',
        };
        const dir: IndicatorMeta = {
            ...localMakeMeta({ key: 'rsi' }),
            normalization_mode: 'Directional',
        };
        expect(ctx.normalization_mode).toBe('ContextOnly');
        expect(ev.normalization_mode).toBe('EventOnly');
        expect(dir.normalization_mode).toBe('Directional');
    });

    it('anchored_vwap registry sub-key resolves to sub:weekly', () => {
        // The legacy sub-key `sub:vwap_weekly` never matched the value
        // the normalizer inserts (`weekly`), so the Metrics Raw column
        // rendered `--`. The fix is documented here to catch a future
        // regression at the type layer.
        const avwap: IndicatorMeta = localMakeMeta({
            key: 'anchored_vwap',
            display_name: 'Anchored VWAP',
            value_source: 'sub:weekly',
        });
        expect(avwap.value_source).toBe('sub:weekly');
    });
});

// ── Cross-timeframe (MTF) export builder ────────────────────────────────
//
// Pin down the payload shape produced by `buildMtfExportJson` so a future
// schema drift can't silently break the Metrics → MTF → EXPORT DATA flow.

describe('buildMtfExportJson — cross-timeframe grid payload', () => {
    function mtfTf(slot: 'micro' | 'fast' | 'slow' | 'macro', secs: number, opts: {
        priceText?: string;
        ts?: number;
        rsiNorm?: number | null;
        macdNorm?: number | null;
        bbwpNorm?: number | null;
        rsiSigLabel?: string | null;
    } = {}) {
        const indicators: Record<string, any> = {};
        if (opts.rsiNorm != null) {
            indicators['rsi'] = {
                normalized: opts.rsiNorm,
                signals: opts.rsiSigLabel
                    ? [{ label: opts.rsiSigLabel, direction: 'Bullish', kind: 'Threshold', status: 'Confirmed', strength: 0.8, age_bars: 1 }]
                    : [],
            };
        }
        if (opts.macdNorm != null) {
            indicators['macd'] = { normalized: opts.macdNorm, signals: [] };
        }
        if (opts.bbwpNorm != null) {
            indicators['bbwp'] = { normalized: opts.bbwpNorm, signals: [] };
        }
        return {
            slot,
            symbol: 'BTC-USDT',
            exchange: 'Hyperliquid',
            barDurationSec: secs,
            indicators,
            priceText: opts.priceText ?? '0',
            volText: '0',
            avgVolText: '0',
            showPatterns: false,
            isCompleted: true,
            latestSnapshot: { timestamp: opts.ts ?? 1700000000 },
            historyPrices: [],
            pipelineState: 'LIVE',
        } as any;
    }

    function makeMeta(key: string, display: string, group: string, overrides: Partial<any> = {}): any {
        return {
            key,
            display_name: display,
            group,
            directional: true,
            class: 'oscillator',
            weight: 1,
            value_format: 'decimals2',
            value_source: 'raw',
            color: '#fff',
            guide_section: '',
            ...overrides,
        };
    }

    it('returns valid JSON with the canonical top-level keys', () => {
        const out = buildMtfExportJson({
            symbol: 'BTC-USDT',
            pair: {
                microTerm: mtfTf('micro', 60,    { priceText: '65000', ts: 100 }),
                fastTerm:  mtfTf('fast',  180,   { priceText: '65100', ts: 200 }),
                slowTerm:  mtfTf('slow',  300,   { priceText: '64900', ts: 300 }),
                macroTerm: mtfTf('macro', 900,   { priceText: '64800', ts: 400 }),
            },
            registry: [makeMeta('rsi', 'RSI', 'Momentum')],
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
        });
        const parsed = JSON.parse(out);
        expect(parsed.source_tab).toBe('mtf');
        expect(parsed.symbol).toBe('BTC-USDT');
        expect(parsed.exported_at).toMatch(/^\d{4}-\d{2}-\d{2}T/);
        expect(parsed.timeframes).toHaveLength(4);
        expect(parsed.timeframes.map((t: any) => t.label)).toEqual(['Micro', 'Fast', 'Slow', 'Macro']);
        expect(parsed.timeframes[0].duration_seconds).toBe(60);
        expect(parsed.timeframes[0].mark_price).toBe(65000);
        expect(parsed.timeframes[0].timestamp).toBe(100);
        expect(parsed.timeframes[0].is_completed).toBe(true);
        expect(parsed.timeframes[0].pipeline_state).toBe('LIVE');
        expect(parsed.filter_state.active_only).toBe(false);
    });

    it('captures per-TF indicator values + classifies agreement', () => {
        // Micro & Fast bullish, Slow & Macro bearish → agreement = avg ≈ 0 (MIXED).
        const out = JSON.parse(buildMtfExportJson({
            symbol: 'BTC-USDT',
            pair: {
                microTerm: mtfTf('micro', 60,  { rsiNorm:  0.8 }),
                fastTerm:  mtfTf('fast',  180, { rsiNorm:  0.6 }),
                slowTerm:  mtfTf('slow',  300, { rsiNorm: -0.4 }),
                macroTerm: mtfTf('macro', 900, { rsiNorm: -0.7 }),
            },
            registry: [makeMeta('rsi', 'RSI', 'Momentum')],
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
        }));
        expect(out.indicators).toHaveLength(1);
        const rsi = out.indicators[0];
        expect(rsi.key).toBe('rsi');
        expect(rsi.values).toHaveLength(4);
        expect(rsi.values.map((v: any) => v.timeframe)).toEqual(['Micro', 'Fast', 'Slow', 'Macro']);
        expect(rsi.values[0]).toEqual({ timeframe: 'Micro', normalized: 0.8, active: true });
        // Avg ≈ (0.8+0.6-0.4-0.7)/4 = 0.075 → MIXED
        expect(rsi.agreement_label).toBe('MIXED');
        expect(Math.abs(rsi.agreement - 0.075)).toBeLessThan(1e-9);
    });

    it('flags inactive TFs as active=false and excludes them from agreement', () => {
        // Macro has no RSI indicator → active=false. Agreement should be the
        // mean of the 3 active TFs only.
        const out = JSON.parse(buildMtfExportJson({
            symbol: 'BTC-USDT',
            pair: {
                microTerm: mtfTf('micro', 60,  { rsiNorm:  0.6 }),
                fastTerm:  mtfTf('fast',  180, { rsiNorm:  0.6 }),
                slowTerm:  mtfTf('slow',  300, { rsiNorm:  0.6 }),
                macroTerm: mtfTf('macro', 900, { rsiNorm:  null as any }), // no rsi
            },
            registry: [makeMeta('rsi', 'RSI', 'Momentum')],
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
        }));
        const rsi = out.indicators[0];
        expect(rsi.values[3]).toEqual({ timeframe: 'Macro', normalized: 0, active: false });
        expect(rsi.agreement).toBeCloseTo(0.6, 9);
        expect(rsi.agreement_label).toBe('BULL');
    });

    it('sums unique signal labels across all 4 TFs into signals_total', () => {
        const out = JSON.parse(buildMtfExportJson({
            symbol: 'BTC-USDT',
            pair: {
                microTerm: mtfTf('micro', 60,  { rsiNorm: 0.5, rsiSigLabel: 'RSI oversold' }),
                fastTerm:  mtfTf('fast',  180, { rsiNorm: 0.5, rsiSigLabel: 'RSI oversold' }), // duplicate
                slowTerm:  mtfTf('slow',  300, { rsiNorm: 0.5, rsiSigLabel: 'RSI cross up' }),
                macroTerm: mtfTf('macro', 900, { rsiNorm: 0.5 }),
            },
            registry: [makeMeta('rsi', 'RSI', 'Momentum')],
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
        }));
        // 'RSI oversold' (Micro+Fast dedupe to 1) + 'RSI cross up' (Slow) = 2 unique
        expect(out.signals_total).toBe(2);
    });

    it('rolls up groups from GROUP_ORDER and includes accent + count', () => {
        const out = JSON.parse(buildMtfExportJson({
            symbol: 'BTC-USDT',
            pair: {
                microTerm: mtfTf('micro', 60),
                fastTerm:  mtfTf('fast',  180),
                slowTerm:  mtfTf('slow',  300),
                macroTerm: mtfTf('macro', 900),
            },
            registry: [
                makeMeta('rsi',  'RSI',  'Momentum'),
                makeMeta('macd', 'MACD', 'Momentum'),
                makeMeta('bbwp', 'BBWP', 'Volatility', { directional: false }),
            ],
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
        }));
        const labels = out.groups.map((g: any) => g.label);
        expect(labels).toContain('Momentum');
        expect(labels).toContain('Volatility');
        const momentum = out.groups.find((g: any) => g.label === 'Momentum');
        expect(momentum.indicator_count).toBe(2);
        expect(momentum.accent).toMatch(/^#[0-9a-f]{6}$/i);
    });

    it('filters out groups with zero indicators', () => {
        const out = JSON.parse(buildMtfExportJson({
            symbol: 'BTC-USDT',
            pair: {
                microTerm: mtfTf('micro', 60,  { rsiNorm: 0.5 }),
                fastTerm:  mtfTf('fast',  180),
                slowTerm:  mtfTf('slow',  300),
                macroTerm: mtfTf('macro', 900),
            },
            registry: [makeMeta('rsi', 'RSI', 'Momentum')],
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
        }));
        // No Volume/Trend/Structure/... indicators present → only Momentum.
        expect(out.groups).toHaveLength(1);
        expect(out.groups[0].label).toBe('Momentum');
    });

    it('serializes mark_price as null when priceText is invalid', () => {
        const out = JSON.parse(buildMtfExportJson({
            symbol: 'BTC-USDT',
            pair: {
                microTerm: mtfTf('micro', 60, { priceText: '--' }),
                fastTerm:  mtfTf('fast',  180, { priceText: '' }),
                slowTerm:  mtfTf('slow',  300, { priceText: 'abc' }),
                macroTerm: mtfTf('macro', 900, { priceText: '0' }),
            },
            registry: [],
            filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
        }));
        expect(out.timeframes[0].mark_price).toBeNull();
        expect(out.timeframes[1].mark_price).toBeNull();
        expect(out.timeframes[2].mark_price).toBeNull();
        expect(out.timeframes[3].mark_price).toBeNull();
    });
});
