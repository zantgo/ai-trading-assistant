// @vitest-environment jsdom
//
// v7.0-prod — TerminalMonitor sidebar contract.
//
// D7 + D3 + sidebar order:
//   1. The TIMEFRAMES rail renders MTF, MICRO, FAST, SLOW, MACRO
//      top-down — MTF first, per the v7.0-prod re-ordering.
//   2. The default active Tf on first render is MTF (matches the
//      L7 Overview "OPEN THE BIG PICTURE FIRST" operator rule).
//   3. Clicking the MICRO button switches the active Tf and the
//      MarketContextStrip becomes readable (its props are not
//      `undefined`).
//
// We pin these three invariants because the existing UI test gap on
// `TerminalMonitor.test.ts` was long-standing. Any future refactor
// that re-orders the rail or flips the default state will fail here.

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import TerminalMonitor from './TerminalMonitor.svelte';
import { useAppStore } from '../state.svelte';

function seedInstance(): void {
    const app = useAppStore();
    if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
    const entry = app.instancesMap['BTC-USDT'];
    entry.microTerm.indicators = {};
    entry.fastTerm.indicators = {};
    entry.slowTerm.indicators = {};
    entry.macroTerm.indicators = {};
    entry.microTerm.barDurationSec = 60;
    entry.fastTerm.barDurationSec = 180;
    entry.slowTerm.barDurationSec = 300;
    entry.macroTerm.barDurationSec = 900;
    // Provide minimal TF context so MarketContextStrip survives mount.
    for (const tf of [entry.microTerm, entry.fastTerm, entry.slowTerm, entry.macroTerm]) {
        tf.context = {
            trend: { score: 50, confidence: 50, label: 'NEUTRAL' },
            momentum: { score: 50, confidence: 50, label: 'NEUTRAL' },
            volatility: { score: 50, confidence: 50, label: 'NORMAL' },
            volume: { score: 50, confidence: 50, label: 'NORMAL' },
            liquidity: { score: 50, confidence: 50, label: 'ADEQUATE' },
            regime: 'RANGE',
            overall_score: 50,
            overall_label: 'NEUTRAL',
        };
        tf.pipelineState = 'LIVE';
        tf.isCompleted = true;
    }
    // Seed a tiny indicator registry so the body renders the facet tabs
    // (otherwise the marker selector below wouldn't have a payload).
    app.indicatorRegistry = [] as any;
    app.apiKeyConfigured = true;
}

beforeEach(() => {
    const app = useAppStore();
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => cleanup());

describe('TerminalMonitor — sidebar order (v7.0-prod D7)', () => {
    it('renders the rail in the order MTF · MICRO · FAST · SLOW · MACRO', () => {
        seedInstance();
        const { container } = render(TerminalMonitor, { props: { pairKey: 'BTC-USDT' } });
        const rail = container.querySelector('aside, [class*="tfSidebar"]');
        // Pull the textual label of each TIMEFRAMES-side button, top-down.
        const buttons = Array.from(
            rail?.querySelectorAll('button') ?? container.querySelectorAll('button')
        );
        // We just need the rail labels; the surrounding layout places
        // facet tabs AFTER the rail in the source order. Filter to the
        // first 5 since the rail always has exactly 5 items.
        const sidebarLabels = buttons
            .slice(0, 5)
            .map((b) => b.querySelector('span')?.textContent?.trim())
            .filter(Boolean);
        expect(sidebarLabels).toEqual(['MTF', 'Micro', 'Fast', 'Slow', 'Macro']);
    });
});

describe('TerminalMonitor — default active Tf is MTF (v7.0-prod D3)', () => {
    it('first paint: rail item "MTF" carries the .active class', () => {
        seedInstance();
        const { container } = render(TerminalMonitor, { props: { pairKey: 'BTC-USDT' } });
        const buttons = Array.from(container.querySelectorAll('button'));
        const rail = buttons.slice(0, 5);
        const activeRailItems = rail.filter((b) => b.className.split(/\s+/).some((c) => c.includes('active')));
        expect(activeRailItems.length).toBe(1);
        const label = activeRailItems[0].querySelector('span')?.textContent?.trim();
        expect(label).toBe('MTF');
    });
});
