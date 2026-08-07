// @vitest-environment jsdom
//
// GeneralDashboard — Market Overview operator dashboard.
//
// These tests cover the three hero states (TRADE / WAIT / STAND ASIDE),
// the 4-up card row, the 9-column Asset Rankings table, and the
// Risk Distribution card's wire-side behaviour (reading from
// OverviewMatrix.risk_distribution when available, falling back to local
// aggregation when not).

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import GeneralDashboard from './GeneralDashboard.svelte';
import { useAppStore } from '../state.svelte';
import type {
    AdvisoryMatrix,
    AnalysisMatrix,
    DecisionContext,
    InstanceState,
    OpportunityMatrix,
    OverviewMatrix,
    RiskDimension,
    RiskMatrix,
} from '../types';

function makeDanger(score: number, overrides: Partial<RiskDimension> = {}): RiskDimension {
    return {
        score,
        level: score >= 80 ? 'Extreme' : score >= 60 ? 'High' : score >= 40 ? 'Moderate' : score >= 20 ? 'Low' : 'VeryLow',
        state: 'Stable',
        confidence: 50,
        evidence: [],
        ...overrides,
    };
}

function makeAdvisory(overrides: Partial<AdvisoryMatrix> = {}): AdvisoryMatrix {
    return {
        symbol: 'BTC-USDT',
        directional_guidance: 'Long',
        market_stance: 'Constructive',
        opportunity_classification: 'Breakout',
        strategy_environment: 'TrendFollowing',
        entry_guidance: 'Immediate',
        exit_guidance: 'NoWarning',
        protection_strategy: 'ATRBased',
        target_strategy: 'ResistanceBased',
        confidence_assessment: 75,
        stop_loss_distance_pct: 0.015,
        cascade_risk_score: 30,
        environment_favorability: makeDanger(25),
        final_recommendation: 'Long setup actionable.',
        ...overrides,
    };
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
    return {
        score: 50,
        bias: 'BULLISH',
        confidence: 0.7,
        score_confidence: 0.8,
        entry_danger: makeDanger(31),
        expected_reward_risk_ratio: 2.5,
        trade_readiness: 'READY',
        contributing_indicators: [],
        ...overrides,
    };
}

function makeOpportunity(overrides: Partial<OpportunityMatrix> = {}): OpportunityMatrix {
    return {
        symbol: 'BTC-USDT',
        primary_opportunity: 'TrendContinuation',
        opportunity_score: 80,
        setup_quality: 'STRONG',
        profiles: [
            {
                opportunity_type: 'TrendContinuation',
                score: 80,
                preconditions_met: 3,
                preconditions_total: 4,
                notes: '',
                direction_family: 'TrendRiding',
                long_entry_zone: { low: 60000, high: 62000 },
                long_target_zone: { low: 65000, high: 68000 },
                long_invalidation_level: 59000,
                short_entry_zone: null,
                short_target_zone: null,
                short_invalidation_level: null,
                long_expected_rr_internal: 2.5,
                short_expected_rr_internal: null,
                trade_viability: 'Actionable',
            },
        ],
        forecast_confidence: 0,
        contributing_signals: [],
        invalidation_note: '',
        entry_zone: { low: 60000, high: 62000 },
        target_zone: { low: 65000, high: 68000 },
        invalidation_level: 59000,
        long_entry_zone: { low: 60000, high: 62000 },
        long_target_zone: { low: 65000, high: 68000 },
        long_invalidation_level: 59000,
        short_entry_zone: { low: 0, high: 0 },
        short_target_zone: { low: 0, high: 0 },
        short_invalidation_level: 0,
        long_expected_rr_internal: 2.5,
        short_expected_rr_internal: 2.5,
        time_horizon: 'SWING',
        confluent_entry_levels: [],
        confluent_target_levels: [],
        confluent_invalidation_levels: [],
        ...overrides,
    } as OpportunityMatrix;
}

function makeAnalysis(overrides: Partial<AnalysisMatrix> = {}): AnalysisMatrix {
    return {
        symbol: 'BTC-USDT',
        bias: 'Bullish',
        confidence: 0.7,
        state_confidence: 0.7,
        market_regime: 'ACCUMULATION',
        trend_assessment: 'Developing',
        momentum_assessment: 'Increasing',
        structure_assessment: 'Healthy',
        volatility_assessment: 'Normal',
        volume_assessment: 'Normal',
        opportunity_analysis: 'TrendContinuation',
        market_quality: 'Good',
        market_quality_score: 70,
        market_phase: 'MARKUP',
        market_interpretation: '',
        rationale: '',
        supporting_signals: [],
        contradicting_signals: [],
        timeframes_considered: 4,
        ...overrides,
    } as AnalysisMatrix;
}

function makeRiskMatrix(score = 40): RiskMatrix {
    return {
        symbol: 'BTC-USDT',
        market_risk: makeDanger(score),
        volatility_risk: makeDanger(score),
        execution_liquidity_risk: makeDanger(score, { confidence: 80 }),
        structure_risk: makeDanger(score),
        momentum_risk: makeDanger(score),
        signal_risk: makeDanger(score),
        execution_risk: makeDanger(score),
        cascade_risk: makeDanger(score),
        overall_risk: makeDanger(score),
    };
}

function makeInstance(symbol: string, overrides: Partial<InstanceState> = {}): InstanceState {
    return {
        symbol,
        exchange: 'Hyperliquid',
        isConnected: true,
        microTerm: { priceText: '63505', latestSnapshot: null } as any,
        fastTerm: {} as any,
        slowTerm: {} as any,
        macroTerm: {} as any,
        historyLatestClose: '0',
        currentView: 'terminal',
        alignment: null,
        analysis: makeAnalysis(),
        risk: makeRiskMatrix(40),
        advisory: makeAdvisory(),
        decisionContext: makeDecisionContext(),
        opportunity: makeOpportunity(),
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
        ...overrides,
    };
}

function seedPair(symbol: string, overrides: Partial<InstanceState> = {}) {
    const app = useAppStore();
    const key = `${symbol}-USDT`;
    if (!app.instancesMap[key]) app.initInstance(symbol);
    const entry = app.instancesMap[key];
    entry.instanceId = entry.instanceId || `inst_test_${symbol}`;
    Object.assign(entry, makeInstance(symbol, overrides));
    return entry;
}

beforeEach(() => {
    const app = useAppStore();
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
    app.overviewMatrix = null;
});

afterEach(() => {
    cleanup();
});

describe('GeneralDashboard — empty state', () => {
    it('renders the placeholder when no instances are configured', () => {
        const { container } = render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('Market Overview')).toBeTruthy();
        expect(screen.getByText(/Add workspaces/i)).toBeTruthy();
        // The hero is only shown when there are instances.
        expect(container.querySelector('[class*="hero"]')).toBeNull();
    });
});

describe('GeneralDashboard — hero states', () => {
    it('renders TRADE when at least one Actionable + READY setup exists', () => {
        seedPair('BTC', {
            decisionContext: makeDecisionContext({ trade_readiness: 'READY' }),
            opportunity: makeOpportunity({
                profiles: [{
                    opportunity_type: 'TrendContinuation',
                    score: 80,
                    preconditions_met: 3,
                    preconditions_total: 4,
                    notes: '',
                    direction_family: 'TrendRiding',
                    long_entry_zone: null,
                    long_target_zone: null,
                    long_invalidation_level: null,
                    short_entry_zone: null,
                    short_target_zone: null,
                    short_invalidation_level: null,
                    long_expected_rr_internal: 2.5,
                    short_expected_rr_internal: null,
                    trade_viability: 'Actionable',
                }],
            }),
        });
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('TRADE')).toBeTruthy();
        expect(screen.getByText(/actionable setup/i)).toBeTruthy();
    });

    it('renders WAIT when only DirectionalNeutral setups exist', () => {
        seedPair('BTC', {
            decisionContext: makeDecisionContext({ trade_readiness: 'FORMING' }),
            opportunity: makeOpportunity({
                profiles: [{
                    opportunity_type: 'MeanReversion',
                    score: 50,
                    preconditions_met: 2,
                    preconditions_total: 4,
                    notes: '',
                    direction_family: 'Neutral',
                    long_entry_zone: null,
                    long_target_zone: null,
                    long_invalidation_level: null,
                    short_entry_zone: null,
                    short_target_zone: null,
                    short_invalidation_level: null,
                    long_expected_rr_internal: null,
                    short_expected_rr_internal: null,
                    trade_viability: 'DirectionalNeutral',
                }],
            }),
        });
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('WAIT')).toBeTruthy();
    });

    it('renders STAND ASIDE when no qualifying profile exists', () => {
        seedPair('BTC', {
            opportunity: makeOpportunity({ profiles: [] }),
        });
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('STAND ASIDE')).toBeTruthy();
    });
});

describe('GeneralDashboard — asset rankings table', () => {
    it('renders 9 columns (Symbol, Price, Bias, Signal, Direction, R:R, Score, Confidence, Risk, Updated)', () => {
        seedPair('BTC');
        render(GeneralDashboard, { props: { wssMap: {} } });
        // Verify each column header is present (use getAllByText for
        // labels that may appear in multiple cards, e.g. "Direction"
        // appears in the Trade Opportunities card AND the table).
        expect(screen.getAllByText('Symbol').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Price').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Bias').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Signal').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Direction').length).toBeGreaterThan(0);
        expect(screen.getAllByText('R:R').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Score').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Confidence').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Risk').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Updated').length).toBeGreaterThan(0);
    });

    it('renders one row per pair', () => {
        seedPair('BTC');
        seedPair('ETH');
        render(GeneralDashboard, { props: { wssMap: {} } });
        // `createInstanceState` stores the bare symbol (e.g. 'BTC') in
        // `inst.symbol`, so the table renders 'BTC' and 'ETH' (the
        // `-USDT` suffix is the pairKey, not the display symbol).
        expect(screen.getAllByText('BTC').length).toBeGreaterThan(0);
        expect(screen.getAllByText('ETH').length).toBeGreaterThan(0);
    });

    it('renders BUY signal for LONG directional_guidance', () => {
        seedPair('BTC', {
            advisory: makeAdvisory({ directional_guidance: 'Long' }),
        });
        render(GeneralDashboard, { props: { wssMap: {} } });
        const cells = screen.getAllByText('BUY');
        expect(cells.length).toBeGreaterThan(0);
    });

    it('renders SELL signal for SHORT directional_guidance', () => {
        seedPair('BTC', {
            advisory: makeAdvisory({ directional_guidance: 'StrongShort' }),
        });
        render(GeneralDashboard, { props: { wssMap: {} } });
        const cells = screen.getAllByText('SELL');
        expect(cells.length).toBeGreaterThan(0);
    });

    it('renders WAIT signal for Neutral directional_guidance', () => {
        seedPair('BTC', {
            advisory: makeAdvisory({ directional_guidance: 'Neutral' }),
        });
        render(GeneralDashboard, { props: { wssMap: {} } });
        const cells = screen.getAllByText('WAIT');
        expect(cells.length).toBeGreaterThan(0);
    });
});

describe('GeneralDashboard — risk distribution', () => {
    it('renders the Risk Distribution card', () => {
        seedPair('BTC');
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('RISK DISTRIBUTION')).toBeTruthy();
        expect(screen.getByText('Low')).toBeTruthy();
        expect(screen.getByText('Moderate')).toBeTruthy();
        expect(screen.getByText('High')).toBeTruthy();
    });

    it('uses OverviewMatrix.risk_distribution when present (L7 source)', () => {
        const app = useAppStore();
        app.overviewMatrix = {
            global_market_bias: 'Bullish',
            market_breadth: 'Positive',
            low_coverage: false,
            breadth_pct: 50,
            regime_distribution: {},
            opportunity_distribution: {},
            risk_distribution: {
                low_pct: 60,
                moderate_pct: 30,
                high_pct: 10,
                risk_environment: 'LOW_RISK',
            },
            asset_ranking: [],
            market_synchronization: 'Synchronized',
            market_health: 'Healthy',
            global_summary: '',
            instance_count: 1,
            active_symbols: ['BTC-USDT'],
        } as OverviewMatrix;
        seedPair('BTC');
        render(GeneralDashboard, { props: { wssMap: {} } });
        // The card should show "LOW RISK" environment from L7.
        expect(screen.getByText(/LOW RISK/i)).toBeTruthy();
    });
});

describe('GeneralDashboard — UTC clock badge', () => {
    it('renders the UTC clock badge next to the title', () => {
        seedPair('BTC');
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('UTC')).toBeTruthy();
    });
});

describe('GeneralDashboard — header KPI strip', () => {
    it('renders the 6 KPI tiles', () => {
        seedPair('BTC');
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('VALID TRADES')).toBeTruthy();
        expect(screen.getByText('BEST OPPORTUNITY')).toBeTruthy();
        expect(screen.getByText('AVG R:R')).toBeTruthy();
        expect(screen.getByText('MARKET BIAS')).toBeTruthy();
        expect(screen.getByText('AVG RISK')).toBeTruthy();
        expect(screen.getByText('COVERAGE')).toBeTruthy();
    });
});

describe('GeneralDashboard — market health bars', () => {
    it('renders the 4 sub-dimension bars', () => {
        seedPair('BTC');
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getByText('MARKET HEALTH')).toBeTruthy();
        expect(screen.getByText('TREND STRENGTH')).toBeTruthy();
        expect(screen.getByText('LIQUIDITY')).toBeTruthy();
        expect(screen.getByText('VOLATILITY')).toBeTruthy();
        expect(screen.getByText('SIGNAL STABILITY')).toBeTruthy();
    });
});

describe('GeneralDashboard — scan status', () => {
    it('renders the scan-status strip with pair count', () => {
        seedPair('BTC');
        seedPair('ETH');
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getAllByText(/2.*pairs/).length).toBeGreaterThan(0);
        expect(screen.getAllByText('last scan').length).toBeGreaterThan(0);
        expect(screen.getAllByText('auto-refresh').length).toBeGreaterThan(0);
    });
});

describe('GeneralDashboard — direction distribution', () => {
    it('renders Long, Short, Neutral counts', () => {
        seedPair('BTC', { advisory: makeAdvisory({ directional_guidance: 'Long' }) });
        seedPair('ETH', { advisory: makeAdvisory({ directional_guidance: 'Short' }) });
        render(GeneralDashboard, { props: { wssMap: {} } });
        // Use getAllByText because LONG/SHORT/NEUTRAL also appear in
        // the Asset Rankings table's Direction column.
        expect(screen.getAllByText('LONG').length).toBeGreaterThan(0);
        expect(screen.getAllByText('SHORT').length).toBeGreaterThan(0);
        expect(screen.getAllByText('NEUTRAL').length).toBeGreaterThan(0);
    });
});

describe('GeneralDashboard — signal quality', () => {
    it('renders Strong/Moderate/Weak buckets', () => {
        seedPair('BTC', { advisory: makeAdvisory({ confidence_assessment: 80 }) });
        seedPair('ETH', { advisory: makeAdvisory({ confidence_assessment: 50 }) });
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getAllByText('STRONG').length).toBeGreaterThan(0);
        expect(screen.getAllByText('MODERATE').length).toBeGreaterThan(0);
    });
});

describe('GeneralDashboard — trade opportunities card', () => {
    it('renders the count of valid setups', () => {
        seedPair('BTC', {
            decisionContext: makeDecisionContext({ trade_readiness: 'READY' }),
        });
        render(GeneralDashboard, { props: { wssMap: {} } });
        expect(screen.getAllByText('TRADE OPPORTUNITIES').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Best Pair').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Direction').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Best R:R').length).toBeGreaterThan(0);
        expect(screen.getAllByText('Confidence').length).toBeGreaterThan(0);
    });
});
