// Relative-time formatter for the Market Overview dashboard.
//
// Outputs a short, human-readable label for "how long ago" with a
// discrete ladder — operators read the dashboard at a glance and need
// stable formatting (no "1 minute ago" → "2 minutes ago" oscillation).
//
// The ladder:
//   < 5s   → "now"
//   < 60s  → "Ns ago"
//   < 60m  → "Nm ago"
//   < 24h  → "Nh ago"
//   else   → "—"
//
// `formatRelativeTime` is pure (no `Date.now()` side effect) so tests
// can stamp time deterministically. Callers pass `now` explicitly.

export type RelativeTime = {
    label: string;
    seconds: number;
};

export function formatRelativeTime(ms: number | null | undefined, now: number = Date.now()): RelativeTime {
    if (ms == null || !isFinite(ms) || ms <= 0) {
        return { label: '—', seconds: NaN };
    }
    const delta = Math.max(0, Math.floor((now - ms) / 1000));
    if (delta < 5) return { label: 'now', seconds: delta };
    if (delta < 60) return { label: `${delta}s ago`, seconds: delta };
    const mins = Math.floor(delta / 60);
    if (mins < 60) return { label: `${mins}m ago`, seconds: delta };
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return { label: `${hrs}h ago`, seconds: delta };
    return { label: '—', seconds: delta };
}
