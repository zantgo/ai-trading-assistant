// PriceChart helpers extracted for reuse
export function smcAgeLabel(ageBars: number | undefined, timeframeSec: number): string {
    if (ageBars == null || !isFinite(ageBars as number)) return "—";
    const secs = (ageBars as number) * (timeframeSec || 60);
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
    return `${Math.floor(secs / 86400)}d`;
}
