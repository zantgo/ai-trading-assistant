#!/usr/bin/env bash

# ==============================================================================
# Trading Platform - Workspace Management Script
# ==============================================================================
#
# Workspace architecture: see docs/conceptual-foundations/01-06-crate-layout-
# and-cycles.md for the canonical crate inventory, dependency graph, and
# cycle-breaking design rationale (single source of truth).
# Configuration: `config.toml` (canonical; legacy `config.json` accepted by
# config-models::load_config() as a fallback).
# Binary: `cargo run --bin execution-daemon -- --web`
# ==============================================================================

set -euo pipefail

# Configuration
LOG_FILE="engine.log"
FRONTEND_DIR="ui"
PID_FILE=".engine.pid"

show_help() {
    echo "Trading Platform - CLI Management Tool"
    echo "Usage: ./manage.sh [command]"
    echo ""
    echo "Commands:"
    echo "  build              Compile frontend assets and verify cargo workspace compiles"
    echo "  run                Run the engine in the foreground with live logs (web mode)"
    echo "  run-silent         Run the engine in the background, redirecting logs to $LOG_FILE"
    echo "  run-headless       Run in headless mode (no Welcome Gate, auto-spawns instances from config.toml, API server active for monitoring)"
    echo "  stop               Stop any background engine instance currently running"
    echo "  status             Check if the engine is running (and print process info)"
    echo "  test               Run all test suites (core → engine → ui)"
    echo "  test-core          Pure indicator math + serialization (shared crate)"
    echo "  test-engine        DB, server, failover (engine crate)"
    echo "  test-engine-full   Engine suite including load/stress test"
    echo "  test-ui            Svelte 5 visual state & component tests"
    echo "  test-indicators    Per-indicator pipeline e2e with console reporting"
    echo "  test-property      Generative property tests across indicators"
    echo "  test-doc           Documentation corpus consistency checks (Phases 8/9/10 gate)"
    echo "  clean              Delete build targets, dependencies, and temporary locks"
    echo "  destroy            Stop the engine, run clean, and permanently delete telemetry.db"
    echo "  help               Show this helper documentation"
    echo ""
}

build() {
    echo "📦 Building Svelte 5 Frontend..."
    cd "$FRONTEND_DIR"
    bun install
    bun run build
    cd - > /dev/null

    echo "🦀 Verifying Rust Workspace Compilation..."
    cargo check
    echo "✅ Build completed successfully."
}

    run_foreground() {
    if [ ! -d "$FRONTEND_DIR/dist" ]; then
        echo "⚠️  Frontend build missing. Triggering compilation first..."
        build
    fi
    echo "🚀 Starting Trading Platform in the foreground..."
    cargo run --bin execution-daemon -- --web
}

run_silent() {
    if [ ! -d "$FRONTEND_DIR/dist" ]; then
        echo "⚠️  Frontend build missing. Triggering compilation first..."
        build
    fi

    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if kill -0 "$PID" 2>/dev/null; then
            echo "⚠️  Engine is already running in the background (PID: $PID)."
            exit 0
        fi
    fi

    echo "🚀 Starting Trading Platform in the background..."
    echo "📝 Logs will be written to: $LOG_FILE"

    # Run cargo in background and record PID
    nohup cargo run --bin execution-daemon -- --web > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"
    echo "✅ Engine running under PID: $!"
}

run_headless() {
    if [ ! -d "$FRONTEND_DIR/dist" ]; then
        echo "⚠️  Frontend build missing (needed for monitoring). Triggering compilation first..."
        build
    fi

    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if kill -0 "$PID" 2>/dev/null; then
            echo "⚠️  Engine is already running in the background (PID: $PID)."
            exit 0
        fi
    fi

    echo "🚀 Starting Trading Platform in HEADLESS mode..."
    echo "   🔧 No Welcome Gate — session auto-initialised from config.toml"
    echo "   📡 Instances auto-spawned from workspace.instances[]"
    echo "   🌐 API server on port 3000 for monitoring"
    echo "📝 Logs will be written to: $LOG_FILE"

    nohup cargo run --bin execution-daemon -- --mode headless > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"
    echo "✅ Engine running in headless mode under PID: $!"
}

stop_instance() {
    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        echo "🛑 Stopping background instance (PID: $PID)..."
        if kill "$PID" 2>/dev/null; then
            rm -f "$PID_FILE"
            echo "✅ Engine stopped."
        else
            echo "⚠️  Process $PID not found. Cleaning stale PID file."
            rm -f "$PID_FILE"
        fi
    else
        # Fallback to kill cargo/engine processes on this port if no pid file is present
        PORT_PID=$(lsof -t -i:3000 || true)
        if [ -n "$PORT_PID" ]; then
            echo "🛑 Found engine running on port 3000 (PID: $PORT_PID). Stopping..."
            kill "$PORT_PID"
            echo "✅ Engine stopped."
        else
            echo "ℹ️  No running instances detected."
        fi
    fi
}

check_status() {
    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if kill -0 "$PID" 2>/dev/null; then
            echo "🟢 Engine status: RUNNING (PID: $PID)"
            echo "📝 Log file size: $(du -sh "$LOG_FILE" | cut -f1)"
            return 0
        fi
    fi

    PORT_PID=$(lsof -t -i:3000 || true)
    if [ -n "$PORT_PID" ]; then
        echo "🟢 Engine status: RUNNING on port 3000 (PID: $PORT_PID)"
        return 0
    fi

    echo "🔴 Engine status: STOPPED"
}

run_tests() {
    local failures=0
    echo "═══════════════════════════════════════════════════════════"
    echo "  STAGE 1/3: TEST-CORE — Pure math, indicators, serialization"
    echo "═══════════════════════════════════════════════════════════"
    test_core || { ((failures++)); echo "❌ TEST-CORE failed"; }
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  STAGE 2/3: TEST-ENGINE — DB + server + e2e"
    echo "═══════════════════════════════════════════════════════════"
    test_engine || { ((failures++)); echo "❌ TEST-ENGINE failed"; }
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  STAGE 3/3: TEST-UI — Svelte 5 components, state, snapshots"
    echo "═══════════════════════════════════════════════════════════"
    test_ui || { ((failures++)); echo "❌ TEST-UI failed"; }
    echo ""
    if [ $failures -eq 0 ]; then
        echo "✅ All 3 test suites passed"
    else
        echo "❌ $failures test suite(s) failed"
        return 1
    fi
}

test_core() {
    echo "🦀 TEST-CORE: Running core-domain + market-analyzer + config-models..."
    cargo test -p core-domain -p market-analyzer -p config-models
}

test_engine() {
    echo "🦀 TEST-ENGINE: Running database-storage + api-gateway + portfolio-supervisor + performance-analytics + network-adapters + execution-daemon..."
    cargo test -p database-storage -p api-gateway -p portfolio-supervisor -p performance-analytics -p network-adapters -p execution-daemon
}

test_indicators() {
    echo "🦀 TEST-INDICATORS: Per-indicator pipeline e2e with terminal console reporting..."
    echo "    Each indicator (37 candle-based) is exercised through calculator → normalizer → signal deriver → lifecycle builder"
    echo "    with synthesized OHLCV candles in 4 market patterns (uptrend, downtrend, range, volatile)."
    echo "    Failures surface duplicate (label, kind) signal pairs that would trigger each_key_duplicate in the UI."
    cargo test -p market-analyzer --test indicator_pipeline_e2e -- --nocapture --test-threads=1
}

test_engine_full() {
    echo "🦀 TEST-ENGINE-FULL: Running all engine tests including load/stress..."
    cargo test --workspace -- --include-ignored
}

test_property() {
    echo "🦀 TEST-PROPERTY: Running generative property tests across all indicators..."
    cargo test -p market-analyzer --test property_ema_sma --test property_rsi --test property_macd --test property_adx --test property_bollinger_atr --test property_squeeze --test property_bbwp --test property_fibonacci --test property_divergence --test property_patterns
}

test_ui() {
    echo "🧪 TEST-UI: Running Svelte 5 frontend Vitest tests..."
    cd "$FRONTEND_DIR"
    bun run test
    cd - > /dev/null
}

clean_workspace() {
    echo "🧹 Cleaning cargo workspace targets..."
    cargo clean
    echo "🧹 Removing frontend dependencies and builds..."
    rm -rf "$FRONTEND_DIR/node_modules"
    rm -rf "$FRONTEND_DIR/bun.lock"
    rm -rf "$FRONTEND_DIR/dist"
    rm -f "$PID_FILE"
    rm -f "$LOG_FILE"
    echo "✅ Workspace clean."
}

destroy_all() {
    echo "🛑 Stopping any active background or running instances..."
    stop_instance

    echo "🧹 Executing standard workspace cleanup..."
    clean_workspace

    echo "💥 Permanently deleting SQLite database and journal files..."
    rm -f "telemetry.db"
    rm -f "telemetry.db-journal"
    rm -f "telemetry.db-shm"
    rm -f "telemetry.db-wal"

    # The platform uses config.toml (single source of truth).
    # `./manage.sh destroy` resets config.toml from the bundled
    # config.default.toml template.
    rm -f "config.toml"
    if [ -f "config.default.toml" ]; then
        echo "⚙️  Restoring config.toml from config.default.toml template..."
        cp "config.default.toml" "config.toml"
    else
        echo "❌ Error: config.default.toml is missing! Cannot restore default configuration."
        exit 1
    fi

    echo "✨ Absolutely everything has been purged and destroyed."
}

test_doc() {
    echo "📋 TEST-DOC: Running documentation corpus consistency checks..."
    python3 scripts/check_docs.py
}

# Main routing logic
if [ $# -eq 0 ]; then
    show_help
    exit 0
fi

case "$1" in
    build)
        build
        ;;
    run)
        run_foreground
        ;;
    run-silent)
        run_silent
        ;;
    run-headless)
        run_headless
        ;;
    stop)
        stop_instance
        ;;
    status)
        check_status
        ;;
    test)
        run_tests
        ;;
    test-core)
        test_core
        ;;
    test-engine)
        test_engine
        ;;
    test-indicators)
        test_indicators
        ;;
    test-engine-full)
        test_engine_full
        ;;
    test-property)
        test_property
        ;;
    test-ui)
        test_ui
        ;;
    test-doc)
        test_doc
        ;;
    clean)
        clean_workspace
        ;;
    destroy)
        destroy_all
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "❌ Error: Unknown command '$1'"
        show_help
        exit 1
        ;;
esac
