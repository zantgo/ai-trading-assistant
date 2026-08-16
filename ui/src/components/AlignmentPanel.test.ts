// @vitest-environment jsdom
//
// AlignmentPanel — v6.10.10 consistency locks:
//   AL-1: the Score Calculation formula carries the ×100 factor so the
//         displayed equation balances (backend scales the blend by 100).
//   AL-2: STRONG_BULLISH/STRONG_BEARISH dimensions render with the
//         bullish/bearish colors (previously fell through to neutral).
//   AL-7: the NO_DATA sentinel renders "—%" + the awaiting interpretation,
//         never a fabricated "Conflict — time horizons diverging" verdict.

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import AlignmentPanel from './AlignmentPanel.svelte';
import styles from './AlignmentPanel.module.css';
import { useAppStore } from '../state.svelte';
import type { AlignmentMatrix, AlignmentDimension } from '../types';

function dim(score: number, state: string): AlignmentDimension {
  return { score, state, confidence: 70 } as unknown as AlignmentDimension;
}

function makeAlignment(overrides: Partial<AlignmentMatrix> = {}): AlignmentMatrix {
  return {
    symbol: 'BTC-USDT',
    timeframes_present: 4,
    dimensions: [
      dim(75, 'STRONG_BULLISH'),
      dim(60, 'BULLISH'),
      dim(-30, 'STRONG_BEARISH'),
      dim(45, 'NEUTRAL'),
      dim(70, 'STRONG_BULLISH'),
      dim(65, 'BULLISH'),
      dim(80, 'STRONG_BULLISH'),
      dim(70, 'BULLISH'),
      dim(55, 'NEUTRAL'),
      dim(65, 'BULLISH'),
    ],
    mtf_trend_alignment: 0.7,
    mtf_momentum_alignment: 0.6,
    mtf_volume_alignment: 0.5,
    mtf_volatility_alignment: 0.4,
    mtf_overall_score: 62,
    mtf_overall_label: 'STRONG_BULL_MTF',
    timeframe_alignments: [
      { timeframe: 'MICRO', timeframe_secs: 60, trend_score: 0.7, momentum_score: 0.6, overall_score: 1.0, regime: 'TRENDING_BULL', active_signals: 5, price: 63390 },
    ],
    signal_cross_tf_count: 2,
    trend_agreement_pct: 82,
    ...overrides,
  } as unknown as AlignmentMatrix;
}

function makeSentinelAlignment(): AlignmentMatrix {
  return {
    symbol: 'BTC-USDT',
    timeframes_present: 0,
    dimensions: Array.from({ length: 10 }, () => dim(0, 'NO_DATA')),
    mtf_trend_alignment: 0,
    mtf_momentum_alignment: 0,
    mtf_volume_alignment: 0,
    mtf_volatility_alignment: 0,
    mtf_overall_score: 0,
    mtf_overall_label: 'NO_DATA',
    timeframe_alignments: [],
    signal_cross_tf_count: 0,
    trend_agreement_pct: 0,
  } as unknown as AlignmentMatrix;
}

function seed(alignment: AlignmentMatrix | null) {
  const app = useAppStore();
  if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
  app.instancesMap['BTC-USDT'].alignment = alignment;
  return app;
}

beforeEach(() => {
  const app = useAppStore();
  for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => {
  cleanup();
});

describe('AlignmentPanel — score formula (AL-1)', () => {
  it('no longer renders the blend formula line (v6.10.19d A)', () => {
    seed(makeAlignment());
    render(AlignmentPanel, { props: { pairKey: 'BTC-USDT' } });
    // 0.5·0.7 + 0.3·0.6 + 0.1·0.5 + 0.1·0.4 = 0.62 → ×100 = 62.0 — the
    // formula line was erased from the panel; the weight chips remain.
    expect(screen.queryByText('(0.5 * (0.70) + 0.3 * (0.60) + 0.1 * (0.50) + 0.1 * (0.40)) × 100 = 62.0')).toBeNull();
    expect(screen.getByText('Score Calculation')).toBeTruthy();
  });
});

describe('AlignmentPanel — strong dimension colors (AL-2)', () => {
  it('STRONG_BULLISH and STRONG_BEARISH dimensions use the directional fill classes', () => {
    seed(makeAlignment());
    render(AlignmentPanel, { props: { pairKey: 'BTC-USDT' } });
    const cards = document.querySelectorAll(`.${styles.dimCard}`);
    expect(cards.length).toBe(10);
    // The three STRONG_BULLISH cards carry the bull fill class; the one
    // STRONG_BEARISH card carries the bear fill class.
    const bullFills = document.querySelectorAll(`.${styles.dimCardFill}.${styles.dimFillBull}`);
    const bearFills = document.querySelectorAll(`.${styles.dimCardFill}.${styles.dimFillBear}`);
    expect(bullFills.length).toBeGreaterThanOrEqual(3);
    expect(bearFills.length).toBeGreaterThanOrEqual(1);
    // The STRONG_BEARISH card's state pill carries the bear class.
    const bearState = document.querySelector(`.${styles.dimCard} .${styles.stateBearish}`);
    expect(bearState).toBeTruthy();
  });
});

describe('AlignmentPanel — NO_DATA sentinel gate (AL-7)', () => {
  it('sentinel renders "—%" and the awaiting interpretation, never a Conflict verdict', () => {
    seed(makeSentinelAlignment());
    render(AlignmentPanel, { props: { pairKey: 'BTC-USDT' } });
    expect(screen.getByText('—%')).toBeTruthy();
    expect(screen.getByText(/Awaiting alignment data/)).toBeTruthy();
    expect(screen.queryByText(/Conflict — time horizons diverging/)).toBeNull();
    expect(screen.queryByText(/Timeframes are in/)).toBeNull();
    expect(screen.queryByText(/TIMEFRAME CONFLICT/)).toBeNull();
  });

  it('a real alignment renders the consensus dial verdict and the axes grid', () => {
    seed(makeAlignment());
    render(AlignmentPanel, { props: { pairKey: 'BTC-USDT' } });
    // The dial center renders "82%"; the interpretation repeats it.
    expect(screen.getAllByText('82%').length).toBeGreaterThanOrEqual(1);
    // v6.10.20 (C): the verdict is a dial header + grey sub-label —
    // "Strong consensus — timeframes aligned" as one string is gone.
    expect(screen.getByText('Strong Consensus')).toBeTruthy();
    expect(screen.getByText('Timeframes are aligned.')).toBeTruthy();
    expect(screen.queryByText('Strong consensus — timeframes aligned')).toBeNull();
  });

  it('v6.10.20 (C): two-container hero — circular dial + CONSENSUS axes grid', () => {
    seed(makeAlignment());
    render(AlignmentPanel, { props: { pairKey: 'BTC-USDT' } });
    // The "Polarization" term is retired; the details container is
    // labeled with the unified word Consensus.
    expect(screen.queryByText('Polarization')).toBeNull();
    expect(screen.getByText('Consensus')).toBeTruthy();
    // Container 1: the circular gauge (SVG ring) with the centered pct.
    expect(document.querySelector(`.${styles.consensusDialSvg}`)).toBeTruthy();
    expect(document.querySelector(`.${styles.consensusDialFill}`)).toBeTruthy();
    // Container 2: exactly four axis cards in a 2×2 grid, labels on top.
    const cards = document.querySelectorAll(`.${styles.axisCard}`);
    expect(cards.length).toBe(4);
    const names = Array.from(document.querySelectorAll(`.${styles.axisName}`)).map((n) => n.textContent);
    expect(names).toEqual(['Trend', 'Momentum', 'Volume', 'Volatility']);
    const values = Array.from(document.querySelectorAll(`.${styles.axisValue}`)).map((n) => n.textContent);
    expect(values).toEqual(['+0.70', '+0.60', '+0.50', '+0.40']);
    // Trend +0.70 is a high-contrast bullish value (|v| > 0.2).
    expect(document.querySelector(`.${styles.axisValue}.${styles.axisStrongBull}`)).toBeTruthy();
  });

  it('v6.10.20 (C): sentinel dial renders a neutral ring and em-dash verdict', () => {
    seed(makeSentinelAlignment());
    render(AlignmentPanel, { props: { pairKey: 'BTC-USDT' } });
    expect(screen.getByText('—%')).toBeTruthy();
    expect(screen.getByText('—')).toBeTruthy();
  });
});
