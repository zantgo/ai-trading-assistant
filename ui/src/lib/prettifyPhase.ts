// Shared phase-token prettifier — used by the Analysis panel (Qualitative
// Assessment "Cycle Phase" card) and the analysis export builder so both
// surfaces render the identical string for the same wire token.
//
// "MARKUP" / "MARKDOWN" / "ACCUMULATION" / "DISTRIBUTION" / "UNKNOWN"
// are already all-caps tokens; the only transformation needed is
// underscore → space for any snake_case variant the wire may carry.

export function prettifyPhase(phase: string): string {
  return phase.replace(/_/g, ' ').replace(/([a-z])([A-Z])/g, '$1 $2').toUpperCase();
}

/**
 * Wrap the panel-rendered analysis keywords with `<strong>` markup so the
 * JSON `interpretation_display` mirrors the bolded screen rendering.
 * Both `interpretation` (raw) and `interpretation_display` (highlighted)
 * are exported for maximum compatibility.
 */
export function highlightKeywords(text: string): string {
  if (!text) return '\u2014';
  const keywords =
    /\b(TRANSITIONAL|DEVELOPING|WEAKENING|UNSTABLE|WEAK|STRONG|HEALTHY|EXHAUSTED|EXPANDING|COMPRESSED|NORMAL|EXTREME|INCREASING|STABLE|REVERSING|BROKEN|EXCEPTIONAL|BULLISH|BEARISH|NEUTRAL)\b/gi;
  return text.replace(keywords, '<strong>$1</strong>');
}
