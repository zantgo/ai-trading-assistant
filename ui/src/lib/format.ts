// format — single source of truth for dashboard number/currency formatting.
//
// Adopted by TAE / PME / PAE / DIE engine dashboards so every surface
// renders numbers identically (previously each dashboard rolled its own
// fmtUsd / fmtPnl / fmtPct / fmtNum variants, which drifted apart).
//
// Contract: every formatter returns a display string that is safe to
// render verbatim; null / undefined / non-finite inputs collapse to the
// em-dash placeholder "—" (the dashboard empty-state token).

export const DASH = '—';

function isFiniteNumber(v: string | number | null | undefined): boolean {
    return v != null && v !== '' && isFinite(Number(v));
}

/** $1,234.56 — locale-grouped currency, 2 decimals. "—" when unset. */
export function fmtUsd(v: string | number | null | undefined): string {
    if (!isFiniteNumber(v)) return DASH;
    return Number(v).toLocaleString(undefined, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
    });
}

/** +$1,234.56 / -$1,234.56 — signed currency. "—" when unset. */
export function signedUsd(v: string | number | null | undefined): string {
    if (!isFiniteNumber(v)) return DASH;
    const n = Number(v);
    return (n > 0 ? '+' : '') + fmtUsd(n);
}

/** 12.34% — percent with `decimals` (default 2). "—" when unset. */
export function fmtPct(v: string | number | null | undefined, decimals = 2): string {
    if (!isFiniteNumber(v)) return DASH;
    return `${Number(v).toFixed(decimals)}%`;
}

/** 1234.56 — plain number with `decimals` (default 2). "—" when unset. */
export function fmtNum(v: string | number | null | undefined, decimals = 2): string {
    if (!isFiniteNumber(v)) return DASH;
    return Number(v).toFixed(decimals);
}

/** +6.43 / -2.10 — signed plain number. "—" when unset. */
export function fmtSigned(v: string | number | null | undefined, decimals = 2): string {
    if (!isFiniteNumber(v)) return DASH;
    const n = Number(v);
    return (n > 0 ? '+' : '') + fmtNum(n, decimals);
}

/** HH:MM:SS — clock time from epoch ms. "—" when unset. */
export function fmtTs(ts: number | null | undefined): string {
    if (ts == null || !isFinite(ts) || ts <= 0) return DASH;
    return new Date(ts).toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
    });
}

/** Human duration from seconds: "1h 05m", "42s". "—" when unset. */
export function fmtDuration(totalSeconds: number | null | undefined): string {
    if (totalSeconds == null || !isFinite(totalSeconds) || totalSeconds < 0) return DASH;
    const s = Math.round(totalSeconds);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (h > 0) return `${h}h ${m.toString().padStart(2, '0')}m`;
    if (m > 0) return `${m}m ${sec.toString().padStart(2, '0')}s`;
    return `${sec}s`;
}

/** Short clock-time label for tabular timestamps (uses fmtTs). */
export function fmtTimeOnly(ts: number | null | undefined): string {
    return fmtTs(ts);
}
