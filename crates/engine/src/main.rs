use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use engine::{config, db, server, analyzer, llm, adapters, automation, performance_evaluator};
use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle, SymbolMapper};

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    match dotenvy::dotenv() {
        Ok(_) => println!("✅ Loaded .env configuration."),
        Err(e) => {
            eprintln!("⚠️  No .env file found: {}", e);
            eprintln!("   Create a .env file at the project root with: DEEPSEEK_API_KEY=sk-...");
            eprintln!("   The dashboard will run, but AI features require a valid key.");
        }
    }

    println!("⚙️  AI Trading Assistant: Loading Master Configuration...");
    let mut app_config = config::load_config();
    app_config.pairs = config::load_pairs();
    println!("✅ Configuration Loaded: Initial pairs: {:?} ({} pair-specific configs)", app_config.symbols, app_config.pairs.len());
    let app_config = Arc::new(RwLock::new(app_config));
    let initial_symbols = app_config.read().await.symbols.clone();
    println!("✅ Configuration Loaded: Initial pairs: {:?}", initial_symbols);

    let (llm_client, key_present) = llm::LlmClient::from_env();
    let llm_client = Arc::new(RwLock::new(llm_client));
    let api_key_configured = Arc::new(AtomicBool::new(false));

    if key_present {
        println!("🔑 Validating DeepSeek API key...");
        let llm = llm_client.read().await;
        match llm.validate_key().await {
            Ok(()) => {
                println!("✅ Key validated successfully.");
                api_key_configured.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Err(e) => {
                eprintln!("⚠️  API Key Validation Failed: {}. You can configure it manually in the UI.", e);
            }
        }
    } else {
        eprintln!("⚠️  No API key found. AI analysis will fall back to local heuristics. Configure via the UI config panel.");
    }

    println!("🗄️  Initializing local SQLite telemetry database...");
    let db_pool = db::init_db().await;
    println!("✅ Database Setup: Connected to local telemetry.db file and verified schema.");

    let (telemetry_tx, telemetry_rx) = channel::<db::TelemetryMsg>(10000);
    let logger_pool = db_pool.clone();

    let logger_handle = tokio::spawn(async move {
        db::run_telemetry_logger(logger_pool, telemetry_rx).await;
    });

    let symbol_mapper = Arc::new(SymbolMapper::new());
    for item in &initial_symbols {
        let (exchange_str, raw_symbol) = item.split_once(':').unwrap_or(("Hyperliquid", item));
        let exchange_enum = match exchange_str {
            "Hyperliquid" => shared::normalized::Exchange::Hyperliquid,
            _ => continue,
        };
        let normalized = format!("{}-USD", raw_symbol.to_uppercase());
        symbol_mapper.register(exchange_enum, &raw_symbol.to_uppercase(), &normalized).await;
        println!("🧭 Symbol Mapper: Registered active mapping: {} -> {} ({})", raw_symbol, normalized, exchange_str);
    }

    let hl_ws_url = app_config.read().await.hyperliquid.ws_url.clone();
    println!("📡 Hyperliquid WS endpoint: {}", hl_ws_url);

    let pairs: Arc<RwLock<HashMap<String, Arc<analyzer::ActivePair>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let app_state = Arc::new(server::AppState {
        pairs: pairs.clone(),
        config: app_config.clone(),
        pool: db_pool.clone(),
        llm_client: llm_client.clone(),
        api_key_configured: api_key_configured.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        ws_url: hl_ws_url.clone(),
    });

    let app = server::build_router(app_state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("❌ Web Server Setup: Failed to bind port 3000");

    println!("🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000");

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("❌ Web Server Setup: Fatal crash running Axum HTTP server");
    });

    let mut handles = Vec::new();
    handles.push(logger_handle);

    for item in &initial_symbols {
        let (exchange, raw_symbol) = item.split_once(':').unwrap_or(("Hyperliquid", item));
        let pair_key = format!("{}-{}", exchange, raw_symbol.to_uppercase());
        let normalized = format!("{}-USD", raw_symbol.to_uppercase());
        println!("🚀 Starting multi-timeframe analysis pipeline for {} ({})...", pair_key, normalized);

        let config_guard = app_config.read().await;
        let pair_cfg = config_guard.pairs.get(&pair_key);
        let default_indicators = config_guard.indicators.clone();

        let short_cfg = pair_cfg
            .map(|p| p.short_term.clone())
            .unwrap_or_else(|| config::TimeframeConfig::new(15, default_indicators.clone()));
        let mid_cfg = pair_cfg
            .map(|p| p.mid_term.clone())
            .unwrap_or_else(|| config::TimeframeConfig::new(60, default_indicators.clone()));
        let long_cfg = pair_cfg
            .map(|p| p.long_term.clone())
            .unwrap_or_else(|| config::TimeframeConfig::new(300, default_indicators.clone()));
        drop(config_guard);

        let (snapshot_tx, snapshot_rx) = channel::<NormalizedEvent>(500);
        let cancel = CancellationToken::new();

        let (short_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
        let (mid_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
        let (long_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);

        let short_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(short_cfg.candles.analysis_limit)));
        let mid_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(mid_cfg.candles.analysis_limit)));
        let long_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(long_cfg.candles.analysis_limit)));

        let short_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
        let mid_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
        let long_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

        let pair = Arc::new(analyzer::ActivePair {
            symbol: raw_symbol.to_uppercase(),
            short: analyzer::TimeframePipeline {
                history: short_history.clone(),
                broadcast_tx: short_broadcast_tx.clone(),
                latest_snapshot: short_latest.clone(),
                timeframe_secs: 15,
                timeframe_label: "Short",
            },
            mid: analyzer::TimeframePipeline {
                history: mid_history.clone(),
                broadcast_tx: mid_broadcast_tx.clone(),
                latest_snapshot: mid_latest.clone(),
                timeframe_secs: 60,
                timeframe_label: "Mid",
            },
            long: analyzer::TimeframePipeline {
                history: long_history.clone(),
                broadcast_tx: long_broadcast_tx.clone(),
                latest_snapshot: long_latest.clone(),
                timeframe_secs: 300,
                timeframe_label: "Long",
            },
            snapshot_tx: snapshot_tx.clone(),
            cancel: cancel.clone(),
        });

        pairs.write().await.insert(pair_key.clone(), Arc::clone(&pair));

        // Three pipeline channels from the event router
        let (short_chan_tx, short_chan_rx) = channel::<NormalizedEvent>(200);
        let (mid_chan_tx, mid_chan_rx) = channel::<NormalizedEvent>(200);
        let (long_chan_tx, long_chan_rx) = channel::<NormalizedEvent>(200);

        // Event router
        let router_symbol = raw_symbol.to_uppercase();
        let router_cancel = cancel.clone();
        handles.push(tokio::spawn(async move {
            analyzer::run_event_router(
                snapshot_rx,
                short_chan_tx,
                mid_chan_tx,
                long_chan_tx,
                router_symbol,
                router_cancel,
            ).await;
        }));

        // Three concurrent pipeline tasks
        for (rx, tf_cfg, hist, snap, label, tf_secs, bcast) in [
            (short_chan_rx, short_cfg.clone(), short_history.clone(), short_latest.clone(), "Short", 15u64, short_broadcast_tx.clone()),
            (mid_chan_rx, mid_cfg.clone(), mid_history.clone(), mid_latest.clone(), "Mid", 60u64, mid_broadcast_tx.clone()),
            (long_chan_rx, long_cfg, long_history.clone(), long_latest.clone(), "Long", 300u64, long_broadcast_tx.clone()),
        ] {
            let a_symbol = raw_symbol.to_uppercase();
            let a_pair_key = pair_key.clone();
            let a_telemetry = telemetry_tx.clone();
            let a_cancel = cancel.clone();
            handles.push(tokio::spawn(async move {
                analyzer::run_single(
                    rx,
                    a_telemetry,
                    bcast,
                    tf_cfg,
                    hist,
                    snap,
                    a_symbol,
                    a_pair_key,
                    tf_secs,
                    label,
                    a_cancel,
                ).await;
            }));
        }

        // WebSocket adapter
        let ws_symbol = raw_symbol.to_uppercase();
        let ws_tx = snapshot_tx.clone();
        let ws_cancel = cancel.clone();
        let ws_url = hl_ws_url.clone();
        handles.push(tokio::spawn(async move {
            adapters::hyperliquid::run_for_symbol(ws_symbol, ws_tx, ws_cancel, &ws_url).await;
        }));

        // Automation loop
        let auto_ctx = automation::AutomationContext {
            pair_key: pair_key.clone(),
            symbol: raw_symbol.to_uppercase(),
            short_history: short_history.clone(),
            mid_history: mid_history.clone(),
            long_history: long_history.clone(),
            short_latest: short_latest.clone(),
            mid_latest: mid_latest.clone(),
            long_latest: long_latest.clone(),
            config: app_config.clone(),
            pool: db_pool.clone(),
            llm_client: llm_client.clone(),
            telemetry_tx: telemetry_tx.clone(),
            cancel: cancel.clone(),
            api_key_configured: api_key_configured.clone(),
        };
        handles.push(tokio::spawn(async move {
            automation::run_pair_automation_loop(auto_ctx).await;
        }));
    }

    let eval_cancel = CancellationToken::new();
    handles.push(tokio::spawn(async move {
        performance_evaluator::run_performance_evaluator(
            performance_evaluator::EvaluatorConfig {
                pool: db_pool.clone(),
                cancel: eval_cancel,
                eval_interval_secs: 300,
            },
        ).await;
    }));

    handles.push(server_handle);

    let _ = futures_util::future::join_all(handles).await;
}
