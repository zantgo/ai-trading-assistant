// studyCharts — SVG path helpers for the BTE Study Report.
//
// Mirrors the BacktestTab equity-path approach (pure math → SVG path
// strings, no chart library): equity curve, drawdown area, rolling
// win-rate line, and the trade PnL histogram.

export interface ChartBounds {
    minX: number;
    maxX: number;
    minY: number;
    maxY: number;
}

export function linePath(
    points: [number, number][],
    w: number,
    h: number,
    pad = 10,
): { path: string; bounds: ChartBounds } {
    if (!points || points.length < 2) {
        return { path: '', bounds: { minX: 0, maxX: 0, minY: 0, maxY: 0 } };
    }
    const xs = points.map((p) => p[0]);
    const ys = points.map((p) => p[1]);
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minY = Math.min(...ys);
    const maxY = Math.max(...ys);
    const spanX = maxX - minX || 1;
    const spanY = maxY - minY || 1;
    const path = points
        .map(([x, y], i) => {
            const px = pad + ((x - minX) / spanX) * (w - pad * 2);
            const py = h - pad - ((y - minY) / spanY) * (h - pad * 2);
            return (i === 0 ? 'M' : 'L') + px.toFixed(1) + ',' + py.toFixed(1);
        })
        .join(' ');
    return { path, bounds: { minX, maxY, maxX, minY } };
}

export function areaPath(
    points: [number, number][],
    w: number,
    h: number,
    baseline = 0,
): string {
    if (!points || points.length < 2) return '';
    const { path, bounds } = linePath(points, w, h);
    const xLast = ((points[points.length - 1][0] - bounds.minX) / (bounds.maxX - bounds.minX || 1)) * (w - 20) + 10;
    const yBase = h - 10 - ((baseline - bounds.minY) / (bounds.maxY - bounds.minY || 1)) * (h - 20);
    return `${path} L${xLast.toFixed(1)},${yBase.toFixed(1)} Z`;
}

/** Rolling win-rate over `window` trades → [tradeIndex, pct][] points. */
export function rollingWinRate(pnls: number[], window = 10): [number, number][] {
    const out: [number, number][] = [];
    for (let end = window; end <= pnls.length; end += 1) {
        const slice = pnls.slice(end - window, end);
        const wins = slice.filter((p) => p > 0).length;
        out.push([end, (wins / window) * 100]);
    }
    return out;
}

/** PnL histogram buckets (10 bins across the min/max range). */
export function pnlHistogram(pnls: number[]): { label: string; count: number; min: number }[] {
    if (pnls.length === 0) return [];
    const min = Math.min(...pnls);
    const max = Math.max(...pnls);
    const span = max - min || 1;
    const bins = 10;
    const counts = new Array(bins).fill(0);
    for (const p of pnls) {
        let idx = Math.floor(((p - min) / span) * bins);
        if (idx >= bins) idx = bins - 1;
        if (idx < 0) idx = 0;
        counts[idx]++;
    }
    return counts.map((count, i) => ({
        label: `${(min + (span * i) / bins).toFixed(1)}…${(min + (span * (i + 1)) / bins).toFixed(1)}`,
        count,
        min: min + (span * i) / bins,
    }));
}

/** Drawdown series from an equity curve → [ts, ddPct][] points. */
export function drawdownSeries(equity: [number, number][]): [number, number][] {
    const out: [number, number][] = [];
    let peak = Number.NEGATIVE_INFINITY;
    for (const [ts, eq] of equity) {
        if (eq > peak) peak = eq;
        const dd = peak > 0 ? ((peak - eq) / peak) * 100 : 0;
        out.push([ts, dd]);
    }
    return out;
}

export function fmtSpan(secs: number): string {
    const d = Math.floor(secs / 86400);
    const h = Math.floor((secs % 86400) / 3600);
    if (d > 0 && h > 0) return `${d}d ${h}h`;
    if (d > 0) return `${d}d`;
    if (h > 0) return `${h}h`;
    return `${Math.floor(secs / 60)}m`;
}
