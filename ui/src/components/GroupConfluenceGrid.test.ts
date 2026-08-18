// @vitest-environment jsdom
//
// GroupConfluenceGrid — L1 dimension chip contract (screen ≡ export):
// the 4 owning cards (Trend / Momentum / Volatility / Volume) render the
// matching `market_context` dimension score exactly once each, in the same
// sign-prefixed 2-decimal form the export's raw score implies. The
// liquidity dimension has NO group card (its single home is the Structural
// Anchors LIQUIDITY tile) and must never appear here. Null context renders
// no chips.

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import GroupConfluenceGrid from './GroupConfluenceGrid.svelte';
import type { IndicatorDto, IndicatorMeta, MarketContext } from '../types';

const registry: IndicatorMeta[] = [
    { key: 'ema_stack', display_name: 'EMA Ribbon', group: 'Trend', class: 'Lagging', render: 'Pane', directional: true, supports_divergence: false, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'number', value_source: 'indicator', color: '#22d3ee', guide_section: 'trend' },
    { key: 'rsi', display_name: 'RSI (14)', group: 'Momentum', class: 'Leading', render: 'Pane', directional: true, supports_divergence: true, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'number', value_source: 'indicator', color: '#a78bfa', guide_section: 'oscillators' },
    { key: 'atr', display_name: 'ATR (14)', group: 'Volatility', class: 'Hybrid', render: 'Pane', directional: false, supports_divergence: false, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'number', value_source: 'indicator', color: '#ef4444', guide_section: 'volatility' },
    { key: 'obv', display_name: 'OBV', group: 'Volume', class: 'Hybrid', render: 'Pane', directional: true, supports_divergence: true, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'number', value_source: 'indicator', color: '#fb923c', guide_section: 'volume' },
    { key: 'fibonacci', display_name: 'Fibonacci', group: 'Structure', class: 'Lagging', render: 'PriceOverlay', directional: true, supports_divergence: false, signal_types: [], default_weight: 1, default_enabled: true, config_params: [], value_format: 'price', value_source: 'indicator', color: '#60a5fa', guide_section: 'levels' },
];

const indicators: Record<string, IndicatorDto> = {};

const context: MarketContext = {
    regime: 'TRENDING',
    overall_score: 0.62,
    overall_label: 'STRONG_BULLISH',
    trend: { score: 0.7, confidence: 0.8, label: 'BULLISH' },
    momentum: { score: 0.5, confidence: 0.7, label: 'BULLISH' },
    volatility: { score: -0.2, confidence: 0.6, label: 'EXPANDING' },
    volume: { score: 0.3, confidence: 0.65, label: 'STRONG' },
    liquidity: { score: 0.4, confidence: 0.6, label: 'HEALTHY' },
};

afterEach(() => cleanup());

describe('GroupConfluenceGrid — L1 dimension chips (screen ≡ export)', () => {
    it('renders the matching dimension score on the 4 owning cards with tooltip', () => {
        render(GroupConfluenceGrid, { props: { registry, indicators, context } });
        // Same values the export's market_context block carries, formatted
        // sign + 2 decimals (the export consumer contract).
        expect(screen.getByText('+0.70')).toBeTruthy();
        expect(screen.getByText('+0.50')).toBeTruthy();
        expect(screen.getByText('-0.20')).toBeTruthy();
        expect(screen.getByText('+0.30')).toBeTruthy();
        // Liquidity has no group card — its single home is the LIQUIDITY tile.
        expect(screen.queryByText('+0.40')).toBeNull();
        // Tooltip = score · confidence% · label (mirrors export values).
        expect(screen.getByTitle('TREND +0.70 · 80% · BULLISH')).toBeTruthy();
        expect(screen.getByTitle('VOLATILITY -0.20 · 60% · EXPANDING')).toBeTruthy();
    });

    it('renders no chips when context is null', () => {
        render(GroupConfluenceGrid, { props: { registry, indicators, context: null } });
        expect(screen.queryByText('+0.70')).toBeNull();
        expect(screen.queryByText('+0.40')).toBeNull();
    });
});
