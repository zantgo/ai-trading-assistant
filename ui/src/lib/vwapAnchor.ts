/// Pick which VWAP anchor the blue dotted line should source from based on
/// the active TF duration. Daily VWAP degrades to a near-horizontal line on
/// long TFs (only 1–2 bars/day hit the UTC-midnight reset), while weekly
/// / monthly anchored VWAPs stack enough reset points in the visible window
/// to actually move the line. One blue VWAP line, different anchor per TF.
///
///   < 1 h        → daily         (`vwap` indicator)
///   1 h ≤ tf < 12 h → weekly    (anchored_vwap.values.weekly)
///   tf ≥ 12 h    → monthly     (anchored_vwap.values.monthly)
///
/// Returned structure names both the field on `FlatIndicatorHistory` (used
/// during the bootstrap `setData` call) and the iSub key (used during the
/// live WS coalescer tick) so the same lookup applies to either path.
///
/// Exported so `PriceChart.svelte` and the unit test share one source of truth.
export interface VwapPick {
    /// Field name on `FlatIndicatorHistory` to read the seeded array from.
    arrayKey: 'vwap' | 'avwap_weekly' | 'avwap_monthly';
    /// Sub-key passed to `iSub(m, indicator, sub)` on the live WS snapshot.
    iSubKey: 'vwap' | 'weekly' | 'monthly';
}

export function vwapPickKey(secs: number): VwapPick {
    if (secs >= 43200) return { arrayKey: 'avwap_monthly', iSubKey: 'monthly' };
    if (secs >= 3600) return { arrayKey: 'avwap_weekly', iSubKey: 'weekly' };
    return { arrayKey: 'vwap', iSubKey: 'vwap' };
}
