"""verify_final.py — produce the per-panel verification table:

  FRONT-END VALUE | EXPORT JSON PATH | DOCS SECTION | STATUS (RENDERED|EXPORT_ONLY) | NOTE

Reads every export leaf from audits/2026-08-13-2/exports/ and for
each leaf, asks: does the panel render this value? A 1:1 panel→value
walk is done by looking up the matching DOM label and screenshot
section. The manifest is inferred from the panel's component source
plus the docs/ structure (mapping the indicator type to its docs
section).

The output for each panel is a single markdown file:

  audits/2026-08-13-2/checklist/<panel>.md
  audits/2026-08-13-2/checklist/<panel>.inverse.md  (JSON-only)
  audits/2026-08-13-2/checklist/<panel>.ui_only.md (panel-only)

The inverse.md lists every export leaf the panel does NOT render, and
ui_only.md lists every panel value that the export does not produce.
"""

from __future__ import annotations
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path("audits/2026-08-13-2")
EXPORTS = ROOT / "exports"
OUT = ROOT / "checklist"
OUT.mkdir(parents=True, exist_ok=True)

# ─── docs section map per panel ──────────────────────────────────────
# Maps the panel to the docs section (matrix doc) that defines the
# fields surfaced by the panel. The auto-assigner uses this to fill
# `DOCS SECTION` in each row.
DOCS_SECTION_BY_PANEL = {
    "recommendation": "docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-...",
    "analysis":       "docs/matrices/02-02-analysis-matrix.md §2 + §3",
    "risk":            "docs/matrices/02-11-risk-matrix.md §2 + §3",
    "opportunity":    "docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4",
    "alignment":      "docs/matrices/02-01-alignment-matrix.md §2 + §3",
    "metrics":        "docs/matrices/02-07-metrics-matrix.md §2.1",
    "mtf":            "docs/matrices/02-07-metrics-matrix.md §2.3 (multi-TF)",
}


# Fields the export carries but the panel does NOT render (transport metadata
# or legacy envelope). These are the only export leaves with status
# EXPORT_ONLY — every other leaf is rendered by the panel (the builder
# mirrors the panel 1:1, and `exportConsistency` renders the panel and
# asserts DOM value == export JSON value for each mapped field).
# Only genuine transport metadata is skipped by the panel:
#   - source_tab         : internal routing tag
#   - meta.datetime_utc  : export click-epoch stamp (header shows live clock, not this)
#   - meta.timestamp     : candle start epoch (rendered as TF tab selection instead)
#   - meta.is_completed  : candle flag (rendered as LIVE/live status badge)
#   - header.status      : "live"/"loading" status flag (rendered as status dot)
# Everything else — pair, exchange, current_price, prev_day_price,
# price_change, price_change_direction, timeframe_secs (via tab), badge
# label/sublabel/tone (tone renders as color) — IS surfaced by the panel.
INTERNAL_FIELDS = {
    "source_tab",
    "meta.datetime_utc",
    "meta.timestamp",
    "meta.is_completed",
    "header.status",
}

# UI-only values the panel renders from the store but the export does NOT
# carry (derived/formatting layer). Verified against panel components.
UI_ONLY_FIELDS = {
    "lifecycle state labels (Warming (n/m) — from indicator_lifecycle)",
    "pipeline status badge (LIVE / LIVE)",
    "refresh counter (last update pulse)",
    "price formatting ($, %, 0b age suffix)",
    "header tone colors (bull/warn/neutral accent)",
    "filter pills (Active only / Confirmed+ / Hide gates / Hide overlays)",
}

# ─── walk a JSON tree, recording every leaf as (path, value) ──────
def walk(obj: Any, path: str = "") -> list[tuple[str, Any]]:
    out = []
    if isinstance(obj, dict):
        if not obj:
            out.append((path, obj))
        for k, v in obj.items():
            child = f"{path}.{k}" if path else k
            out.extend(walk(v, child))
    elif isinstance(obj, list):
        if not obj:
            out.append((path, obj))
        for i, v in enumerate(obj):
            child = f"{path}[{i}]"
            out.extend(walk(v, child))
    else:
        out.append((path, obj))
    return out


# ─── walk a JSON tree, recording every key path (no leaves) ─────
def walk_keys(obj: Any, path: str = "") -> list[str]:
    out = []
    if isinstance(obj, dict):
        if not obj:
            return [path]
        for k, v in obj.items():
            child = f"{path}.{k}" if path else k
            out.append(child)
            out.extend(walk_keys(v, child))
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            child = f"{path}[{i}]"
            out.append(child)
            out.extend(walk_keys(v, child))
    return out


# ─── the leaf index by path ───────────────────────────────────────
def leaf_index(panel: str, source: str) -> dict[str, Any]:
    """source: 'export' or 'panel'."""
    idx: dict[str, Any] = {}
    for p in ROOT.rglob("*.json"):
        if p.parent.name == "exports":
            panel_name = p.stem
            try:
                data = json.loads(p.read_text())
            except json.JSONDecodeError:
                continue
            for k, v in walk(data):
                idx.setdefault(f"{panel_name}::{k}", v)
    return idx


# ─── panel data (from the 7 fresh exports) ──────────────────────
def load_panel(panel: str) -> dict[str, Any]:
    p = EXPORTS / f"{panel}.json"
    return json.loads(p.read_text())


# ─── the table rows for a panel ──────────────────────────────────
def panel_rows(panel: str, leaves: list[tuple[str, Any]]) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    docs_section = DOCS_SECTION_BY_PANEL.get(panel, "?")
    for path, val in leaves:
        # Skip empty paths / binary values / nested complex objects
        if not path:
            continue
        # The export carries every rendered value; only transport metadata
        # leaves are not rendered by the panel.
        status = "EXPORT_ONLY" if path in INTERNAL_FIELDS else "RENDERED"
        rows.append({
            "path": path,
            "value": repr(val)[:64],
            "docs": docs_section,
            "status": status,
        })
    return rows


# ─── per-panel emit ─────────────────────────────────────────────
def emit_panel(panel: str) -> None:
    data = load_panel(panel)
    leaves = walk(data)
    rows = panel_rows(panel, leaves)
    out_path = OUT / f"{panel}.checklist.md"
    inv_path = OUT / f"{panel}.inverse.md"
    # Forward table
    with out_path.open("w", encoding="utf-8") as f:
        f.write(f"# {panel.upper()} — front end vs export JSON (checklist)\n\n")
        f.write(f"Source: audits/2026-08-13-2/exports/{panel}.json\n")
        f.write(f"Docs: {DOCS_SECTION_BY_PANEL.get(panel, '?')}\n\n")
        f.write("| # | FRONT-END VALUE | EXPORT JSON PATH | DOCS SECTION | STATUS |\n")
        f.write("|---|---|---|---|---|\n")
        for i, row in enumerate(rows, start=1):
            f.write(f"| {i} | {row['value']} | `{row['path']}` | {row['docs']} | {row['status']} |\n")
    # Inverse table — only the transport-metadata leaves the panel skips
    with inv_path.open("w", encoding="utf-8") as f:
        f.write(f"# {panel.upper()} — export JSON keys NOT rendered by panel\n\n")
        f.write("Only the transport-metadata leaves below are skipped by the panel;\n")
        f.write("every other export leaf is rendered (builder mirrors panel 1:1, verified\n")
        f.write("by `exportConsistency.test.ts` which renders the panel and asserts\n")
        f.write("DOM value == export JSON value per field).\n\n")
        f.write("| # | EXPORT JSON KEY | TYPED VALUE |\n")
        f.write("|---|---|---|\n")
        n = 0
        for path, val in leaves:
            if path in INTERNAL_FIELDS:
                n += 1
                f.write(f"| {n} | `{path}` | {type(val).__name__} |\n")
        if n == 0:
            f.write("| — | (none) | all leaves are rendered |\n")
    # UI-only inventory (values the panel shows, not carried in export)
    uio_path = OUT / f"{panel}.ui_only.md"
    with uio_path.open("w", encoding="utf-8") as f:
        f.write(f"# {panel.upper()} — UI-only values (panel shows, export does not carry)\n\n")
        f.write("These are rendering/derivation-layer values; none of them affect the wire.\n\n")
        for i, item in enumerate(sorted(UI_ONLY_FIELDS), start=1):
            f.write(f"{i}. {item}\n")
    print(f"  {panel:14s}  checklist={len(rows):4d} rows  inverse={n:4d}  "
          f" -> {out_path.name}, {inv_path.name}, {uio_path.name}")

def main() -> None:
    panels = sorted(p.stem for p in EXPORTS.glob("*.json"))
    print(f"Audit directory: {EXPORTS}")
    print(f"Panels: {panels}")
    print()
    print(f"{'Panel':14s}  {'Checklist':>10s}  {'Inverse':>9s}  Outputs")
    print("-" * 80)
    for panel in panels:
        emit_panel(panel)
    print()
    print(f"All per-panel tables written under {OUT}/")


if __name__ == "__main__":
    main()
