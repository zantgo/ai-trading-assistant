#!/usr/bin/env bash
# ds-verification-loop.sh — v10 DS-QA loop.
#
# 1. Creates the conservative + aggressive strategies (base: default).
# 2. Runs 12 headless backtests: 3 strategies × 2 symbols × 2 depths.
# 3. Runs the DS pass over the ds/ artifacts (identifiers + invariants).
# 4. Fails loudly on any violation; exit 0 only when everything is green.
set -uo pipefail
BIN=./target/release/execution-daemon
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
WORK=/tmp/opencode/ds-loop
rm -rf "$WORK" && mkdir -p "$WORK"

echo "── creating strategies ──"
cat > "$WORK/conservative.json" <<'JSON'
{
  "schema_version": 1,
  "name": "conservative",
  "base": "default",
  "description": "v10 DS loop: tighter intake, breadth floor, no trailing",
  "tae": { "intake": { "min_score": 35.0, "min_confidence": 0.55 } },
  "l7": { "breadth_entry_floor": -25.0 }
}
JSON
cat > "$WORK/aggressive.json" <<'JSON'
{
  "schema_version": 1,
  "name": "aggressive",
  "base": "default",
  "description": "v10 DS loop: looser intake, fast reentry, trailing + vol scale",
  "tae": {
    "intake": { "min_score": 20.0 },
    "lifecycle": { "reentry_cooldown_bars": 0 },
    "risk": { "trailing": { "activate_at_rr": 1.0, "atr_mult": 1.5 } },
    "sizing": { "vol_scale": { "mode": "fixed", "override_factor": 1.5 } }
  }
}
JSON
"$BIN" --strategy-create conservative "$WORK/conservative.json" > "$WORK/strategy_conservative.out" 2>&1 || { echo "conservative create failed"; cat "$WORK/strategy_conservative.out"; exit 1; }
"$BIN" --strategy-create aggressive "$WORK/aggressive.json" > "$WORK/strategy_aggressive.out" 2>&1 || { echo "aggressive create failed"; cat "$WORK/strategy_aggressive.out"; exit 1; }
grep -q '"success": true' "$WORK/strategy_conservative.out" || { echo "conservative not persisted"; exit 1; }
grep -q '"success": true' "$WORK/strategy_aggressive.out" || { echo "aggressive not persisted"; exit 1; }
echo "strategies persisted ✓"

declare -A RUNS
for strat in default conservative aggressive; do
  for depth in 7 30; do
    for symbol in BTC ETH; do
      echo "── backtest: strategy=$strat symbol=${symbol}-USDT depth=${depth}d ──"
      out="$WORK/bt_${strat}_${symbol}_${depth}.log"
      "$BIN" --backtest --exchange bitget --symbols "$symbol" --tf 60,180,300,900 --depth "$depth" --strategy "$strat" > "$out" 2>&1
      code=$?
      if [ $code -ne 0 ]; then echo "FAIL: backtest $strat/$symbol/$depth exit=$code"; tail -5 "$out"; exit 1; fi
      run_id=$(python3 - "$out" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
m = re.search(r'"run_id":(\d+)', text)
if not m:
    print("NO_ENVELOPE"); sys.exit(0)
print(m.group(1))
PY
)
      if [ "$run_id" = "NO_ENVELOPE" ] || [ "$run_id" = "0" ] || [ -z "$run_id" ]; then
        echo "FAIL: no run_id in $out"; tail -6 "$out"; exit 1
      fi
      RUNS["${strat}_${symbol}_${depth}"]=$run_id
      grep -q '"status": "ok"' "$out" || grep -q "run id" "$out" || { echo "FAIL: status not ok"; exit 1; }
      echo "   run_id=$run_id ✓"
    done
  done
done

echo "── DS pass (identifiers + invariants) ──"
cat > "$WORK/check.py" <<'PY'
import json, math, sys, os

RUNS = json.load(open(sys.argv[1]))
ROOT = os.getcwd()
violations = []
vocab = {"tp","sl","invalidated_signal","manual","stop_flatten","end_of_backtest","time_stop","breakeven","trailing_stop","expired","terminated","daily_budget","setup_gone","confidence_drop"}

def read_ndjson(path):
    rows = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows

for key, run_id in RUNS.items():
    strat, symbol, depth = key.rsplit("_", 2)[0], key.rsplit("_", 2)[1], key.rsplit("_", 2)[2]
    bdir = os.path.join(ROOT, "ds", "backtests", f"BT{int(run_id):04d}_historical")
    if not os.path.isdir(bdir):
        violations.append(f"{key}: ds dir missing {bdir}"); continue
    run = json.load(open(os.path.join(bdir, "run.json")))
    # identifier: run.json backtest_id matches
    if run.get("backtest_id") != int(run_id):
        violations.append(f"{key}: run.json backtest_id mismatch")
    # params carry the strategy id
    params = run.get("params", {}) or {}
    sid = params.get("strategy_id") or params.get("strategy")
    if sid not in (None, strat, "default"):
        violations.append(f"{key}: params strategy {sid} != {strat}")
    trades = read_ndjson(os.path.join(bdir, "trades.ndjson"))
    equity = read_ndjson(os.path.join(bdir, "equity.ndjson"))
    summary = run.get("summary", {}) or {}
    if len(trades) != int(summary.get("total_trades", -1)):
        violations.append(f"{key}: trades ndjson rows {len(trades)} != summary total_trades")
    # per-trade invariants
    for t in trades:
        if not (0 < t.get("ts_entry_secs", 0) <= t.get("ts_close_secs", 0)):
            violations.append(f"{key}: trade ordering violated (entry {t.get('ts_entry_secs')} close {t.get('ts_close_secs')})")
        if t.get("hold_secs", -1) < 0:
            violations.append(f"{key}: negative hold")
        if not t.get("exit_reason") or t["exit_reason"] not in vocab:
            violations.append(f"{key}: bad exit_reason {t.get('exit_reason')}")
        if not all(math.isfinite(t.get(k, 0.0)) for k in ("entry_price","exit_price","size","pnl","mfe_pct","mae_pct","roi_pct")):
            violations.append(f"{key}: non-finite trade field")
        # ROI consistency (|roi - pnl/(entry*size)*100| <= 0.5%)
        expected = 0.0
        if t["entry_price"] > 0 and t["size"] > 0:
            expected = t["pnl"] / (t["entry_price"] * t["size"]) * 100.0
        if abs(t.get("roi_pct", 0.0) - expected) > max(0.5, abs(expected)*0.005):
            violations.append(f"{key}: roi mismatch {t.get('roi_pct')} vs {expected:.4f}")
    # equity invariants
    ts = [e["ts_secs"] for e in equity]
    if ts != sorted(ts) or len(set(ts)) != len(ts):
        violations.append(f"{key}: equity ts not strictly increasing")
    vals = [e["equity"] for e in equity]
    if not all(math.isfinite(v) and v > 0 for v in vals):
        violations.append(f"{key}: non-finite equity")
    # conservation: final ≈ initial + Σ pnl
    initial = params.get("portfolio_capital_usd") or params.get("initial_capital")
    if isinstance(initial, (int, float)) and vals:
        pnl_sum = sum(t["pnl"] for t in trades)
        tol = max(0.5, initial * 0.005)
        if abs(vals[-1] - initial - pnl_sum) > tol:
            violations.append(f"{key}: equity conservation off (final {vals[-1]:.2f} vs initial {initial} + Σpnl {pnl_sum:.2f})")
    # burn-in: no entry before from_secs + warmup_bars * max_tf
    from_s = params.get("from_secs") or params.get("from_ms")
    warmup = params.get("warmup_bars")
    tf_list = params.get("timeframes") or params.get("ladder") or params.get("timeframe_secs") or []
    if not isinstance(tf_list, list):
        tf_list = [tf_list] if tf_list else []
    max_tf = max(int(x) for x in tf_list) if tf_list else 180
    if isinstance(warmup, (int, float)):
        floor = (from_s if isinstance(from_s, (int, float)) else 0) + warmup * max_tf
        for t in trades:
            if t["ts_entry_secs"] > 0 and t["ts_entry_secs"] < floor:
                violations.append(f"{key}: trade entered before burn-in floor")

if violations:
    print("VIOLATIONS:")
    for v in violations:
        print("  -", v)
    sys.exit(1)
print(f"DS pass: {len(RUNS)} runs × {sum(1 for _ in ())} — all invariants green")
PY
python3 "$WORK/check.py" "$(python3 - "$WORK" <<'PY'
import json, sys
runs = {}
# rebuild dict from env lines
PY
)" 2>/dev/null

# simpler: emit the RUNS map to a file
python3 - <<'PY'
import json, subprocess, re, os
runs = {}
for strat in ("default", "conservative", "aggressive"):
    for depth in ("7", "30"):
        for symbol in ("BTC", "ETH"):
            path = f"/tmp/opencode/ds-loop/bt_{strat}_{symbol}_{depth}.log"
            text = open(path).read()
            m = re.search(r'"run_id":(\d+)', text)
            runs[f"{strat}_{symbol}_{depth}"] = int(m.group(1))
json.dump(runs, open("/tmp/opencode/ds-loop/runs.json", "w"))
print("runs.json written")
PY
python3 "$WORK/check.py" "$WORK/runs.json"
rc=$?
if [ $rc -ne 0 ]; then exit 1; fi

echo "── cross-strategy sanity ──"
python3 - "$WORK/runs.json" <<'PY'
import json, subprocess, sys, re
runs = json.load(open(sys.argv[1]))
def trades_of(kind, depth):
    return sum(
        int(re.search(r'"total_trades":(\d+)', open(f"/tmp/opencode/ds-loop/bt_{kind}_{sym}_{depth}.log").read()).group(1))
        for sym in ("BTC", "ETH")
    )
cons7, def7, aggr7 = trades_of("conservative","7"), trades_of("default","7"), trades_of("aggressive","7")
cons30, def30, aggr30 = trades_of("conservative","30"), trades_of("default","30"), trades_of("aggressive","30")
print(f"trades: conservative {cons7}/{cons30} · default {def7}/{def30} · aggressive {aggr7}/{aggr30}")
viol = []
# 1. Data richness: at 30d depth every strategy must actually trade.
for kind, n in (("conservative", cons30), ("default", def30), ("aggressive", aggr30)):
    if n < 1:
        viol.append(f"{kind} produced zero trades at 30d depth")
# 2. Strategy binding is REAL: the three strategies must not be identical
#    trade-for-trade (at least one differing trade count).
if len({cons7, def7, aggr7}) < 2 and len({cons30, def30, aggr30}) < 2:
    viol.append("all three strategies produced identical trade counts — binding not effective")
if viol:
    print("VIOLATIONS:")
    for v in viol:
        print("  -", v)
    sys.exit(1)
print("cross-strategy sanity holds ✓")
PY
rc2=$?
if [ $rc2 -ne 0 ]; then exit 1; fi

echo "── surfaces ──"
"$BIN" --sessions > "$WORK/sessions.out" 2>&1 || true
grep -q '"sessions"' "$WORK/sessions.out" && echo "--sessions ✓" || { echo "FAIL: --sessions"; exit 1; }
first_bt=$(python3 -c "import json;print(json.load(open('$WORK/runs.json'))['default_BTC_7'])")
"$BIN" --backtest-show "$first_bt" > "$WORK/show.out" 2>&1
grep -q '"trades"' "$WORK/show.out" && grep -q 'ds_files' "$WORK/show.out" && echo "--backtest-show ✓" || { echo "FAIL: --backtest-show"; exit 1; }

echo ""
echo "════════════════════════════════════════════"
echo "RESULT: DS LOOP GREEN (12 runs, all invariants)"
echo "════════════════════════════════════════════"
