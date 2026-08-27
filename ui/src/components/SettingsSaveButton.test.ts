// SettingsSaveButton — the canonical settings save control contract.
// One state machine, enforced: never clickable unless dirty, never while
// saving, never right after a successful save.
import { describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import SettingsSaveButton from './SettingsSaveButton.svelte';

function renderAt(state: 'idle' | 'dirty' | 'saving' | 'saved' | 'error') {
    const onsave = vi.fn();
    const result = render(SettingsSaveButton, { props: { state, onsave } });
    return { onsave, result };
}

describe('SettingsSaveButton state machine', () => {
    afterEach(cleanup);

    it('idle: disabled, no status label', () => {
        renderAt('idle');
        const btn = screen.getByRole('button');
        expect(btn).toHaveProperty('disabled', true);
        expect(btn.textContent).toBe('SAVE');
        expect(screen.queryByText('Unsaved changes')).toBeNull();
    });

    it('dirty: enabled and labelled SAVE with "Unsaved changes"', () => {
        const { onsave } = renderAt('dirty');
        const btn = screen.getByRole('button');
        expect(btn).toHaveProperty('disabled', false);
        expect(btn.textContent).toBe('SAVE');
        expect(screen.getByText('Unsaved changes')).toBeTruthy();
        fireEvent.click(btn);
        expect(onsave).toHaveBeenCalledTimes(1);
    });

    it('saving: disabled, labelled SAVING…', () => {
        renderAt('saving');
        const btn = screen.getByRole('button');
        expect(btn).toHaveProperty('disabled', true);
        expect(btn.textContent).toBe('SAVING…');
        expect(screen.getByText('Saving…')).toBeTruthy();
    });

    it('saved: disabled, labelled SAVED, no longer clickable', () => {
        const { onsave } = renderAt('saved');
        const btn = screen.getByRole('button');
        expect(btn).toHaveProperty('disabled', true);
        expect(btn.textContent).toBe('SAVED');
        expect(screen.getByText('All changes saved')).toBeTruthy();
        fireEvent.click(btn);
        expect(onsave).not.toHaveBeenCalled();
    });

    it('error: re-enabled as SAVE and clickable again (retry)', () => {
        const { onsave } = renderAt('error');
        const btn = screen.getByRole('button');
        expect(btn).toHaveProperty('disabled', false);
        expect(btn.textContent).toBe('SAVE');
        fireEvent.click(btn);
        expect(onsave).toHaveBeenCalledTimes(1);
    });

    it('idle: clicking does nothing (disabled)', () => {
        const { onsave } = renderAt('idle');
        fireEvent.click(screen.getByRole('button'));
        expect(onsave).not.toHaveBeenCalled();
    });
});
