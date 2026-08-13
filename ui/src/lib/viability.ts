// TradeViability normalization.
//
// The Rust enum `core_domain::analysis::TradeViability` is serialized with
// `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`, so the wire carries
// `"ACTIONABLE" | "DIRECTIONAL_NEUTRAL" | "GEOMETRY_INVERTED" | "NO_CLEAR"`.
// The frontend's badge checks and the panel's conditional tests compare to
// PascalCase tokens (`"Actionable"`, `"NoClear"`, …). For display parity
// (the badge text the panel renders vs. the badge text the export emits),
// normalize the wire value to PascalCase once at the boundary.

const TO_PASCAL: Record<string, string> = {
  ACTIONABLE: 'Actionable',
  DIRECTIONAL_NEUTRAL: 'DirectionalNeutral',
  GEOMETRY_INVERTED: 'GeometryInverted',
  NO_CLEAR: 'NoClear',
  // Pass-through for already-normalized or backbone values.
  Actionable: 'Actionable',
  DirectionalNeutral: 'DirectionalNeutral',
  GeometryInverted: 'GeometryInverted',
  NoClear: 'NoClear',
  // Legacy / corruption-safe fallbacks.
  actionable: 'Actionable',
  directional_neutral: 'DirectionalNeutral',
  geometry_inverted: 'GeometryInverted',
  no_clear: 'NoClear',
  undefined: 'NoClear',
  null: 'NoClear',
};

export function normalizeViability(raw: unknown): 'Actionable' | 'DirectionalNeutral' | 'GeometryInverted' | 'NoClear' {
  if (raw == null) return 'NoClear';
  const key = String(raw);
  return (TO_PASCAL[key] ?? 'NoClear') as 'Actionable' | 'DirectionalNeutral' | 'GeometryInverted' | 'NoClear';
}
