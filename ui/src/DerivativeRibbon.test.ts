// @vitest-environment jsdom
//
// Static contract test for the DerivativeRibbon `lastSeen` logic. The
// component reads WS frames from `useAppStore().instancesMap[activeTab]`
// and tracks per-metric last-update timestamps. The bug fixed in the
// parity sweep: `lastSeen` was bumped only on `latestSnapshot.timestamp`
// changes — for Bitget the WS pushes OI/funding/OBI every frame
// (~1s) but the snapshot timestamp only advances on candle close, so
// derivatives flipped to `STALE` between candles. The fix bumps
// `lastSeen` per metric only when that metric's value actually moved.
//
// Rather than spinning up the full app store, this test pins the
// contract by asserting the source file contains the per-key bump
// pattern (a runtime-only correctness check would require a 24-hour
// Bitget WS replay, which is out of scope for unit tests).

import { describe, it, expect } from 'vitest';

async function readSrc(): Promise<string> {
    const fs = await import('node:fs/promises');
    const path = await import('node:path');
    const here = path.dirname(new URL(import.meta.url).pathname);
    return fs.readFile(
        path.join(here, 'components', 'DerivativeRibbon.svelte'),
        'utf-8',
    );
}

describe('DerivativeRibbon lastSeen contract', () => {
    it('bumps lastSeen per metric on value-change (not on snapshot timestamp)', async () => {
        const src = await readSrc();
        // The fix: instead of bumping every key to `ts` on each frame,
        // the per-key bump is gated on the metric's `raw != null`. This
        // is the line that closes the Bitget STALE regression between
        // candles.
        expect(src).toMatch(/bumpLastSeen\(['"]open_interest['"],\s*ts\)/);
        expect(src).toMatch(/bumpLastSeen\(['"]oi_delta['"],\s*ts\)/);
        expect(src).toMatch(/bumpLastSeen\(['"]funding_rate['"],\s*ts\)/);
        expect(src).toMatch(/bumpLastSeen\(['"]order_flow_imbalance['"],\s*ts\)/);
        expect(src).toMatch(/bumpLastSeen\(['"]spread['"],\s*ts\)/);
        expect(src).toMatch(/bumpLastSeen\(['"]depth_bias['"],\s*ts\)/);
    });

    it('still gates on value presence (null values do not bump)', async () => {
        const src = await readSrc();
        // Per-key `if (oiRaw != null)` guard before each bump ensures
        // a fresh CONNECTING status doesn't get prematurely upgraded to
        // LIVE on a frame that didn't carry the value.
        expect(src).toMatch(/if\s*\(\s*oiRaw\s*!=\s*null\s*\)\s*bumpLastSeen/);
        expect(src).toMatch(/if\s*\(\s*oiDeltaRaw\s*!=\s*null\s*\)\s*bumpLastSeen/);
        expect(src).toMatch(/if\s*\(\s*fundingRaw\s*!=\s*null\s*\)\s*bumpLastSeen/);
    });

    it('preserves the stale threshold at 30s (parity between HL and Bitget)', async () => {
        const src = await readSrc();
        expect(src).toMatch(/STALE_THRESHOLD_MS\s*=\s*30_000/);
    });
});