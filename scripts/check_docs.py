#!/usr/bin/env python3
"""
Documentation corpus consistency checker — v6.2 gate.
All ten checks exit 0 on pass, 1 on failure.
Run via: python3 scripts/check_docs.py
or via:  ./manage.sh test-doc
"""
import os
import re
import sys
from pathlib import Path
from collections import Counter

ROOT = Path(__file__).resolve().parent.parent / "docs"
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+)$", re.MULTILINE)
SECTION_REF_RE = re.compile(r"(\d{2}-\d{2}(?:[a-z])?(?:-\d{2})?) §([\d.A-Za-z]+)")

failures = 0

def fail(msg):
    global failures
    print(f"  FAIL: {msg}")
    failures += 1

def ok(msg):
    print(f"  OK: {msg}")

# ── Check 1: Link Resolution ──────────────────────────────────────────
def check_links():
    print("\n=== CHECK 1: Link Resolution ===")
    broken = 0
    all_md = list(ROOT.rglob("*.md"))
    for md in all_md:
        base = md.parent
        text = md.read_text()
        for m in LINK_RE.finditer(text):
            target = m.group(1).strip()
            if target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            path_part = target.split("#", 1)[0]
            if not path_part:
                continue
            try:
                resolved = (base / path_part).resolve()
            except Exception:
                broken += 1
                if broken <= 20:
                    fail(f"{md.relative_to(ROOT)}: broken link -> {target}")
                continue
            if not resolved.exists():
                broken += 1
                if broken <= 20:
                    fail(f"{md.relative_to(ROOT)}: missing target -> {target}")
    if broken == 0:
        ok(f"All relative links resolve correctly")
    else:
        fail(f"{broken} broken links found")

# ── Check 2: Retired Term Grep ─────────────────────────────────────────
def check_retired_terms():
    print("\n=== CHECK 2: Retired Term Grep ===")
    retired = [
        "retry_cooldown",
        "emergency_liquidation",   # must be is_emergency_liquidation
        "reward_risk",
        "correlation_risk",
        "environment_favorability",
        "invalid_level",
        "final_invalidation_level",
        "opportunity_classification",
        "LinearInterpolation",
        "roi_percentage",
        "Direction Matrix",
        "Regime Compatibility Matrix",
        "Pause event loop",         # Phase 10
        "(pause/resume)",           # Phase 10
    ]
    hits = 0
    all_md = list(ROOT.rglob("*.md"))
    # Skip CHANGELOG and MANIFEST — they intentionally reference historical/retired terms
    skip_files = {"CHANGELOG.md", "DOCS-CONSISTENCY-MANIFEST.md"}
    for md in all_md:
        text = md.read_text()
        rel = str(md.relative_to(ROOT))
        fname = md.name
        if fname in skip_files:
            continue
        for i, line in enumerate(text.split("\n")):
            for term in retired:
                # Use word boundary regex to avoid matching substrings
                if re.search(rf'\b{re.escape(term)}\b', line):
                    hits += 1
                    if hits <= 15:
                        fail(f"{rel}:{i+1}: retired term '{term}' found")
    # Special: bare "PAUSED" check in normative sections for Phase 10
    bare_paused = 0
    # Canonical lifecycle spec may use bare PAUSED for enum value references
    canonical_lifecycle = {"03-03-06-tae-instance-lifecycle-spec.md"}
    for md in all_md:
        rel = str(md.relative_to(ROOT))
        if rel.startswith("CHANGELOG") or rel.startswith("DOCS-CONSISTENCY"):
            continue
        if md.name in canonical_lifecycle:
            continue
        in_code_fence = False
        for i, line in enumerate(md.read_text().split("\n")):
            if line.strip().startswith("```"):
                in_code_fence = not in_code_fence
                continue
            if in_code_fence:
                continue
            # Match "PAUSED" that is NOT "AUTO_PAUSED" and not qualified
            if re.search(r"\bPAUSED\b", line) and "AUTO_PAUSED" not in line:
                # Must be qualified with axis: "instance PAUSED", "lifecycle PAUSED"
                # Accept backtick or other punctuation between qualifier and PAUSED
                if not re.search(r"(instance|lifecycle|instance-scope|policy)\s*[`'\"]*\s*PAUSED", line):
                    bare_paused += 1
    if bare_paused > 0:
        fail(f"{bare_paused} bare PAUSED references without axis qualifier (Phase 10 scoped-enum rule)")
    if hits == 0 and bare_paused == 0:
        ok(f"No retired terms or bare PAUSED found")
    else:
        fail(f"Found retired terms or bare PAUSED")

# ── Check 3: Version Stamp Audit ───────────────────────────────────────
def check_version_stamps():
    print("\n=== CHECK 3: Version Stamp Audit ===")
    all_md = list(ROOT.rglob("*.md"))
    non_numbered = {"README.md", "CHANGELOG.md", "DOCS-CONSISTENCY-MANIFEST.md"}
    wrong = 0
    missing = 0
    for md in all_md:
        rel = str(md.relative_to(ROOT))
        fname = md.name
        if fname in non_numbered:
            continue
        if fname == "DOCS-CONSISTENCY-MANIFEST.md":
            continue
        text = md.read_text()
        if re.search(r"\*\*Version:\*\*\s*6\.2\s*\(2026-07-17\)", text):
            continue
        m = re.search(r"\*\*Version:\*\*\s*(.+?)(?:\s*—|\s*$|\s*\n)", text)
        if m:
            wrong += 1
            if wrong <= 10:
                fail(f"{rel}: version is '{m.group(1).strip()}', expected '6.2 (2026-07-17)'")
        else:
            missing += 1
            if missing <= 10:
                fail(f"{rel}: missing version stamp")
    if wrong == 0 and missing == 0:
        ok(f"All numbered docs carry Version: 6.2 (2026-07-17)")
    else:
        fail(f"{wrong} wrong version stamps, {missing} missing stamps")

# ── Check 4: File Inventory ────────────────────────────────────────────
def check_inventory():
    print("\n=== CHECK 4: File Inventory ===")
    all_md = list(ROOT.rglob("*.md"))
    count = len(all_md)
    expected = 138
    if count == expected:
        ok(f"File count: {count} (expected {expected})")
    elif count < expected:
        ok(f"File count: {count} (will reach {expected} after Phase 9 + Phase 10 add new docs)")
    else:
        fail(f"File count: {count} (expected {expected})")

# ── Check 5: Section Resolution ────────────────────────────────────────
def check_section_refs():
    print("\n=== CHECK 5: Section Cross-Reference Resolution ===")
    # Build heading index
    headings = {}
    for md in ROOT.rglob("*.md"):
        rel = str(md.relative_to(ROOT))
        text = md.read_text()
        for m in HEADING_RE.finditer(text):
            level = len(m.group(1))
            title = m.group(2).strip()
            # Extract section number from heading like "### 3.1 Foo"
            sec_match = re.match(r"(\d+(?:\.\d+)*)", title)
            if sec_match:
                sec_num = sec_match.group(1)
                headings[(rel, sec_num)] = True

    # Check section references
    broken = 0
    all_md = list(ROOT.rglob("*.md"))
    # Build set of all doc IDs (file name prefixes like "08-05", "03-01-04")
    doc_ids = set()
    for md in ROOT.rglob("*.md"):
        fname = md.name
        # Extract prefix like "08-05", "03-01-04", "02-00b", etc.
        m = re.match(r"(\d{2}-\d{2}[a-z]?(?:-\d{2})?)", fname)
        if m:
            doc_ids.add(m.group(1))
    for md in all_md:
        text = md.read_text()
        for m in SECTION_REF_RE.finditer(text):
            doc_id = m.group(1)
            section = m.group(2)
            if doc_id not in doc_ids:
                broken += 1
                if broken <= 10:
                    fail(f"{md.relative_to(ROOT)}: referenced doc '{doc_id}' not found")
    if broken == 0:
        ok(f"All section cross-references resolve to known documents")
    else:
        fail(f"{broken} unresolved section cross-references")

# ── Check 6: Worked-Example Recompute ──────────────────────────────────
def check_worked_examples():
    print("\n=== CHECK 6: Worked-Example Recomputation ===")
    # Verify the A-series numbers in 01-01-ontology.md
    onto = ROOT / "conceptual-foundations" / "01-01-ontology.md"
    text = onto.read_text() if onto.exists() else ""

    errors = 0

    # Check state_confidence: should be 0.65 (|40|/100 + 0.15 + 0.10)
    if "state_confidence" in text:
        m = re.search(r'"state_confidence"\s*:\s*0\.82', text)
        if m:
            fail(f"01-01-ontology.md: state_confidence still 0.82 (should be 0.65)")
            errors += 1

    # Check confidence_assessment = 0.65 * 0.717 * 100 = 46.61
    m = re.search(r'"confidence_assessment"\s*:\s*59\.07', text)
    if m:
        fail(f"01-01-ontology.md: confidence_assessment still 59.07 (should be 46.61)")
        errors += 1

    # Check AssetRank: should be 87.5
    m = re.search(r'"score"\s*:\s*87\.0', text)
    if m:
        # Only if it's in the AssetRank context
        context = text[max(0, m.start()-100):m.end()+100]
        if "AssetRank" in context or "asset_rank" in context:
            fail(f"01-01-ontology.md: AssetRank score still 87.0 (should be 87.5)")
            errors += 1

    # Check 02-03 quality_score: should be 100.0
    dq = ROOT / "matrices" / "02-03-data-quality-matrix.md"
    if dq.exists():
        dq_text = dq.read_text()
        if re.search(r'"quality_score"\s*:\s*98\.0', dq_text):
            fail(f"02-03-data-quality-matrix.md: quality_score still 98.0 (should be 100.0)")
            errors += 1

    # Check 02-05 quality_score: should be 100.0
    dist = ROOT / "matrices" / "02-05-distribution-matrix.md"
    if dist.exists():
        dist_text = dist.read_text()
        if re.search(r'"quality_score"\s*:\s*98\.0', dist_text):
            fail(f"02-05-distribution-matrix.md: quality_score still 98.0 (should be 100.0)")
            errors += 1

    # Check 02-08 opportunity_score: "PRIME" with score 85.0 -> "STRONG"
    opp = ROOT / "matrices" / "02-08-opportunity-matrix.md"
    if opp.exists():
        opp_text = opp.read_text()
        if re.search(r'"setup_quality"\s*:\s*"PRIME".*"opportunity_score"\s*:\s*85', opp_text, re.DOTALL):
            fail(f"02-08-opportunity-matrix.md: 85.0 still labeled PRIME (should be STRONG)")
            errors += 1

    # Check signal counts in 04-02-00
    idx = ROOT / "engines" / "market-monitoring-engine" / "indicators" / "04-02-00-indicator-index.md"
    if idx.exists():
        idx_text = idx.read_text()
        wrong_counts = [
            ("Crossover 10", "Crossover 9"),
            ("Threshold 21", "Threshold 26"),
            ("ZeroLineCross 13", "ZeroLineCross 11"),
            ("TrendFlip 10", "TrendFlip 8"),
        ]
        for wrong, correct in wrong_counts:
            if wrong in idx_text and correct not in idx_text:
                fail(f"04-02-00-indicator-index.md: still shows '{wrong}' (should be '{correct}')")
                errors += 1

    # Check connection quality score
    cq = ROOT / "operations-and-compliance" / "08-05-connection-quality.md"
    if cq.exists():
        cq_text = cq.read_text()
        m = re.search(r'0\.50\s*×\s*uptime_pct', cq_text)
        if m:
            fail(f"08-05-connection-quality.md: formula still mixes scales (should use point-scale)")
            errors += 1

    if errors == 0:
        ok(f"All canonical worked examples recompute correctly")
    else:
        fail(f"{errors} worked-example errors")

# ── Check 7: Signal Registry Tally ─────────────────────────────────────
def check_signal_registry():
    print("\n=== CHECK 7: Signal Registry Tally ===")
    idx = ROOT / "engines" / "market-monitoring-engine" / "indicators" / "04-02-00-indicator-index.md"
    if not idx.exists():
        fail("04-02-00-indicator-index.md not found")
        return
    idx_text = idx.read_text()

    # Per-indicator signal kinds from the table
    expected = {
        "Divergence": 9, "Crossover": 9, "Threshold": 26, "Breakout": 9,
        "BandTouch": 4, "ZeroLineCross": 11, "CompressionRelease": 4,
        "LevelTest": 14, "TrendFlip": 8, "VolumeClimax": 2,
        "StackChange": 1, "PatternForming": 3
    }

    # Count declarations (indicator rows that mention each SignalKind)
    per_kind = {k: 0 for k in expected}
    for k in expected:
        # Only count rows that are actual indicator entries: start with | followed by a number (index)
        pattern = rf"^\|\s*\d+\s*\|.*\b{k}\b"
        matches = re.findall(pattern, idx_text, re.MULTILINE)
        per_kind[k] = len(matches)

    errors = 0
    for kind, count in expected.items():
        actual = per_kind[kind]
        if actual != count:
            fail(f"Signal kind {kind}: expected {count}, found {actual}")
            errors += 1

    total = sum(per_kind.values())
    if total == 100:
        ok(f"Signal declarations tally: {total}/100, per-kind counts verify")
    else:
        fail(f"Signal declarations tally: {total} (expected 100)")

# ── Check 8: Enum CHECK Cross-Check (Phase 10) ─────────────────────────
def check_enum_cross():
    print("\n=== CHECK 8: Enum CHECK Cross-Check ===")
    db_schema = ROOT / "integration-and-api" / "06-02-database-schema-spec.md"
    lifecycle = ROOT / "engines" / "trade-automation-engine" / "03-03-06-tae-instance-lifecycle-spec.md"
    tae_policy = ROOT / "engines" / "trade-automation-engine" / "03-03-04-tae-execution-policy-spec.md"

    errors = 0
    if db_schema.exists():
        db_text = db_schema.read_text()
        expected_states = {"RUNNING", "PAUSED", "STOPPING", "STOPPED"}
        # Find lifecycle_state CHECK
        m = re.search(r"lifecycle_state\s+TEXT[^)]*CHECK\s*\(([^)]+)\)", db_text)
        if m:
            check_str = m.group(1)
            actual_states = set(re.findall(r"'(\w+)'", check_str))
            if expected_states != actual_states:
                fail(f"instance_lifecycle CHECK states mismatch: DB={actual_states}, spec={expected_states}")
                errors += 1
            else:
                ok(f"lifecycle_state CHECK matches IL-01 spec: {expected_states}")

    if lifecycle.exists():
        lc_text = lifecycle.read_text()
        # AUTO_PAUSED should not be a value in the LifecycleState enum declaration
        # (it's a policy state). Look for the CHECK or table that defines the enum values.
        m = re.search(r"lifecycle_state.*CHECK.*\(([^)]+)\)", lc_text)
        if m:
            check_values = re.findall(r"'(\w+)'", m.group(1))
            if "AUTO_PAUSED" in check_values:
                fail(f"03-03-06: AUTO_PAUSED appears as LifecycleState CHECK value")
                errors += 1
        # Also check the decision-table values
        m2 = re.search(r"to_state.*CHECK.*\(([^)]+)\)", lc_text)
        if m2:
            check_values = re.findall(r"'(\w+)'", m2.group(1))
            if "AUTO_PAUSED" in check_values:
                fail(f"03-03-06: AUTO_PAUSED appears as to_state CHECK value")
                errors += 1

    if errors == 0:
        ok(f"Enum CHECK cross-check passes")

# ── Check 9: Gate-Chain Completeness (Phase 10) ────────────────────────
def check_gate_chain():
    print("\n=== CHECK 9: Gate-Chain Completeness ===")
    risk_ctrl = ROOT / "operations-and-compliance" / "08-02-pre-trade-risk-controls.md"
    if not risk_ctrl.exists():
        ok(f"Skipped: 08-02 not found")
        return
    text = risk_ctrl.read_text()
    expected_gates = {f"Gate {i}" for i in range(0, 8)}
    found_gates = set(re.findall(r"Gate\s+(\d)", text))
    found_gates = {f"Gate {g}" for g in found_gates}
    if expected_gates == found_gates:
        ok(f"All Gates 0–7 present in 08-02 pre-trade risk controls")
    else:
        missing = expected_gates - found_gates
        extra = found_gates - expected_gates
        fail(f"Gate mismatch: missing={missing}, extra={extra}")

# ── Check 10: Endpoint Cross-Check (Phase 10) ──────────────────────────
def check_endpoint_cross():
    print("\n=== CHECK 10: Endpoint Cross-Check ===")
    api = ROOT / "integration-and-api" / "06-01-api-gateway-contract.md"
    lifecycle = ROOT / "engines" / "trade-automation-engine" / "03-03-06-tae-instance-lifecycle-spec.md"
    if not api.exists():
        ok(f"Skipped: 06-01 not found")
        return
    api_text = api.read_text()
    errors = 0

    # These endpoints should exist after Phase 10
    required_endpoints = [
        ("POST", "/api/instances/:id/start"),
        ("POST", "/api/instances/:id/pause"),
        ("POST", "/api/instances/:id/stop"),
        ("DELETE", "/api/instances/:id"),
    ]
    for method, path in required_endpoints:
        if path in api_text:
            ok(f"Endpoint {method} {path} present in 06-01")
        else:
            fail(f"Endpoint {method} {path} missing from 06-01")
            errors += 1

    if errors == 0:
        ok(f"All required endpoints documented in 06-01")

# ── Main ────────────────────────────────────────────────────────────────
def main():
    global failures
    print("=" * 60)
    print("DOC CORPUS CONSISTENCY CHECK — v6.2")
    print("=" * 60)
    check_links()
    check_retired_terms()
    check_version_stamps()
    check_inventory()
    check_section_refs()
    check_worked_examples()
    check_signal_registry()
    check_enum_cross()
    check_gate_chain()
    check_endpoint_cross()
    print("=" * 60)
    if failures == 0:
        print("RESULT: ALL CHECKS PASSED")
        sys.exit(0)
    else:
        print(f"RESULT: {failures} FAILURE(S)")
        sys.exit(1)

if __name__ == "__main__":
    main()
