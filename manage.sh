#!/usr/bin/env bash

# ==============================================================================
# AI Trading Assistant - Workspace Management Script
# ==============================================================================

set -euo pipefail

# Configuration
LOG_FILE="engine.log"
FRONTEND_DIR="crates/frontend"
PID_FILE=".engine.pid"

show_help() {
    echo "AI Trading Assistant - CLI Management Tool"
    echo "Usage: ./manage.sh [command]"
    echo ""
    echo "Commands:"
    echo "  build              Compile frontend assets and verify cargo workspace compiles"
    echo "  run                Run the engine in the foreground with live logs"
    echo "  run-silent         Run the engine in the background, redirecting logs to $LOG_FILE"
    echo "  cli                Launch the interactive CLI trading console"
    echo "  stop               Stop any background engine instance currently running"
    echo "  status             Check if the engine is running (and print process info)"
    echo "  test               Run all 5 test suites (core → correlation → e2e → engine-full → ui)"
    echo "  test-core          Pure indicator math + serialization (shared crate, 154 tests)"
    echo "  test-engine        DB, paper trading, server, failover (engine crate, 69 tests)"
    echo "  test-engine-full   Engine suite including load/stress test (70 tests)"
    echo "  test-ui            Svelte 5 visual state & component tests (24 tests)"
    echo "  test-property      Generative property tests across 12 indicators (38 tests)"
    echo "  test-correlation   Pearson correlation + drawdown validation (15 tests)"
    echo "  test-e2e           End-to-end analytical loop + history endpoint (2 tests)"
    echo "  test-load          Multi-pair load/stress test only (1 test, manual run)"
    echo "  clean              Delete build targets, node_modules, and temporary locks"
    echo "  destroy            Stop the engine, run clean, and permanently delete telemetry.db"
    echo "  help               Show this helper documentation"
    echo ""
}

check_env() {
    if [ ! -f ".env" ]; then
        echo "❌ Error: .env file missing in workspace root."
        echo "   Copy .env.example to .env and configure your DEEPSEEK_API_KEY."
        exit 1
    fi
}

build() {
    echo "📦 Building Svelte 5 Frontend..."
    cd "$FRONTEND_DIR"
    npm install
    npm run build
    cd - > /dev/null

    echo "🦀 Verifying Rust Workspace Compilation..."
    cargo check
    echo "✅ Build completed successfully."
}

run_foreground() {
    check_env
    if [ ! -d "$FRONTEND_DIR/dist" ]; then
        echo "⚠️  Frontend build missing. Triggering compilation first..."
        build
    fi
    echo "🚀 Starting AI Trading Assistant in the foreground..."
    cargo run -- --web
}

run_silent() {
    check_env
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

    echo "🚀 Starting AI Trading Assistant in the background..."
    echo "📝 Logs will be written to: $LOG_FILE"
    
    # Run cargo in background and record PID
    nohup cargo run -- --web > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"
    echo "✅ Engine running under PID: $!"
}

cli_mode() {
    check_env
    echo "🖥️  Starting AI Trading Assistant — CLI Console..."
    cargo run -- --cli
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
    echo "  STAGE 1/5: TEST-CORE — Pure math, indicators, serialization"
    echo "═══════════════════════════════════════════════════════════"
    test_core || { ((failures++)); echo "❌ TEST-CORE failed"; }
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  STAGE 2/5: TEST-CORRELATION — Pearson + drawdown validation"
    echo "═══════════════════════════════════════════════════════════"
    test_correlation || { ((failures++)); echo "❌ TEST-CORRELATION failed"; }
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  STAGE 3/5: TEST-E2E — End-to-end analytical loop + history"
    echo "═══════════════════════════════════════════════════════════"
    test_e2e || { ((failures++)); echo "❌ TEST-E2E failed"; }
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  STAGE 4/5: TEST-ENGINE-FULL — DB + paper trade + server + load"
    echo "═══════════════════════════════════════════════════════════"
    test_engine_full || { ((failures++)); echo "❌ TEST-ENGINE-FULL failed"; }
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  STAGE 5/5: TEST-UI — Svelte 5 components, state, snapshots"
    echo "═══════════════════════════════════════════════════════════"
    test_ui || { ((failures++)); echo "❌ TEST-UI failed"; }
    echo ""
    if [ $failures -eq 0 ]; then
        echo "✅ All 5 test suites passed"
    else
        echo "❌ $failures test suite(s) failed"
        return 1
    fi
}

test_core() {
    echo "🦀 TEST-CORE: Running shared crate tests (indicators + serialization)..."
    cargo test -p shared
}

test_engine() {
    echo "🦀 TEST-ENGINE: Running engine integration tests (DB + paper trading + server)..."
    cargo test -p engine
}

test_engine_full() {
    echo "🦀 TEST-ENGINE-FULL: Running engine tests including load/stress..."
    cargo test -p engine -- --include-ignored
}

test_property() {
    echo "🦀 TEST-PROPERTY: Running generative property tests across all indicators..."
    cargo test -p shared --test property_ema_sma --test property_rsi --test property_macd --test property_adx --test property_bollinger_atr --test property_squeeze --test property_bbwp --test property_fibonacci --test property_divergence --test property_patterns
}

test_correlation() {
    echo "🦀 TEST-CORRELATION: Running Pearson correlation + drawdown validation..."
    cargo test -p engine --test portfolio_risk_tests
}

test_e2e() {
    echo "🦀 TEST-E2E: Running end-to-end analytical loop + history endpoint..."
    cargo test -p engine --test system_e2e_analysis
}

test_load() {
    echo "🦀 TEST-LOAD: Running multi-pair load/stress test..."
    cargo test -p engine --test load_max_pairs -- --ignored
}

test_ui() {
    echo "🧪 TEST-UI: Running Svelte 5 frontend Vitest tests..."
    cd "$FRONTEND_DIR"
    npm run test
    cd - > /dev/null
}

clean_workspace() {
    echo "🧹 Cleaning cargo workspace targets..."
    cargo clean
    echo "🧹 Removing node_modules and frontend builds..."
    rm -rf "$FRONTEND_DIR/node_modules"
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

    if [ -f "config.default.toml" ]; then
        echo "⚙️  Restoring config.toml from config.default.toml template..."
        cp "config.default.toml" "config.toml"
    else
        echo "❌ Error: config.default.toml is missing! Cannot restore default configuration."
        exit 1
    fi

    echo "✨ Absolutely everything has been purged and destroyed."
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
    cli)
        cli_mode
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
    test-engine-full)
        test_engine_full
        ;;
    test-property)
        test_property
        ;;
    test-correlation)
        test_correlation
        ;;
    test-e2e)
        test_e2e
        ;;
    test-load)
        test_load
        ;;
    test-ui)
        test_ui
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
