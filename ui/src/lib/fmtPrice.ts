// Price formatter — mirrors `fmtPriceScale` from RecommendationPanel.svelte
// so any builder can produce the same `$X.XX` / `$X.XXXX` strings the
// panel renders.

export function fmtPriceScale(n: number, mp: number): string {
    if (!mp || mp <= 0 || !isFinite(n)) return n.toFixed(2);
    if (mp >= 1000) return n.toFixed(0);
    if (mp >= 1) return n.toFixed(2);
    if (mp >= 0.01) return n.toFixed(4);
    if (mp >= 0.0001) return n.toFixed(6);
    return n.toFixed(8);
}