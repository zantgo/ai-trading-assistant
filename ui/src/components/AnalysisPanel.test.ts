// @vitest-environment jsdom
//
// AnalysisPanel — signal lean / signal-square consistency (v6.10.8):
//   AN-1: neutral signals render with the neutral (gray) square + flat
//         dash icon — never the bearish red styling and down arrow.
//   AN-2: the lean hero distinguishes "no data yet" (empty lists) from
//         "all timeframes neutral" (signals exist, no directional lean).
//   AN-3: a zero-opposing count renders "3:0", never "3:1".

import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { tick } from 'svelte';
import AnalysisPanel from './AnalysisPanel.svelte';
import { useAppStore } from '../state.svelte';
import type { AnalysisMatrix, IndicatorMap } from '../types';

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
        // v6.10.18 (I-7): neutral TFs (score 0, |score| ≤ 10) do not vote
        // — the hero reads "Neutral signals" (no lean) exactly as before.
        seed(makeAnalysis({
            supporting_signals: ['MICRO (neutral): score +0, RANGE regime, 0 signals'],
            contradicting_signals: ['FAST (neutral): score +0, RANGING regime, 0 signals'],
        }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('Neutral signals')).toBeTruthy();
        expect(screen.getByText('No directional lean across timeframes')).toBeTruthy();
        expect(screen.queryByText('No signals')).toBeNull();
    });

    it('empty signal lists keep the pre-warmup placeholder (AN-2)', () => {
        seed(makeAnalysis({ supporting_signals: [], contradicting_signals: [] }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('No signals')).toBeTruthy();
        expect(screen.getByText('Waiting for cross-TF consensus')).toBeTruthy();
    });

    it('zero opposing signals render "3:0", never "3:1" (AN-3)', () => {
        // v6.10.18 (I-7): the hero vote uses decisive scores (|score| > 10).
        seed(makeAnalysis({
            supporting_signals: [
                'MICRO (bullish): score +35, TRENDING_BULL regime, 3 signals',
                'FAST (bullish): score +25, TRENDING_BULL regime, 2 signals',
                'SLOW (bullish): score +15, TRENDING_BULL regime, 1 signal',
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
                'MICRO (bullish): score +35, TRENDING_BULL regime, 3 signals',
                'FAST (bullish): score +25, TRENDING_BULL regime, 2 signals',
            ],
            contradicting_signals: ['SLOW (bearish): score -30, RANGING regime, 1 signal'],
        }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('2.0:1 signal ratio')).toBeTruthy();
    });

    it('v6.11: Trend card renders the trend_stability_sharpe numeric badge', () => {
        seed(makeAnalysis({ trend_stability_sharpe: 3.85 }));
        render(AnalysisPanel, { props: {} });
        const badge = screen.getByText('3.85');
        expect(badge.textContent?.trim()).toBe('3.85');
        // Absent (null) → no badge.
        cleanup();
        seed(makeAnalysis({ trend_stability_sharpe: null }));
        render(AnalysisPanel, { props: {} });
        expect(screen.queryByText('3.85')).toBeNull();
    });

    it('v6.10.21: Trend badge tint mirrors the stability bands', () => {
        // ≥ +2 → strong-positive (green), > 0 → positive (light green),
        // ≤ −2 → strong-negative (red), else negative (light red).
        seed(makeAnalysis({ trend_stability_sharpe: 3.85 }));
        render(AnalysisPanel, { props: {} });
        let badge = screen.getByText('3.85');
        expect(badge.className).toContain('sharpeStrongPos');

        cleanup();
        seed(makeAnalysis({ trend_stability_sharpe: 0.8 }));
        render(AnalysisPanel, { props: {} });
        badge = screen.getByText('0.80');
        expect(badge.className).toContain('sharpePos');

        cleanup();
        seed(makeAnalysis({ trend_stability_sharpe: -5.7 }));
        render(AnalysisPanel, { props: {} });
        badge = screen.getByText('-5.70');
        expect(badge.className).toContain('sharpeStrongNeg');

        cleanup();
        seed(makeAnalysis({ trend_stability_sharpe: -0.73 }));
        render(AnalysisPanel, { props: {} });
        badge = screen.getByText('-0.73');
        expect(badge.className).toContain('sharpeNeg');
    });

    it('v6.12: all five cards render their 0-100 dimension-score badges (v6.13: rounded %)', () => {
        seed(makeAnalysis({
            trend_score: 62.35,
            momentum_score: 48.72,
            structure_score: 71.4,
            volatility_score: 78.15,
            volume_score: 82.6,
        }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('62%')).toBeTruthy();
        expect(screen.getByText('49%')).toBeTruthy();
        expect(screen.getByText('71%')).toBeTruthy();
        expect(screen.getByText('78%')).toBeTruthy();
        expect(screen.getByText('83%')).toBeTruthy();
    });

    it('v6.12: score badges are tinted by band heat (≥70 strong, ≥40 mid, <40 weak)', () => {
        seed(makeAnalysis({
            trend_score: 82.0,
            momentum_score: 55.0,
            structure_score: 25.0,
            volatility_score: 50.0,
            volume_score: 75.0,
        }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('82%').className).toContain('scoreStrong');
        expect(screen.getByText('55%').className).toContain('scoreMid');
        expect(screen.getByText('25%').className).toContain('scoreWeak');
    });

    it('v6.12: Trend card shows BOTH the dimension-score and the Sharpe badge (v6.13: with tooltips)', () => {
        seed(makeAnalysis({ trend_score: 62.35, trend_stability_sharpe: 20.0 }));
        render(AnalysisPanel, { props: {} });
        expect(screen.getByText('62%')).toBeTruthy();
        expect(screen.getByText('20.00')).toBeTruthy();
        // v6.13: hover tooltips qualify the numbers — the score badge
        // explains the cross-timeframe agreement semantics and the
        // Sharpe badge its statistical meaning.
        expect(screen.getByText('62%').getAttribute('title')).toContain('agreement across timeframes');
        expect(screen.queryByTitle(/Trend stability sharpe/i)).toBeTruthy();
    });

    it('v6.12: ▲ delta arrow appears when a score rises vs the previous frame', async () => {
        const app = seed(makeAnalysis({ trend_score: 60.0 }));
        render(AnalysisPanel, { props: {} });
        await tick();
        app.instancesMap['BTC-USDT'].analysis = makeAnalysis({ trend_score: 63.5 });
        await tick();
        await tick();
        const badge = screen.getByText(/64%/);
        expect(badge.textContent).toContain('▲');
        // Unchanged score → no arrow.
        app.instancesMap['BTC-USDT'].analysis = makeAnalysis({ trend_score: 63.5 });
        await tick();
        await tick();
        expect(screen.getByText(/64%/).textContent).not.toContain('▲');
    });

    it('v6.12: ▼ delta arrow appears when a score falls vs the previous frame', async () => {
        const app = seed(makeAnalysis({ momentum_score: 70.0 }));
        render(AnalysisPanel, { props: {} });
        await tick();
        app.instancesMap['BTC-USDT'].analysis = makeAnalysis({ momentum_score: 65.0 });
        await tick();
        await tick();
        const badge = screen.getByText(/65%/);
        expect(badge.textContent).toContain('▼');
    });

    it('v6.11: export qualitative_assessment carries the trend-stability Sharpe', async () => {
        const app = seed(makeAnalysis({ trend_stability_sharpe: 3.85 }));
        const tf = app.instancesMap['BTC-USDT'].microTerm;
        tf.indicators = {
            bbwp: { raw_value: 94.8, normalized: 0.948, state_label: 'NEUTRAL', values: null },
            adx: { raw_value: 40.06, normalized: 0.6, state_label: 'NEUTRAL', values: null },
        } as unknown as IndicatorMap;
        const writes: string[] = [];
        Object.defineProperty(navigator, 'clipboard', {
            value: { writeText: async (t: string) => { writes.push(t); return true; } },
            writable: true,
            configurable: true,
        });
        render(AnalysisPanel, { props: {} });
        const exportBtn = screen.getByTitle('Copy all Analysis data as JSON');
        await fireEvent.click(exportBtn);
        await tick();
        await new Promise((r) => setTimeout(r, 0));
        const payload = JSON.parse(writes[0]);
        expect(payload.qualitative_assessment.trend_stability_sharpe).toBeCloseTo(3.85, 2);
        expect(payload.qualitative_assessment.trend_stability_sharpe_display).toBe('3.85');
    });

    it('v6.10.19a (D1): export traceability carries the representative BBWP/ADX raw values', async () => {
        // Regression: the panel read the transient snapshot map with the
        // wrong key (`raw` vs the wire `raw_value`) — the representative
        // fields were null on every live export. The canonical source is
        // the term-level indicator map, exactly like the Metrics tab.
        const app = seed(makeAnalysis());
        const tf = app.instancesMap['BTC-USDT'].microTerm;
        tf.indicators = {
            bbwp: { raw_value: 94.8, normalized: 0.948, state_label: 'NEUTRAL', values: null },
            adx: { raw_value: 40.06, normalized: 0.6, state_label: 'NEUTRAL', values: null },
        } as unknown as IndicatorMap;

        const writes: string[] = [];
        Object.defineProperty(navigator, 'clipboard', {
            value: { writeText: async (t: string) => { writes.push(t); return true; } },
            writable: true,
            configurable: true,
        });
        render(AnalysisPanel, { props: {} });
        const exportBtn = screen.getByTitle('Copy all Analysis data as JSON');
        await fireEvent.click(exportBtn);
        await tick();
        await new Promise((r) => setTimeout(r, 0));
        expect(writes.length).toBe(1);
        const payload = JSON.parse(writes[0]);
        expect(payload.representative_bbwp).toBeCloseTo(94.8, 1);
        expect(payload.representative_adx).toBeCloseTo(40.06, 1);
    });

    it('v6.10.21: matrix-pinned representative BBWP/ADX win over the micro-map fallback', async () => {
        // The analysis mirror is per-slot last-writer-wins — the matrix's
        // own pinned representative fields are the exact inputs the
        // rationale quotes; the micro map must NOT override them.
        const app = seed(makeAnalysis({
            representative_bbwp: 53.0,
            representative_adx: 35.3,
        }));
        const tf = app.instancesMap['BTC-USDT'].microTerm;
        tf.indicators = {
            bbwp: { raw_value: 11.6, normalized: 0.116, state_label: 'NEUTRAL', values: null },
            adx: { raw_value: 27.48, normalized: 0.27, state_label: 'NEUTRAL', values: null },
        } as unknown as IndicatorMap;

        const writes: string[] = [];
        Object.defineProperty(navigator, 'clipboard', {
            value: { writeText: async (t: string) => { writes.push(t); return true; } },
            writable: true,
            configurable: true,
        });
        render(AnalysisPanel, { props: {} });
        const exportBtn = screen.getByTitle('Copy all Analysis data as JSON');
        await fireEvent.click(exportBtn);
        await tick();
        await new Promise((r) => setTimeout(r, 0));
        const payload = JSON.parse(writes[0]);
        expect(payload.representative_bbwp).toBeCloseTo(53.0, 1);
        expect(payload.representative_adx).toBeCloseTo(35.3, 1);
    });
});
