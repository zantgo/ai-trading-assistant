use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use engine::{config, db, server, analyzer, llm, adapters, automation, performance_evaluator, candle_aggregator, portfolio_risk, strategy_optimizer, workspace, safety, cli};
use shared::models::MarketSnapshot;
use shared::normalized::{NormalizedEvent, NormalizedCandle, SymbolMapper};
use shared::indicators::DivergenceDetector;
use engine::sr_engine::SrRoleTracker;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let web_mode = args.iter().any(|a| a == "--web" || a == "--gui");
    let cli_mode = args.iter().any(|a| a == "--cli");

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

    db::check_encryption_warning(&db_pool).await;

    let (telemetry_tx, telemetry_rx) = channel::<db::TelemetryMsg>(10000);
    let logger_pool = db_pool.clone();
    let logger_llm = llm_client.clone();

    let logger_handle = tokio::spawn(async move {
        db::run_telemetry_logger(logger_pool, telemetry_rx, logger_llm).await;
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

    let portfolio_risk = Arc::new(portfolio_risk::PortfolioRiskState::default());
    let pair_close_histories = Arc::new(RwLock::new(HashMap::<String, Vec<f64>>::new()));

    let workspace = Arc::new(workspace::Workspace::new(
        app_config.clone(),
        db_pool.clone(),
        symbol_mapper.clone(),
        telemetry_tx.clone(),
        api_key_configured.clone(),
        hl_ws_url.clone(),
    ));

    let app_state = Arc::new(server::AppState {
        pairs: pairs.clone(),
        workspace: workspace.clone(),
        config: app_config.clone(),
        pool: db_pool.clone(),
        llm_client: llm_client.clone(),
        api_key_configured: api_key_configured.clone(),
        symbol_mapper: symbol_mapper.clone(),
        telemetry_tx: telemetry_tx.clone(),
        ws_url: hl_ws_url.clone(),
    });

    let app = server::build_router(app_state.clone());

    let mut handles = Vec::new();
    handles.push(logger_handle);

    if cli_mode {
        println!("🖥️  CLI Mode: Starting interactive console session...");
        println!("   Type 'help' for available commands, 'quit' to exit.");

        // Start legacy pipeline tasks in background if symbols exist
        if !initial_symbols.is_empty() {
            // (Legacy pair initialization runs below, same as headless mode)
        }

        // Drop the web app — CLI handles all interaction
        drop(app);

        let cli_console = cli::CliConsole::new(
            workspace.clone(),
            db_pool.clone(),
            llm_client.clone(),
        );
        cli_console.run().await;

        println!("👋 CLI session ended. Shutting down...");
        return;
    }

    if web_mode {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .expect("❌ Web Server Setup: Failed to bind port 3000");

        println!("🌐 Web Server Setup: Visualizer Dashboard live at http://127.0.0.1:3000");

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("❌ Web Server Setup: Fatal crash running Axum HTTP server");
        });
        handles.push(server_handle);
    } else {
        println!("🖥️  CLI Mode: Running as headless daemon — Web server disabled (use --web to enable).");
        drop(app);
    }

    for item in &initial_symbols {
        let (exchange, raw_symbol) = item.split_once(':').unwrap_or(("Hyperliquid", item));
        let pair_key = format!("{}-{}", exchange, raw_symbol.to_uppercase());
        let normalized = format!("{}-USD", raw_symbol.to_uppercase());
        println!("🚀 Starting multi-timeframe analysis pipeline for {} ({})...", pair_key, normalized);

        let config_guard = app_config.read().await;
        let pair_cfg = config_guard.pairs.get(&pair_key);
        let default_indicators = config_guard.indicators.clone();

        let mid_cfg = pair_cfg
            .map(|p| p.mid_term.clone())
            .unwrap_or_else(|| config::TimeframeConfig::new(60, default_indicators.clone()));
        let long_cfg = pair_cfg
            .map(|p| p.long_term.clone())
            .unwrap_or_else(|| config::TimeframeConfig::new(300, default_indicators.clone()));
        let macro_cfg = pair_cfg
            .and_then(|p| p.macro_term.clone())
            .unwrap_or_else(|| config::TimeframeConfig::new(
                config_guard.macro_timeframe.duration_seconds,
                default_indicators.clone(),
            ));
        let supermacro_cfg = pair_cfg
            .and_then(|p| p.supermacro_term.clone())
            .unwrap_or_else(|| config::TimeframeConfig::new(
                config_guard.supermacro_timeframe.duration_seconds,
                default_indicators.clone(),
            ));
        let fib_config = config_guard.fibonacci.clone();
        drop(config_guard);

        let (snapshot_tx, snapshot_rx) = channel::<NormalizedEvent>(500);
        let cancel = CancellationToken::new();

        let (mid_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
        let (long_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
        let (macro_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);
        let (supermacro_broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(200);

        let mid_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(mid_cfg.candles.analysis_limit)));
        let long_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(long_cfg.candles.analysis_limit)));
        let macro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(macro_cfg.candles.analysis_limit)));
        let supermacro_history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(supermacro_cfg.candles.analysis_limit)));

        let mid_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
        let long_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
        let macro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));
        let supermacro_latest = Arc::new(RwLock::new(None::<MarketSnapshot>));

        let pair = Arc::new(analyzer::ActivePair {
            symbol: raw_symbol.to_uppercase(),
            mid: analyzer::TimeframePipeline {
                history: mid_history.clone(),
                broadcast_tx: mid_broadcast_tx.clone(),
                latest_snapshot: mid_latest.clone(),
                timeframe_secs: 60,
                timeframe_label: "Mid",
                divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
                sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
                fibonacci: fib_config.clone(),
            },
            long: analyzer::TimeframePipeline {
                history: long_history.clone(),
                broadcast_tx: long_broadcast_tx.clone(),
                latest_snapshot: long_latest.clone(),
                timeframe_secs: 300,
                timeframe_label: "Long",
                divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
                sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
                fibonacci: fib_config.clone(),
            },
            r#macro: analyzer::TimeframePipeline {
                history: macro_history.clone(),
                broadcast_tx: macro_broadcast_tx.clone(),
                latest_snapshot: macro_latest.clone(),
                timeframe_secs: macro_cfg.candles.duration_seconds,
                timeframe_label: "Macro",
                divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
                sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
                fibonacci: fib_config.clone(),
            },
            supermacro: analyzer::TimeframePipeline {
                history: supermacro_history.clone(),
                broadcast_tx: supermacro_broadcast_tx.clone(),
                latest_snapshot: supermacro_latest.clone(),
                timeframe_secs: supermacro_cfg.candles.duration_seconds,
                timeframe_label: "SuperMacro",
                divergence_detector: Arc::new(tokio::sync::Mutex::new(DivergenceDetector::new(20))),
                sr_tracker: Arc::new(tokio::sync::Mutex::new(SrRoleTracker::new(0.003))),
                fibonacci: fib_config.clone(),
            },
            snapshot_tx: snapshot_tx.clone(),
            cancel: cancel.clone(),
        });

        pairs.write().await.insert(pair_key.clone(), Arc::clone(&pair));

        // Four pipeline channels from the event router
        let (mid_chan_tx, mid_chan_rx) = channel::<NormalizedEvent>(200);
        let (long_chan_tx, long_chan_rx) = channel::<NormalizedEvent>(200);
        let (macro_chan_tx, macro_chan_rx) = channel::<NormalizedEvent>(200);
        let (supermacro_chan_tx, supermacro_chan_rx) = channel::<NormalizedEvent>(200);

        // Event router
        let router_symbol = raw_symbol.to_uppercase();
        let router_cancel = cancel.clone();
        handles.push(tokio::spawn(async move {
            analyzer::run_event_router(
                snapshot_rx,
                mid_chan_tx,
                long_chan_tx,
                macro_chan_tx,
                supermacro_chan_tx,
                router_symbol,
                router_cancel,
            ).await;
        }));

        // Four concurrent pipeline tasks
        let supermacro_secs = supermacro_cfg.candles.duration_seconds;
        let macro_secs = macro_cfg.candles.duration_seconds;
        let (candle_fwd_tx, mut candle_fwd_rx) = tokio::sync::mpsc::unbounded_channel::<NormalizedCandle>();

        let pipeline_specs = [
            (mid_chan_rx, mid_cfg.clone(), mid_history.clone(), mid_latest.clone(), "Mid", 60u64, mid_broadcast_tx.clone(), pair.mid.divergence_detector.clone(), Some(candle_fwd_tx.clone())),
            (long_chan_rx, long_cfg.clone(), long_history.clone(), long_latest.clone(), "Long", 300u64, long_broadcast_tx.clone(), pair.long.divergence_detector.clone(), None),
            (macro_chan_rx, macro_cfg, macro_history.clone(), macro_latest.clone(), "Macro", macro_secs, macro_broadcast_tx.clone(), pair.r#macro.divergence_detector.clone(), None),
            (supermacro_chan_rx, supermacro_cfg, supermacro_history.clone(), supermacro_latest.clone(), "SuperMacro", supermacro_secs, supermacro_broadcast_tx.clone(), pair.supermacro.divergence_detector.clone(), None),
        ];

        for (rx, tf_cfg, hist, snap, label, tf_secs, bcast, div_det, candle_fwd) in pipeline_specs {
            let a_symbol = raw_symbol.to_uppercase();
            let a_pair_key = pair_key.clone();
            let a_telemetry = telemetry_tx.clone();
            let a_cancel = cancel.clone();
            let a_fib = fib_config.clone();
            handles.push(tokio::spawn(async move {
                analyzer::run_single(
                    rx,
                    a_telemetry,
                    bcast,
                    tf_cfg,
                    a_fib,
                    div_det,
                    hist,
                    snap,
                    a_symbol,
                    a_pair_key,
                    tf_secs,
                    label,
                    a_cancel,
                    candle_fwd,
                ).await;
            }));
        }

        // Candle aggregator: bridge 1m completed candles into 4h/1d macro candles
        let (candle_bcast_tx, candle_bcast_rx) = tokio::sync::broadcast::channel::<NormalizedCandle>(1200);
        handles.push(tokio::spawn(async move {
            loop {
                match candle_fwd_rx.recv().await {
                    Some(candle) => {
                        let _ = candle_bcast_tx.send(candle);
                    }
                    None => break,
                }
            }
        }));

        let (agg_4h_tx, mut agg_4h_rx) = tokio::sync::mpsc::channel::<candle_aggregator::AggregatedCandle>(200);
        let (agg_1d_tx, mut agg_1d_rx) = tokio::sync::mpsc::channel::<candle_aggregator::AggregatedCandle>(200);
        let agg_symbol = raw_symbol.to_uppercase();
        handles.push(candle_aggregator::spawn_candle_aggregator(
            agg_symbol.clone(),
            candle_bcast_rx,
            agg_4h_tx,
            agg_1d_tx,
        ));

        let logger_agg_symbol = agg_symbol;
        let logger_agg_telemetry = telemetry_tx.clone();
        handles.push(tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(c4h) = agg_4h_rx.recv() => {
                        let _ = logger_agg_telemetry.send(db::TelemetryMsg::ConsoleLog(format!(
                            "🕯️  [{}] 4h Candle Aggregated | Close: ${:.4} | Sources: {}",
                            logger_agg_symbol, c4h.candle.close, c4h.source_count
                        ))).await;
                    }
                    Some(c1d) = agg_1d_rx.recv() => {
                        let _ = logger_agg_telemetry.send(db::TelemetryMsg::ConsoleLog(format!(
                            "🕯️  [{}] 1d Candle Aggregated | Close: ${:.4} | Sources: {}",
                            logger_agg_symbol, c1d.candle.close, c1d.source_count
                        ))).await;
                    }
                    else => break,
                }
            }
        }));

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
            mid_history: mid_history.clone(),
            long_history: long_history.clone(),
            macro_history: macro_history.clone(),
            supermacro_history: supermacro_history.clone(),
            mid_latest: mid_latest.clone(),
            long_latest: long_latest.clone(),
            macro_latest: macro_latest.clone(),
            supermacro_latest: supermacro_latest.clone(),
            config: app_config.clone(),
            pool: db_pool.clone(),
            llm_client: llm_client.clone(),
            telemetry_tx: telemetry_tx.clone(),
            cancel: cancel.clone(),
            api_key_configured: api_key_configured.clone(),
            portfolio_risk: portfolio_risk.clone(),
            pair_close_histories: pair_close_histories.clone(),
            safety: Arc::new(safety::SafetyManager::new(3, 5, 8, 30.0)),
            intervals: {
                let cfg = app_config.read().await;
                cfg.intervals.clone()
            },
            next_interval_override: Arc::new(RwLock::new(None)),
        };
        handles.push(tokio::spawn(async move {
            automation::run_pair_automation_loop(auto_ctx).await;
        }));
    }

    let eval_cancel = CancellationToken::new();
    let eval_pool = db_pool.clone();
    let eval_cancel1 = eval_cancel.clone();
    handles.push(tokio::spawn(async move {
        performance_evaluator::run_performance_evaluator(
            performance_evaluator::EvaluatorConfig {
                pool: eval_pool,
                cancel: eval_cancel1,
                eval_interval_secs: 300,
            },
        ).await;
    }));

    handles.push(tokio::spawn(async move {
        strategy_optimizer::run_strategy_optimizer(
            strategy_optimizer::OptimizerConfig {
                pool: db_pool,
                cancel: eval_cancel,
                interval_secs: 3600,
            },
        ).await;
    }));

    let _ = futures_util::future::join_all(handles).await;
}
