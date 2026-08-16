// Level-source display map. v6.10.20: the Opportunities panel and its
// export render the full source names (zero-abbreviation invariant) —
// "FIBONACCI" instead of "FIB", "SUPPORT AND RESISTANCE" instead of
// "SR", etc. Kept as a single map so screen and clipboard never disagree.

export const LEVEL_SOURCE_ABBREV: Record<string, string> = {
  FIBONACCI: 'FIBONACCI',
  VOLUME_PROFILE: 'VOLUME PROFILE',
  PIVOT_POINTS: 'PIVOT POINTS',
  SUPPORT_RESISTANCE: 'SUPPORT AND RESISTANCE',
  LIQUIDITY_CLUSTER: 'LIQUIDITY CLUSTER',
  ATR_FALLBACK: 'ATR',
};
