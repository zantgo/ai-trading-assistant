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
  it('renders a non-Stable dimension state pill with the matching arrow', () => {
    const risk = makeRisk('Stable');
    // The state pill lives on the dimension cards — elevate one dim.
    risk.market_risk = { ...dim(60, 'Moderate'), state: 'Elevated' };
    seed(risk);
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    // The pill renders "↑ ELEVATED" (stateArrow elevated → up arrow).
    expect(screen.getByText('ELEVATED')).toBeTruthy();
    expect(screen.getAllByText('↑').length).toBeGreaterThan(0);
    // The header sublabel carries the prettified overall state.
    expect(screen.getByText('Stable')).toBeTruthy();
  });

  it('renders "→ STABLE" for stable states (unchanged, honest)', () => {
    seed(makeRisk('Stable'));
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    expect(screen.getAllByText('STABLE').length).toBeGreaterThan(0);
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

  it('a real matrix renders the hero ring and dimension cards', () => {
    seed(makeRisk('Stable'));
    render(RiskPanel, { props: { pairKey: 'BTC-USDT' } });
    expect(screen.getByText('/ 100')).toBeTruthy();
    expect(screen.getAllByText('Moderate').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Risk Dimensions')).toBeTruthy();
  });
});
