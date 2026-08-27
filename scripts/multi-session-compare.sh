#!/usr/bin/env bash
# multi-session-compare.sh — v10.1 parallel-experiment harness.
#
# Runs one headless backtest per experiment folder (IN PARALLEL), then
# aggregates every folder's ds/ tree into one cross-folder comparison
# report via `execution-daemon --compare-folders`.
#
# Rate-limit note: exchange REST limits are PER-EXCHANGE. Folders on
# different exchanges (--exchanges hyperliquid,bitget) run fully
# independently; folders on the SAME exchange share that exchange's candle
# endpoint limit — fine for the default matrix, stagger if you go beyond a
# handful of same-exchange folders.
#
# Depth note (exchange-aware ceilings): Hyperliquid's 5000-candle cap means
# a 60s ladder can only reach ~3 days of depth — and the warm-up window
# (warmup_bars × macro TF) needs more than that, so a 60s ladder is NOT
# viable on Hyperliquid at all. The defaults below use a 300s-ladder
# (300s..3600s) which satisfies both exchanges: warm-up 12.5d < depth 14d
# < Hyperliquid's 17.4d ceiling at 300s. Bitget's candle API allows the
# full range. Failed runs are reported at the end, not silently skipped.
#
# Usage:
#   ./scripts/multi-session-compare.sh
#   ./scripts/multi-session-compare.sh --exchanges hyperliquid,bitget \
#       --strategies default,conservative --depth 7 --clean
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BIN="${E2E_BIN:-$ROOT/target/release/execution-daemon}"
EXPS="$ROOT/experiments"

EXCHANGES="hyperliquid"
SYMBOLS="BTC-USDT,ETH-USDT"
STRATEGIES="default,conservative,aggressive"
DEPTH=14
TF="300,900,1800,3600"
CAPITAL=1000
ALLOCATION=10
CLEAN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --exchanges) EXCHANGES="$2"; shift 2 ;;
        --symbols) SYMBOLS="$2"; shift 2 ;;
        --strategies) STRATEGIES="$2"; shift 2 ;;
        --depth) DEPTH="$2"; shift 2 ;;
        --tf) TF="$2"; shift 2 ;;
        --capital) CAPITAL="$2"; shift 2 ;;
        --allocation) ALLOCATION="$2"; shift 2 ;;
        --clean) CLEAN=1; shift ;;
        *) echo "unknown arg: $1"; exit 2 ;;
    esac
done

if [ "$CLEAN" = "1" ] && [ -d "$EXPS" ]; then
    echo "── cleaning $EXPS ──"
    rm -rf "$EXPS"
fi
mkdir -p "$EXPS"

[ -x "$BIN" ] || { echo "❌ release binary missing — run: cargo build --release --bin execution-daemon"; exit 1; }

IFS=',' read -r -a EXCH_LIST <<< "$EXCHANGES"
IFS=',' read -r -a SYM_LIST <<< "$SYMBOLS"
IFS=',' read -r -a STRAT_LIST <<< "$STRATEGIES"

# strategy JSONs (same definitions as the DS verification loop)
cat > "$EXPS/conservative.json" <<'JSON'
{
  "schema_version": 1,
  "name": "conservative",
  "base": "default",
  "description": "v10 compare harness: tighter intake, breadth floor, no trailing",
  "tae": { "intake": { "min_score": 35.0, "min_confidence": 0.55 } },
  "l7": { "breadth_entry_floor": -25.0 }
}
JSON
cat > "$EXPS/aggressive.json" <<'JSON'
{
  "schema_version": 1,
  "name": "aggressive",
  "base": "default",
  "description": "v10 compare harness: looser intake, fast reentry, trailing + vol scale",
  "tae": {
    "intake": { "min_score": 20.0 },
    "lifecycle": { "reentry_cooldown_bars": 0 },
    "risk": { "trailing": { "activate_at_rr": 1.0, "atr_mult": 1.5 } },
    "sizing": { "vol_scale": { "mode": "fixed", "override_factor": 1.5 } }
  }
}
JSON

FOLDER_PATHS=()
PORT=3100
EXP_IDX=0

run_experiment() {
    local exchange="$1" symbol="$2" strategy="$3"
    local name="${exchange}-${strategy}-${symbol}"
    local dir="$EXPS/$name"
    mkdir -p "$dir"
    cp config.default.toml "$dir/config.toml"
    sed -i "s/^port = 3000/port = $PORT/" "$dir/config.toml"
    FOLDER_PATHS+=("$dir")
    (
        cd "$dir" || exit 1
        # bind the strategy into THIS folder's config (default is built-in)
        if [ "$strategy" != "default" ]; then
            "$BIN" --strategy-create "$strategy" "$EXPS/$strategy.json" > strategy.out 2>&1 || true
        fi
        "$BIN" --backtest --exchange "$exchange" --symbols "$symbol" \
            --tf "$TF" --depth "$DEPTH" --capital "$CAPITAL" \
            --allocation "$ALLOCATION" --strategy "$strategy" \
            > backtest.log 2>&1
    ) &
    PORT=$((PORT + 1))
    EXP_IDX=$((EXP_IDX + 1))
}

echo "── launching $((${#EXCH_LIST[@]} * ${#SYM_LIST[@]} * ${#STRAT_LIST[@]})) experiments in parallel ──"
for ex in "${EXCH_LIST[@]}"; do
    for sym in "${SYM_LIST[@]}"; do
        for strat in "${STRAT_LIST[@]}"; do
            run_experiment "$ex" "$sym" "$strat"
        done
    done
done

echo "── waiting for all backtests ──"
wait
echo "── all experiments finished ──"

# Surface failed runs loudly (failed backtests leave no ds/ tree and would
# otherwise be silently absent from the comparison).
FAILED=0
for d in "${FOLDER_PATHS[@]}"; do
    if [ -f "$d/backtest.log" ] && grep -q '"status":"failed"' "$d/backtest.log" 2>/dev/null; then
        echo "⚠️  FAILED: $(basename "$d") — $(grep -o '"error":"[^"]*"' "$d/backtest.log" | head -1)"
        FAILED=1
    fi
done
[ "$FAILED" = "1" ] && echo "ℹ️  Failed experiments are excluded from the comparison (see their backtest.log)."

echo "── cross-folder comparison ──"
"$BIN" --compare-folders "${FOLDER_PATHS[@]}" | tee "$EXPS/COMPARISON.md"
"$BIN" --compare-folders "${FOLDER_PATHS[@]}" > "$EXPS/COMPARISON.json" 2>/dev/null

echo ""
echo "════════════════════════════════════════════════"
echo "RESULT: report at $EXPS/COMPARISON.md (+ .json)"
echo "════════════════════════════════════════════════"
