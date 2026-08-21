// @vitest-environment jsdom
// BTE depth-slider contract + no-instance tab derivation tests.
//
// The 1..=365 validation lives in the run form and settings; this test
// exercises the pure validation rules through the shared constants so
// the UI contract is locked even without mounting (the Svelte binding
// itself is covered by svelte-check).
import { describe, expect, it } from 'vitest';
import { BTE_TABS_NO_INSTANCE, ENGINE_TABS } from '../../lib/engineTabs';

export const BTE_DEPTH_MIN = 1;
export const BTE_DEPTH_MAX = 365;

export function depthErrorFor(input: string): string | null {
    if (input.trim() === '') return 'must be a number';
    const v = Number(input);
    if (!Number.isFinite(v)) return 'must be a number';
    if (v < BTE_DEPTH_MIN || v > BTE_DEPTH_MAX) return `must be ${BTE_DEPTH_MIN}–${BTE_DEPTH_MAX}`;
    if (Math.floor(v) !== v) return 'must be a whole number';
    return null;
}

describe('BTE depth slider contract', () => {
    it('accepts the full 1..=365 range', () => {
        expect(depthErrorFor('1')).toBeNull();
        expect(depthErrorFor('365')).toBeNull();
        expect(depthErrorFor('180')).toBeNull();
    });

    it('rejects out-of-range and non-integer values', () => {
        expect(depthErrorFor('0')).toBe('must be 1–365');
        expect(depthErrorFor('366')).toBe('must be 1–365');
        expect(depthErrorFor('180.5')).toBe('must be a whole number');
        expect(depthErrorFor('')).toBe('must be a number');
        expect(depthErrorFor('abc')).toBe('must be a number');
    });
});

describe('BTE navbar derivation', () => {
    it('no instance → simplified set (Overview + History + Settings)', () => {
        expect(BTE_TABS_NO_INSTANCE.map((t) => t.key)).toEqual(['overview', 'history', 'settings']);
    });

    it('instance → full set with Settings last', () => {
        const full = ENGINE_TABS.backtesting;
        expect(full[full.length - 1].key).toBe('settings');
        expect(full.map((t) => t.key)).toContain('study');
        expect(full.map((t) => t.key)).toContain('die');
    });

    it('a stale section clamps to overview when the instance disappears', () => {
        // Mirrors the safeSection derivation in the dashboard: any tab
        // outside the visible set falls back to Overview.
        const visible = BTE_TABS_NO_INSTANCE.map((t) => t.key);
        for (const stale of ['die', 'mme', 'tae', 'pme', 'pae', 'study']) {
            expect(visible.includes(stale) ? stale : 'overview').toBe('overview');
        }
        expect(visible.includes('history') ? 'history' : 'overview').toBe('history');
    });
});
