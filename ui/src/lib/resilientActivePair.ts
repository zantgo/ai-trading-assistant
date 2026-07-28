// Pure helper extracted from `App.svelte::resilientActivePair` so the
// 2-second "missing-pair" grace window can be unit-tested without
// mounting Svelte.
//
// The original implementation kept the cache as `$state(null)` so the
// `$derived.by` could update it on every re-run. Svelte 5 forbids
// mutating `$state` from inside a `$derived` — `sources.js:152-163`
// throws `state_unsafe_mutation` whenever `set()` is called from a
// reactive context whose `current_sources` doesn't include the source
// being mutated. Plain variables are NOT wrapped in `set()`, so a
// plain `let lastGoodPair = null` is safe inside a derived.
//
// This helper is the testable, side-effect-free version. Callers pass
// the cache by value (read it, get a new cache back) and decide
// whether to assign the new cache to their plain-variable holder.

import type { InstanceState } from '../types';

export interface PairCacheEntry {
    pair: InstanceState;
    pairKey: string;
    capturedAt: number;
}

/**
 * Compute the resilient active-pair for the top bar's price block.
 *
 * @param activePair   Current `app.instancesMap[app.selectedInstance]`,
 *                     or `undefined` when the key is briefly missing
 *                     (mid back-navigation, mid `applyConfigToStore`,
 *                     etc.).
 * @param selectedKey  Current `app.selectedInstance`, used only for
 *                     caching so a later `activePair === undefined`
 *                     branch knows which pair to restore.
 * @param cache        Last good pair + capture timestamp. `null` on
 *                     first render. **Plain (non-reactive) variable**
 *                     on the caller side.
 * @param graceMs      Window in ms during which the cached pair is
 *                     returned when `activePair` becomes `undefined`.
 * @param now          Monotonic clock to compare against
 *                     `cache.capturedAt`. Tests inject a fake clock.
 *
 * @returns `{ result, nextCache }`:
 *   - `result` — the pair to render in the top bar.
 *   - `nextCache` — what the caller should store in its plain variable
 *     for the next invocation (the same reference if no update is
 *     needed, so identity checks still work).
 */
export function applyResilientCache(
    activePair: InstanceState | undefined,
    selectedKey: string | null,
    cache: PairCacheEntry | null,
    graceMs: number,
    now: number,
): { result: InstanceState | undefined; nextCache: PairCacheEntry | null } {
    if (activePair) {
        const next: PairCacheEntry = {
            pair: activePair,
            pairKey: selectedKey!,
            capturedAt: now,
        };
        return { result: activePair, nextCache: next };
    }
    if (cache && now - cache.capturedAt < graceMs) {
        return { result: cache.pair, nextCache: cache };
    }
    return { result: undefined, nextCache: cache };
}

/**
 * Re-export of `Date.now()` for tests that want to inject a fake clock.
 * Kept as a named export so the helper stays pure (no globals, no
 * `import { Date }` games).
 */
export function resilientCacheNow(): number {
    return Date.now();
}