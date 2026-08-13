// Regression tests for the v7.0-audit Risk tab export.

import { describe, it, expect } from 'vitest';
import { buildRiskTabExport } from './riskTab';
import type { LayerHeaderSpec } from '../layerHeader';
import type { RiskMatrix, RiskDimension } from '../../types';

const headerSpec: LayerHeaderSpec = {
  layerNumber: 5,
  layerName: 'Risk',
  badge: { label: 'Moderate', color: '#f59e0b', background: 'rgba(245,158,11,0.08)', state: 'valid' },
  meta: [],
  status: 'live',
};

function dim(score: number, level: string, state = 'STABLE'): RiskDimension {
  return { score, level, state, confidence: 78, evidence: [] } as unknown as RiskDimension;
}

function makeRisk(): RiskMatrix {
  return {
    overall_risk: { score: 50, level: 'Moderate', state: 'Stable', confidence: 78, evidence: [] },
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

describe('buildRiskTabExport', () => {
  it('emits structured headline_parts (counts separate from words)', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: makeRisk(),
      flow: null,
      cluster: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.headline_parts).not.toBeNull();
    expect(p.headline_parts.very_low_count).toBe(0);
    expect(p.headline_parts.low_count).toBe(4);
    expect(p.headline_parts.moderate_count).toBe(4);
    expect(p.headline_parts.overall_level).toBe('Moderate');
    expect(p.interpretation_headline).toContain('4 moderate');
  });

  it('interpretation_full mirrors the screen paragraph', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: makeRisk(),
      flow: null,
      cluster: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.interpretation_full).toContain('<strong>Moderate risk environment.</strong>');
    expect(p.interpretation_full).toContain('at 78% confidence');
  });

  it('summary_counts carry labels with counts', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: makeRisk(),
      flow: null,
      cluster: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.summary_counts.moderate).toEqual({ label: 'Moderate', count: 4 });
    expect(p.summary_counts.low).toEqual({ label: 'Low', count: 4 });
  });

  it('dimension scores and confidence are integer-rounded; state_display has arrow', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: makeRisk(),
      flow: null,
      cluster: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const market = p.dimensions.find((d: { key: string }) => d.key === 'market_risk');
    expect(market.score).toBe(60);
    expect(market.confidence).toBe(78);
    expect(market.state_display).toBe('→ STABLE');
  });

  it('no risk → hero null, awaiting placeholder dims, init interpretation, valid meta identity', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: null,
      flow: null,
      cluster: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.hero).toBeNull();
    // 8 awaiting placeholder rows mirror the screen's AWAITING cards
    // (name + weight + badge + paragraph).
    expect(p.dimensions).toHaveLength(8);
    expect(p.dimensions[0]).toMatchObject({
      name: 'Market Risk',
      weight_pct: 14,
      awaiting: true,
      awaiting_badge: 'AWAITING',
      not_active: false,
    });
    expect(p.dimensions[2].name).toBe('Exec Liquidity Risk');
    expect(p.dimensions.every((d: { awaiting: boolean }) => d.awaiting)).toBe(true);
    expect(p.interpretation_full).toContain('Risk synthesis is initializing');
    expect(p.meta.pair).toBe('BTC-USDT');
  });

  it('emits disclosure (8 weight chips + note) and hero hint', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: makeRisk(),
      flow: null,
      cluster: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.disclosure).toBeDefined();
    expect(p.disclosure.weights).toHaveLength(8);
    expect(p.disclosure.weights[0]).toHaveProperty('label');
    expect(p.disclosure.weights[0]).toHaveProperty('pct');
    expect(p.disclosure.note).toContain('weighted sum of the 8 dimension scores');
    expect(p.hero).toBeDefined();
    expect(p.hero.hint).toContain('Lower is safer');
    expect(p.awaiting_dimensions_text).toContain('Awaiting risk assessment');
  });
});