#!/usr/bin/env python3
"""E2E backtest verifier (v8.2) — sqlite invariants for a persisted run.

Usage:
    e2e_backtest_verify.py check <db> <run_id> <capital> <from_secs> <to_secs> <min_tf_secs> <burn_in_secs>
    e2e_backtest_verify.py compare <db> <run_id_a> <run_id_b>

check: exit 0 when every invariant holds; prints one line per violation.
       Includes the archive-coverage assertion: the archive must reach
       `from_secs - burn_in_secs` for the smallest ladder TF (the run
       actually saw the full requested window — partial data fails).
compare: exit 0 when both runs have identical trade sequences (determinism).
"""
import json
import sqlite3
import sys

EXIT_REASONS = {"tp", "sl", "invalidated_signal", "manual", "stop_flatten", "end_of_backtest"}


def trades(db: str, run_id: int) -> list[dict]:
    con = sqlite3.connect(db)
    try:
        rows = con.execute(
            "SELECT seq, ts_close_secs, direction, entry_price, exit_price, size, pnl, exit_reason "
            "FROM backtest_trades WHERE run_id = ? ORDER BY seq",
            (run_id,),
        ).fetchall()
    finally:
        con.close()
    return [
        {
            "seq": r[0], "ts_close_secs": r[1], "direction": r[2],
            "entry_price": r[3], "exit_price": r[4], "size": r[5],
            "pnl": r[6], "exit_reason": r[7],
        }
        for r in rows
    ]


def equity(db: str, run_id: int) -> list[tuple[int, float]]:
    con = sqlite3.connect(db)
    try:
        rows = con.execute(
            "SELECT ts_secs, equity FROM backtest_equity WHERE run_id = ? ORDER BY ts_secs",
            (run_id,),
        ).fetchall()
    finally:
        con.close()
    return rows


def check(args: list[str]) -> int:
    db, run_id_s, capital_s, from_s, to_s, min_tf_s, burn_in_s = args
    run_id = int(run_id_s)
    capital = float(capital_s)
    from_secs = int(from_s)
    to_secs = int(to_s)
    min_tf_secs = int(min_tf_s)
    burn_in_secs = int(burn_in_s)
    violations: list[str] = []

    con = sqlite3.connect(db)
    n = con.execute("SELECT COUNT(*) FROM backtest_runs WHERE id = ?", (run_id,)).fetchone()[0]
    if n != 1:
        violations.append(f"run {run_id} missing from backtest_runs")
    con.close()

    # Archive coverage: the smallest ladder TF must reach the burn-in
    # window start — partial data would silently shrink the backtest.
    con = sqlite3.connect(db)
    params_json = con.execute(
        "SELECT params_json FROM backtest_runs WHERE id = ?", (run_id,)
    ).fetchone()
    con.close()
    symbol = None
    if params_json and params_json[0]:
        try:
            symbol = json.loads(params_json[0]).get("symbol", "").split(",")[0]
        except Exception:
            symbol = None
    if symbol:
        con = sqlite3.connect(db)
        earliest = con.execute(
            "SELECT MIN(ts_secs) FROM candle_archive WHERE symbol = ? AND timeframe_secs = ?",
            (symbol, min_tf_secs),
        ).fetchone()[0]
        con.close()
        required = from_secs - burn_in_secs
        if earliest is None:
            violations.append(f"archive has no {symbol} {min_tf_secs}s candles at all")
        elif earliest > required:
            violations.append(
                f"archive coverage for {symbol} {min_tf_secs}s starts at {earliest}, "
                f"required <= {required} (partial data — the run saw less than the requested depth)"
            )

    t = trades(db, run_id)
    eq = equity(db, run_id)

    # Trade sanity + exit-reason vocabulary + window bounds + burn-in.
    for tr in t:
        if tr["exit_reason"] not in EXIT_REASONS:
            violations.append(f"trade {tr['seq']}: unknown exit_reason {tr['exit_reason']!r}")
        if tr["size"] <= 0:
            violations.append(f"trade {tr['seq']}: non-positive size")
        if tr["ts_close_secs"] < from_secs - 60:
            violations.append(f"trade {tr['seq']}: closed before window start")
        if tr["ts_close_secs"] > to_secs + 60:
            violations.append(f"trade {tr['seq']}: closed after window end")
        if tr["direction"] == "LONG" and tr["exit_price"] <= 0:
            violations.append(f"trade {tr['seq']}: bad exit price")
        if tr["direction"] == "SHORT" and tr["exit_price"] <= 0:
            violations.append(f"trade {tr['seq']}: bad exit price")

    # Equity curve: never negative; conservation at the end.
    for ts, e in eq:
        if e < 0:
            violations.append(f"equity went negative at {ts}: {e}")
    if eq:
        final_equity = eq[-1][1]
        realized = sum(tr["pnl"] for tr in t)
        drift = final_equity - (capital + realized)
        tolerance = max(1.0, 0.01 * capital)
        if abs(drift) > tolerance:
            violations.append(
                f"equity conservation: final={final_equity:.2f} capital+pnl={capital + realized:.2f} drift={drift:.2f}"
            )

    if violations:
        for v in violations:
            print(f"VIOLATION: {v}")
        return 1
    print(
        f"OK run={run_id} trades={len(t)} equity_points={len(eq)} "
        f"final_equity={eq[-1][1] if eq else capital:.2f}"
    )
    return 0


def compare(args: list[str]) -> int:
    db, a_s, b_s = args
    a, b = trades(db, int(a_s)), trades(db, int(b_s))
    if len(a) != len(b):
        print(f"VIOLATION: trade count differs {len(a)} vs {len(b)}")
        return 1
    for ta, tb in zip(a, b):
        for k in ("ts_close_secs", "direction", "entry_price", "exit_price", "size", "pnl", "exit_reason"):
            if ta[k] != tb[k]:
                print(f"VIOLATION: trade {ta['seq']} differs on {k}: {ta[k]!r} vs {tb[k]!r}")
                return 1
    print(f"OK deterministic: {len(a)} identical trades across runs {a_s} and {b_s}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    cmd = sys.argv[1]
    if cmd == "check":
        sys.exit(check(sys.argv[2:]))
    if cmd == "compare":
        sys.exit(compare(sys.argv[2:]))
    print(__doc__)
    sys.exit(2)
