// @vitest-environment jsdom
//
// v7.0-prod — LiquidationHeatmapTierPicker contract.
//
// Picker invariants (D5/D6 — leverage ∈ [1, 100] integer-only):
//   • Initial value `[10]` (D5 default 10×, single chip).
//   • Add accepts integers in [1, 100], rejects everything else.
//   • Add rejects duplicates (silent no-op).
//   • Remove trims the list down by one.
//   • Row renders one chip per integer with an `× remove` control.
//   • Empty row surfaces the "No tiers selected" hint.

import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import LiquidationHeatmapTierPicker from './LiquidationHeatmapTierPicker.svelte';

afterEach(() => cleanup());

function mount(initial: number[]) {
    let current: number[] = initial;
    render(LiquidationHeatmapTierPicker, {
        props: {
            tiers: current,
            onChange: (next: number[]) => { current = next; },
            min: 1,
            max: 100,
        },
    });
    return {
        get current() { return current; },
    };
}

describe('LiquidationHeatmapTierPicker — invariants', () => {
    it('renders one chip per integer in the supplied set (default `[10]` → one chip)', () => {
        mount([10]);
        const chips = Array.from(document.querySelectorAll('button')).filter(
            (b) => b.getAttribute('aria-label')?.startsWith('Remove ')
        );
        expect(chips.length).toBe(1);
        expect(chips[0].getAttribute('aria-label')).toBe('Remove 10x tier');
        expect(chips[0].previousElementSibling?.textContent).toContain('10×');
    });

    it('empty input falls back to the operator hint', () => {
        mount([]);
        expect(document.body.textContent).toContain('No tiers selected');
    });

    it('ADD button invokes onChange with new tier appended (sorted)', () => {
        const api = mount([10]);
        const input = document.querySelector('input[type="number"]') as HTMLInputElement;
        const addBtn = Array.from(document.querySelectorAll('button')).find(
            (b) => b.textContent?.trim() === 'ADD'
        );
        input.value = '25';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        addBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        expect(api.current).toEqual([10, 25]);
    });

    it('rejects fractional inputs (e.g. "12.5")', () => {
        const api = mount([10]);
        const input = document.querySelector('input[type="number"]') as HTMLInputElement;
        const addBtn = Array.from(document.querySelectorAll('button')).find(
            (b) => b.textContent?.trim() === 'ADD'
        );
        input.value = '12.5';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        addBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        expect(api.current).toEqual([10]);
    });

    it('rejects out-of-range inputs (< 1 or > 100)', () => {
        for (const bad of ['0', '101', '-5', '500']) {
            const api = mount([10]);
            const input = document.querySelector('input[type="number"]') as HTMLInputElement;
            const addBtn = Array.from(document.querySelectorAll('button')).find(
                (b) => b.textContent?.trim() === 'ADD'
            );
            input.value = bad;
            input.dispatchEvent(new Event('input', { bubbles: true }));
            addBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
            expect(api.current).toEqual([10]);
        }
    });

    it('rejects duplicates', () => {
        const api = mount([10, 25]);
        const input = document.querySelector('input[type="number"]') as HTMLInputElement;
        const addBtn = Array.from(document.querySelectorAll('button')).find(
            (b) => b.textContent?.trim() === 'ADD'
        );
        input.value = '10';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        addBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        expect(api.current).toEqual([10, 25]);
    });

    it('pressing Enter on the input commits the integer', () => {
        const api = mount([10]);
        const input = document.querySelector('input[type="number"]') as HTMLInputElement;
        input.value = '75';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
        expect(api.current).toEqual([10, 75]);
    });

    it('chip remove × invokes onChange with the tier stripped', () => {
        const api = mount([10, 25, 50]);
        const removeBtn = Array.from(document.querySelectorAll('button')).find(
            (b) => b.getAttribute('aria-label') === 'Remove 25x tier'
        );
        removeBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        expect(api.current).toEqual([10, 50]);
    });
});
