import { describe, expect, it } from 'vitest';
import { computeOpportunityBars } from './opportunityBars';

describe('computeOpportunityBars', () => {
  it('returns 100% hold when opportunity matrix is missing', () => {
    expect(computeOpportunityBars(null)).toEqual({ bullish: 0, bearish: 0, hold: 100 });
  });

  it('returns 100% hold when both directional R:R values are zero', () => {
    const bars = computeOpportunityBars({
      long_expected_rr_internal: 0,
      short_expected_rr_internal: 0,
      opportunity_score: 50,
    } as unknown as any);

    expect(bars).toEqual({ bullish: 0, bearish: 0, hold: 100 });
  });

  it('normalizes long/short conviction and caps directional conviction by opportunity_score', () => {
    const bars = computeOpportunityBars({
      long_expected_rr_internal: 0.05,
      short_expected_rr_internal: 1.0,
      opportunity_score: 64,
    } as unknown as any);

    expect(bars.bullish).toBeGreaterThanOrEqual(3);
    expect(bars.bullish).toBeLessThanOrEqual(5);
    expect(bars.bearish).toBeGreaterThanOrEqual(55);
    expect(bars.bearish).toBeLessThanOrEqual(65);
    expect(bars.hold).toBeGreaterThanOrEqual(30);
    expect(bars.hold).toBeLessThanOrEqual(40);
    expect(bars.bullish + bars.bearish + bars.hold).toBe(100);
  });

  it('returns a visible hold portion for moderate setups', () => {
    const bars = computeOpportunityBars({
      long_expected_rr_internal: 1.5,
      short_expected_rr_internal: 0,
      opportunity_score: 78,
    } as unknown as any);

    expect(bars.bullish).toBeGreaterThan(0);
    expect(bars.bearish).toBeGreaterThanOrEqual(0);
    expect(bars.hold).toBeGreaterThan(0);
    expect(bars.bullish + bars.bearish + bars.hold).toBe(100);
  });
});
