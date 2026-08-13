// Level-source abbreviation map. Mirrors `tradePlan.ts:381-382` so the
// export JSON produces the same abbreviated tokens the screen displays
// (e.g. "FIB" instead of "FIBONACCI", "VP" instead of "VOLUME_PROFILE").

export const LEVEL_SOURCE_ABBREV: Record<string, string> = {
  FIBONACCI: 'FIB',
  VOLUME_PROFILE: 'VP',
  PIVOT_POINTS: 'PP',
  SUPPORT_RESISTANCE: 'SR',
  LIQUIDITY_CLUSTER: 'LIQ',
  ATR_FALLBACK: 'ATR',
};