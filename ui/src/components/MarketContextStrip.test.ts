// @vitest-environment jsdom
//
// MarketContextStrip — v6.10.11 consistency locks:
//   M-1: the five L1 LOCAL synthesis dimension chips (trend / momentum /
//        volatility / volume / liquidity) render when expanded — the same
//        five dimensions the single-TF export carries.

import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import MarketContextStrip from './MarketContextStrip.svelte';

function makeContext() {
  return {
    regime: 'TRENDING_BULL',
    overall_score: 0.45,
    overall_label: 'WEAK_BULL',
    trend: { score: 0.4, confidence: 0.72, label: 'WEAK_BULL' },
    momentum: { score: 0.3, confidence: 0.6, label: 'WEAK_BULL' },
    volatility: { score: -0.2, confidence: 0.55, label: 'NORMAL' },
    volume: { score: 0.1, confidence: 0.5, label: 'NORMAL' },
    liquidity: { score: 0.05, confidence: 0.48, label: 'NEUTRAL' },
  } as any;
}

afterEach(() => cleanup());

describe('MarketContextStrip — five-dimension synthesis (M-1)', () => {
  it('renders the awaiting placeholder when there is no context', () => {
    render(MarketContextStrip, { props: { context: null } });
    expect(screen.getByText(/Awaiting completed snapshot/)).toBeTruthy();
  });

  it('renders the one-line header by default (regime + overall)', () => {
    render(MarketContextStrip, { props: { context: makeContext() } });
    expect(screen.getByText('TRENDING_BULL')).toBeTruthy();
    expect(screen.getByText('+0.45')).toBeTruthy();
    // The five dimension chips are hidden until expanded.
    expect(screen.queryByText('+0.40')).toBeNull();
  });

  it('expanding reveals the five dimension chips with score + confidence + label', async () => {
    render(MarketContextStrip, { props: { context: makeContext() } });
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByText('Trend')).toBeTruthy();
    expect(screen.getByText('Momentum')).toBeTruthy();
    expect(screen.getByText('Volatility')).toBeTruthy();
    expect(screen.getByText('Volume')).toBeTruthy();
    expect(screen.getByText('Liquidity')).toBeTruthy();
    // Scores (signed 2dp) + confidence mirrored from the export's
    // market_context block (Math.round(confidence * 100)%).
    expect(screen.getByText('+0.40')).toBeTruthy();
    expect(screen.getByText('-0.20')).toBeTruthy();
    expect(screen.getByText('72%')).toBeTruthy();
    expect(screen.getByText('48%')).toBeTruthy();
    // The label appears on both the overall header and the trend chip.
    expect(screen.getAllByText('WEAK_BULL').length).toBeGreaterThanOrEqual(2);
  });

  it('renders the signal count badge when provided', () => {
    render(MarketContextStrip, { props: { context: makeContext(), signalCount: 7 } });
    expect(screen.getByText('7 signals')).toBeTruthy();
  });
});
