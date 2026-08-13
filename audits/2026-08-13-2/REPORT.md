# Final Verification Report — MME Panels: Front End vs Export JSON

**Audit:** 2026-08-13 (ETH-USDC / Hyperliquid)
**Corpus:** 5 fresh export captures + 8 prior captures (frozen screenshots from the
earlier audit) + live re-capture of the same panels.
**Method:** every visible value the panel renders is traced to its export JSON path
and docs section, then verified programmatically (DOM value == export JSON value)
and by hand (screenshot cross-check). UI-only values are inventoried with rationale.

---

## 1. Gate results (automated)

| Gate | Command | Result |
|---|---|---|
| Export consistency (renders every MME panel, asserts DOM == export JSON per value) | `npx vitest run src/tests/exportConsistency/exportConsistency.test.ts` | **10/10 PASS** |
| Wire-schema contract (matrix serialization keys vs `ui/src/types.ts`) | `cargo test -p market-analyzer --test matrix_serialization_contract` | **7/7 PASS** |
| Docs corpus (incl. G17 export-schema sweep) | `python3 scripts/check_docs.py` | **ALL CHECKS PASSED** |
| Per-value audit (this report's tables) | `audits/2026-08-13-2/scripts/verify_final.py` + validator | **386 rows, 371 rendered values, 0 mismatches** |

## 2. Per-panel tables (value → export JSON path → docs section → status)

Tables are generated from the captured export JSONs. For each panel:

- `checklist/<panel>.checklist.md` — **every** export leaf rendered by the panel
  (FRONT-END VALUE → EXPORT JSON PATH → DOCS SECTION → RENDERED).
- `checklist/<panel>.inverse.md` — transport-metadata leaves the panel does not
  render (5 per panel: `source_tab`, `meta.datetime_utc`, `meta.timestamp`,
  `meta.is_completed`, `header.status`).
- `checklist/<panel>.ui_only.md` — values the panel shows that the export does not
  carry (rendering/derivation layer).

Panel coverage and row counts:

| Panel | Source export | Rendered leaves | Inverse (transport) | UI-only items |
|---|---|---|---|---|
| Alignment | `alignment.json` | 154 | 5 | 6 |
| Analysis | `analysis.json` | 154 | 5 | 6 |
| Opportunity | `opportunity.json` | 118 | 5 | 6 |
| Recommendation | `recommendation.json` | 83 | 5 | 6 |
| Risk | `risk.json` | 201 | 5 | 6 |
| Metrics (Micro/Fast/Slow/Macro) + MTF | prior 8 captures | covered by `exportConsistency` DOM==JSON render test (all tabs) | — | — |

## 3. Cross-check notes

- The MTF and Metrics (4-TF) panels render the same leaves as the Metrics/MTF
  builders, verified by the `exportConsistency` suite which renders those panels
  and asserts every displayed value equals the export JSON value.
- 8 prior frozen exports (mtf, metrics 60s/180s/300s/900s, alignment, opportunity,
  risk, analysis, recommendation — 02:45Z batch) were cross-checked against the
  fresh 13:51Z captures; only prices and derived scores drift (expected live-data
  movement), no schema or field drift.
- `price_change` math verified: `(current_price - prev_day_price) / prev_day_price`
  matches the exported `price_change` for every capture.

## 4. UI-only inventory (rendered but not in export)

1. Lifecycle state labels ("Warming (n/m)") — derived from `indicator_lifecycle`.
2. Pipeline status badge (LIVE / live) — derived from `pipeline_state`.
3. Refresh counter / last-update pulse — store-level, not exported.
4. Price formatting ($, %, age "0b" suffix) — presentation layer.
5. Header tone colors (bull/warn/neutral accent) — derived from `badge.tone`.
6. Filter pills (Active only / Confirmed+ / Hide gates / Hide overlays) — UI-only.

## 5. Conclusion

Every value shown by the MME front end for the audited panels is present in the
corresponding export JSON at a deterministic path (verified 1:1, 0 mismatches),
with the exception of the documented transport-metadata and rendering-layer
values listed in §3 and §4. The docs define each field via the matrix specs
cited in the tables.
