// @vitest-environment jsdom
//
// AnalysisPanel — signal lean / signal-square consistency (v6.10.8):
//   AN-1: neutral signals render with the neutral (gray) square + flat
//         dash icon — never the bearish red styling and down arrow.
//   AN-2: the lean hero distinguishes "no data yet" (empty lists) from
//         "all timeframes neutral" (signals exist, no directional lean).
//   AN-3: a zero-opposing count renders "3:0", never "3:1".

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import AnalysisPanel from './AnalysisPanel.svelte';
import { useAppStore } from '../state.svelte';
import type { AnalysisMatrix } from '../types';

function makeAnalysis(overrides: Partial<AnalysisMatrix> = {}): AnalysisMatrix {
    return {
        symbol: 'BTC-USDT',
        bias: 'Bullish',
        market_bias_score: 0.4,
        state_confidence: 0.6,
        confidence: 0.6,
        market_regime: 'TRENDING_BULL',
        trend_assessment: 'Healthy',
        momentum_assessment: 'Stable',
        structure_assessment: 'Healthy',
        volatility_assessment: 'Normal',
        volume_assessment: 'Normal',
        opportunity_analysis: 'TrendContinuation',
        market_quality: 'Good',
        market_quality_score: 70,
        market_phase: 'MARKUP',
        market_interpretation: 'Bullish market.',
        rationale: 'MTF overall score 30/100 → Bullish.',
        supporting_signals: ['MICRO (bullish): score +5, TRENDING_BULL regime, 3 signals'],
        contradicting_signals: [],
        timeframes_considered: 4,
        ...overrides,
    } as AnalysisMatrix;
}

function seed(analysis: AnalysisMatrix) {
    const app = useAppStore();
    app.activeTab = 'BTC-USDT';
    if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
    app.instancesMap['BTC-USDT'].analysis = analysis;
    return app;
}

beforeEach(() => {
    const app = useAppStore();
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => {
    cleanup();
});

describe('AnalysisPanel — signal lean (AN-1/2/3)', () => {
    it('renders a neutral signal with the neutral gray square + flat icon (AN-1)', () => {
        seed(makeAnalysis({
            supporting_signals: ['MICRO (neutral): score +0, RANGE regime, 0 signals'],
            contradicting_signals: [],
        }));
        render(AnalysisPanel, { props: {} });
        const square = screen.getByTitle('MICRO (neutral): score +0, RANGE regime, 0 signals');
        const svg = square.querySelector('svg');
        // Flat gray dash (stroke carried by the <svg> element) — NOT the
        // red down-arrow (line + polyline, red stroke).
        expect(svg?.getAttribute('stroke')).toBe('#94a3b8');
        expect(svg?.querySelectorAll('polyline').length).toBe(0);
    });

    it('still renders bull/bear squares with arrows for directional signals (AN-1)', () => {
        seed(makeAnalysis({
            supporting_signals: ['MICRO (bullish): score +5, TRENDING_BULL regime, 3 signals'],
            contradicting_signals: ['FAST (bearish): score -3, RANGING regime, 1 signal'],
        }));
        render(AnalysisPanel, { props: {} });
        const bull = screen.getByTitle('MICRO (bullish): score +5, TRENDING_BULL regime, 3 signals');
        expect(bull.querySelector('svg')?.getAttribute('stroke')).toBe('#22c55e');
        expect(bull.querySelector('svg')?.querySelector('polyline')).toBeTruthy();
        const bear = screen.getByTitle('FAST (bearish): score -3, RANGING regime, 1 signal');
        expect(bear.querySelector('svg')?.getAttribute('stroke')).toBe('#ef4444');
        expect(bear.querySelector('svg')?.querySelector('polyline')).toBeTruthy();
    });

    it('all-neutral signals render an honest neutral hero, not "No signals" (AN-2)', () => {
        seed(makeAnalysis({
            supporting_signals: ['MICRO (neutral): score +0, RANGE regime, 0 signals'],
            contradicting_signals: ['FAST (neutral): score +0, RANGING regime, 0 signals'],
        }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('Neutral signals')).toBeTruthy();
        expect(screen.getByText('No directional lean across timeframes')).toBeTruthy();
        expect(screen.queryByText('No signals')).toBeNull();
        expect(screen.getByText(/Neutral signals · no directional lean/)).toBeTruthy();
    });

    it('empty signal lists keep the pre-warmup placeholder (AN-2)', () => {
        seed(makeAnalysis({ supporting_signals: [], contradicting_signals: [] }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('No signals')).toBeTruthy();
        expect(screen.getByText('Waiting for cross-TF consensus')).toBeTruthy();
    });

    it('zero opposing signals render "3:0", never "3:1" (AN-3)', () => {
        seed(makeAnalysis({
            supporting_signals: [
                'MICRO (bullish): score +5, TRENDING_BULL regime, 3 signals',
                'FAST (bullish): score +2, TRENDING_BULL regime, 2 signals',
                'SLOW (bullish): score +1, TRENDING_BULL regime, 1 signal',
            ],
            contradicting_signals: [],
        }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('3:0 signal ratio')).toBeTruthy();
        expect(screen.queryByText(/3:1/)).toBeNull();
    });

    it('renders the doc-example 2.0:1 ratio when both sides have counts (AN-3)', () => {
        seed(makeAnalysis({
            supporting_signals: [
                'MICRO (bullish): score +5, TRENDING_BULL regime, 3 signals',
                'FAST (bullish): score +2, TRENDING_BULL regime, 2 signals',
            ],
            contradicting_signals: ['SLOW (bearish): score -3, RANGING regime, 1 signal'],
        }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('2.0:1 signal ratio')).toBeTruthy();
    });
});
