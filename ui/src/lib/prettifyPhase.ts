// Shared phase-token prettifier — used by the Analysis panel (Qualitative
// Assessment "Cycle Phase" card) and the analysis export builder so both
// surfaces render the identical string for the same wire token.
//
// The wire carries PascalCase tokens ("Markup", "Accumulation", ...);
// the panel renders them uppercase. The transformation uppercases
// after underscore→space + camel-split so both PascalCase wire values
// and legacy SCREAMING tokens resolve identically.

export function prettifyPhase(phase: string): string {
  return phase.replace(/_/g, ' ').replace(/([a-z])([A-Z])/g, '$1 $2').toUpperCase();
}

/**
 * Escape HTML metacharacters so backend-sourced free text can never inject
 * markup through the `{@html}` sink. Applied BEFORE keyword wrapping — the
 * `<strong>` wrappers are added after escaping, so they survive.
 */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

/**
 * Wrap the panel-rendered analysis keywords with `<strong>` markup so the
 * JSON `interpretation_display` mirrors the bolded screen rendering.
 * Both `interpretation` (raw) and `interpretation_display` (highlighted)
 * are exported for maximum compatibility.
 *
 * M9 (production audit): the input is HTML-escaped first — the previous
 * version passed backend strings straight into `{@html}`, a latent stored
 * XSS sink (only template-generated enum data prevented exploitation).
 */
export function highlightKeywords(text: string): string {
  if (!text) return '\u2014';
  const keywords =
    /\b(TRANSITIONAL|DEVELOPING|WEAKENING|UNSTABLE|WEAK|STRONG|HEALTHY|EXHAUSTED|EXPANDING|COMPRESSED|NORMAL|EXTREME|INCREASING|STABLE|REVERSING|BROKEN|EXCEPTIONAL|BULLISH|BEARISH|NEUTRAL)\b/gi;
  return escapeHtml(text).replace(keywords, '<strong>$1</strong>');
}
