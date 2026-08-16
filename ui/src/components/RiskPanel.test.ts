// @vitest-environment jsdom
//
// RiskPanel — v6.10.9 consistency locks:
//   RK-A: the state pill renders functional states (the backend now derives
//         Critical/Elevated/Increasing/Improving — no longer always STABLE).
//   RK-D: the warmup sentinel matrix (RiskMatrix::empty signature) renders
//         as AWAITING, never as fabricated "Moderate" data.

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import RiskPanel from './RiskPanel.svelte';
import { useAppStore } from '../state.svelte';
import type { RiskMatrix, RiskDimension } from '../types';

function dim(score: number, level: string, state = 'Stable'): RiskDimension {
  return { score, level, state, confidence: 78, evidence: [] } as unknown as RiskDimension;
}

function makeRisk(state = 'Stable'): RiskMatrix {
  return {
    overall_risk: { score: 50, level: 'Moderate', state, confidence: 78, evidence: [] },
    market_risk: dim(60, 'Moderate'),
    volatility_risk: dim(55, 'Moderate'),
    execution_liquidity_risk: dim(30, 'Low'),
    structure_risk: dim(45, 'Moderate'),
    momentum_risk: dim(40, 'Low'),
    signal_risk: dim(25, 'Low'),
    execution_risk: dim(35, 'Low'),
    cascade_risk: dim(50, 'Moderate'),
  } as unknown as RiskMatrix;
}

function makeSentinelRisk(): RiskMatrix {
  const sentinel = { score: 50, level: 'Moderate', state: 'Stable', confidence: 50, evidence: [] };
  return {
    overall_risk: { ...sentinel },
    market_risk: { ...sentinel },
    volatility_risk: { ...sentinel },
    execution_liquidity_risk: { ...sentinel },
    structure_risk: { ...sentinel },
    momentum_risk: { ...sentinel },
    signal_risk: { ...sentinel },
    execution_risk: { ...sentinel },
    cascade_risk: { ...sentinel },
  } as unknown as RiskMatrix;
}

function seed(risk: RiskMatrix | null) {
  const app = useAppStore();
  if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
  const entry = app.instancesMap['BTC-USDT'];
  entry.risk = risk;
  return app;
}

beforeEach(() => {
  const app = useAppStore();
  for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => {
  cleanup();
});

describe('RiskPanel — functional risk state (RK-A)', () => {
  it('renders a RISING trend badge with the ↗ arrow (Scheme A, v6.10.19d C)', () => {
    const risk = makeRisk('Stable');
    // Trend states win over the level token — elevate one dim's trend.
    risk.market_risk = { ...dim(60, 'Moderate'), state: 'Increasing' };
    seed(risk);
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    expect(screen.getByText('RISING')).toBeTruthy();
    expect(screen.getAllByText('↗').length).toBeGreaterThan(0);
    // The header sublabel carries the prettified overall state.
    expect(screen.getByText('Stable')).toBeTruthy();
  });

  it('stable states fall back to the level token (Moderate → STEADY)', () => {
    seed(makeRisk('Stable'));
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    expect(screen.getAllByText('STEADY').length).toBeGreaterThan(0);
    expect(screen.queryByText('STABLE')).toBeNull();
  });
});

describe('RiskPanel — warmup sentinel gate (RK-D)', () => {
  it('sentinel matrix renders AWAITING cards, not fabricated "Moderate" data', () => {
    seed(makeSentinelRisk());
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    // All eight placeholder cards carry the AWAITING badge.
    expect(screen.getAllByText('AWAITING').length).toBe(8);
    // The hero must NOT show the sentinel's fabricated 50/100.
    expect(screen.queryByText('/ 100')).toBeNull();
    expect(screen.queryByText('50')).toBeNull();
    // The interpretation is the initializing copy, not "Moderate risk environment".
    expect(screen.getByText(/Risk synthesis is initializing/)).toBeTruthy();
  });

  it('a real matrix renders the hero risk bar and dimension cards', () => {
    seed(makeRisk('Stable'));
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    // The ring is gone — the hero leads with the risk progress bar
    // (score rendered as "50 / 100").
    expect(screen.getByText('50 / 100')).toBeTruthy();
    expect(screen.getAllByText('Moderate').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Risk Dimensions')).toBeTruthy();
  });
});

describe('RiskPanel — v6.11 execution-friction gauge (volatility_to_spread_ratio)', () => {
  it('renders the ATR-to-Spread field on the Execution Risk card', () => {
    const risk = makeRisk('Stable');
    risk.execution_risk = { ...dim(35, 'Low'), volatility_to_spread_ratio: 12.4 };
    seed(risk);
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    const field = screen.getByTitle(
      /ATR\(14\) ÷ top-of-book spread — execution-friction gauge/
    );
    expect(field.textContent?.trim()).toBe('12.4×');
    expect(screen.getByText('ATR-to-Spread')).toBeTruthy();
  });

  it('v6.10.21: volatility-to-spread value is tinted by the L5 band tiers', () => {
    // ≥ 10 favorable (green) · 3–10 neutral · 1.5–3 amber · < 1.5 red.
    const tint = (ratio: number): string => {
      const risk = makeRisk('Stable');
      risk.execution_risk = { ...dim(35, 'Low'), volatility_to_spread_ratio: ratio };
      cleanup();
      seed(risk);
      render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
      const field = screen.getByTitle(
        /ATR\(14\) ÷ top-of-book spread — execution-friction gauge/
      );
      return field.className;
    };
    expect(tint(12.4)).toContain('execVolGood');
    expect(tint(6.0)).toContain('execVolNeutral');
    expect(tint(2.0)).toContain('execVolWarn');
    expect(tint(1.2)).toContain('execVolBad');
  });

  it('hides the field when the ratio is absent (other dimensions)', () => {
    seed(makeRisk('Stable'));
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    expect(
      screen.queryByTitle(/ATR\(14\) ÷ top-of-book spread/)
    ).toBeNull();
  });
});
