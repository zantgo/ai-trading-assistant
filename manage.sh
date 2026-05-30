#!/usr/bin/env bash

# ==============================================================================
# AI Trading Assistant - Workspace Management Script
# ==============================================================================

set -euo pipefail

# Configuration
LOG_FILE="engine.log"
FRONTEND_DIR="crates/engine/frontend"
PID_FILE=".engine.pid"

show_help() {
    echo "AI Trading Assistant - CLI Management Tool"
    echo "Usage: ./manage.sh [command]"
    echo ""
    echo "Commands:"
    echo "  build         Compile frontend assets and verify cargo workspace compiles"
    echo "  run           Run the engine in the foreground with live logs"
    echo "  run-silent    Run the engine in the background, redirecting logs to $LOG_FILE"
    echo "  stop          Stop any background engine instance currently running"
    echo "  status        Check if the engine is running (and print process info)"
    echo "  test          Run all workspace tests (both Rust and Svelte/Vitest)"
    echo "  test-rust     Run Rust unit and integration tests only"
    echo "  test-ui       Run Svelte 5 frontend unit tests only"
    echo "  clean         Delete build targets, node_modules, and temporary locks"
    echo "  destroy       Stop the engine, run clean, and permanently delete telemetry.db"
    echo "  help          Show this helper documentation"
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
    cargo run
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
    nohup cargo run > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"
    echo "✅ Engine running under PID: $!"
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
    test_rust
    test_ui
}

test_rust() {
    echo "🦀 Running Rust Workspace Tests..."
    cargo test
}

test_ui() {
    echo "🧪 Running Frontend Vitest Tests..."
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
    stop)
        stop_instance
        ;;
    status)
        check_status
        ;;
    test)
        run_tests
        ;;
    test-rust)
        test_rust
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
