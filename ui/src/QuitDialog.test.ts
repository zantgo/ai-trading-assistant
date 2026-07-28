// @vitest-environment jsdom
// Regression lock for the QuitDialog flow.
//
// The dialog used to leave the user staring at a "Shutting down..."
// spinner indefinitely if:
//   - the backend was slow / hung and `app.quitSession()` never
//     resolved, or
//   - the call returned `false` (backend rejected).
//
// The fix wraps the call in a 10 s `withTimeout(...)` and surfaces
// any failure as an inline `.quitError` block for `ERROR_DISMISS_MS`
// (3 s) before the dialog auto-closes. These tests lock the contract.

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import QuitDialog from './QuitDialog.svelte';
import { useAppStore } from './state.svelte';

beforeEach(() => {
    const app = useAppStore();
    // Wipe any state the previous test left.
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.useRealTimers();
});

describe('QuitDialog — happy path', () => {
    it('calls app.quitSession on confirm and closes immediately on success', async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        const app = useAppStore();
        const quitSpy = vi.spyOn(app, 'quitSession').mockResolvedValue(true);

        const onclose = vi.fn();
        render(QuitDialog, { props: { onclose } });

        const quitBtn = screen.getByRole('button', { name: /quit/i });
        await fireEvent.click(quitBtn);

        // Wait for the microtask to resolve so handleConfirm's `await` runs.
        await vi.advanceTimersByTimeAsync(0);

        expect(quitSpy).toHaveBeenCalledTimes(1);
        // On success we close immediately (no error toast) — the
        // 3 s setTimeout only fires when hasError is true.
        expect(onclose).toHaveBeenCalledTimes(1);
    });
});

describe('QuitDialog — failure paths surface an inline error', () => {
    /** Wait until `screen.findByRole('alert')` resolves, then return its
     *  text content. Equivalent to `toHaveTextContent` without pulling in
     *  `@testing-library/jest-dom` (which isn't in this project's deps). */
    async function alertText(): Promise<string> {
        const node = await screen.findByRole('alert');
        return node.textContent ?? '';
    }

    it('shows the error toast when quitSession returns false and auto-closes after ERROR_DISMISS_MS', async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        const app = useAppStore();

        let resolveFn: (v: boolean) => void = () => {};
        const inFlight = new Promise<boolean>((res) => { resolveFn = res; });
        vi.spyOn(app, 'quitSession').mockReturnValue(inFlight);

        const onclose = vi.fn();
        render(QuitDialog, { props: { onclose } });

        await fireEvent.click(screen.getByRole('button', { name: /quit/i }));

        // Resolve with `false` (the backend-rejected path).
        resolveFn(false);
        await vi.advanceTimersByTimeAsync(0);

        expect(await alertText()).toMatch(/Quit did not complete/i);
        expect(onclose).not.toHaveBeenCalled();

        // Advance the error-dismiss timer; onclose fires.
        await vi.advanceTimersByTimeAsync(3_000);
        expect(onclose).toHaveBeenCalledTimes(1);
    });

    it('shows the error toast and auto-closes when quitSession throws (network failure)', async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        const app = useAppStore();

        let rejectFn: (e: unknown) => void = () => {};
        const inFlight = new Promise<boolean>((_res, rej) => { rejectFn = rej; });
        vi.spyOn(app, 'quitSession').mockReturnValue(inFlight);

        const onclose = vi.fn();
        render(QuitDialog, { props: { onclose } });

        await fireEvent.click(screen.getByRole('button', { name: /quit/i }));

        rejectFn(new Error('network down'));
        await vi.advanceTimersByTimeAsync(0);

        expect(await alertText()).toMatch(/Quit failed: network down/i);
        expect(onclose).not.toHaveBeenCalled();

        await vi.advanceTimersByTimeAsync(3_000);
        expect(onclose).toHaveBeenCalledTimes(1);
    });

    it('times out at 10 s when the backend never responds and auto-closes', async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        const app = useAppStore();
        // A promise that never resolves — simulates a hung backend.
        vi.spyOn(app, 'quitSession').mockImplementation(
            () => new Promise<boolean>(() => {}),
        );

        const onclose = vi.fn();
        render(QuitDialog, { props: { onclose } });

        await fireEvent.click(screen.getByRole('button', { name: /quit/i }));

        // 5 s in: still showing "Shutting down...", no error yet, no close.
        await vi.advanceTimersByTimeAsync(5_000);
        expect(onclose).not.toHaveBeenCalled();
        expect(screen.queryByRole('alert')).toBeNull();

        // 11 s in: timeout fires, error toast appears, close is queued.
        await vi.advanceTimersByTimeAsync(6_000);
        expect(await alertText()).toMatch(/Backend did not respond/i);
        expect(onclose).not.toHaveBeenCalled();

        // 3 s after the error: close fires.
        await vi.advanceTimersByTimeAsync(3_000);
        expect(onclose).toHaveBeenCalledTimes(1);
    });

    it('re-enables both buttons after the in-flight call resolves (false)', async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        const app = useAppStore();

        // A deferred promise so we can observe the "in flight" state
        // before the resolution — a plain `mockResolvedValue(false)`
        // resolves synchronously and the buttons never observe the
        // `disabled={true}` state from the test's perspective.
        let resolveFn: (v: boolean) => void = () => {};
        const inFlight = new Promise<boolean>((res) => { resolveFn = res; });
        vi.spyOn(app, 'quitSession').mockReturnValue(inFlight);

        const onclose = vi.fn();
        render(QuitDialog, { props: { onclose } });

        const quitBtn = screen.getByRole('button', { name: /quit/i }) as HTMLButtonElement;
        const cancelBtn = screen.getByRole('button', { name: /cancel/i }) as HTMLButtonElement;
        await fireEvent.click(quitBtn);

        // While the call is in flight, both buttons are disabled.
        await waitFor(() => expect(quitBtn.disabled).toBe(true));
        expect(cancelBtn.disabled).toBe(true);

        // Resolve the call; the error path runs and the buttons must
        // re-enable so the user can click Cancel without waiting for
        // the 3 s auto-dismiss.
        resolveFn(false);
        await vi.advanceTimersByTimeAsync(0);
        await waitFor(() => expect(quitBtn.disabled).toBe(false));
        expect(cancelBtn.disabled).toBe(false);
    });
});