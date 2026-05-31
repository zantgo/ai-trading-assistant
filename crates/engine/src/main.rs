use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use engine::{config, db, server, analyzer, llm, adapters};
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

    // Initialize telemetry queue
    let (telemetry_tx, telemetry_rx) = channel::<db::TelemetryMsg>(10000);
    let logger_pool = db_pool.clone();

    // Dedicate a background worker task entirely to logging and disk I/O
    let logger_handle = tokio::spawn(async move {
        db::run_telemetry_logger(logger_pool, telemetry_rx).await;
    });

    // Initialize symbol mapping system
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
        println!("🚀 Starting analysis pipeline for {} ({})...", pair_key, normalized);

        let (snapshot_tx, snapshot_rx) = channel::<NormalizedEvent>(100);
        let (broadcast_tx, _) = tokio::sync::broadcast::channel::<MarketSnapshot>(100);
        let history = Arc::new(RwLock::new(VecDeque::<NormalizedCandle>::with_capacity(100)));
        let cancel = CancellationToken::new();

        let pair = Arc::new(analyzer::ActivePair {
            symbol: pair_key.clone(),
            history: history.clone(),
            broadcast_tx: broadcast_tx.clone(),
            snapshot_tx: snapshot_tx.clone(),
            cancel: cancel.clone(),
        });

        pairs.write().await.insert(pair_key.clone(), Arc::clone(&pair));

        let analyzer_config = app_config.clone();
        let analyzer_telemetry = telemetry_tx.clone();
        let analyzer_history = history.clone();
        let analyzer_broadcast = broadcast_tx.clone();
        let analyzer_cancel = cancel.clone();
        let analyzer_symbol = raw_symbol.to_uppercase();
        let analyzer_pair_key = pair_key.clone();
        handles.push(tokio::spawn(async move {
            analyzer::run_single(
                snapshot_rx,
                analyzer_telemetry,
                analyzer_broadcast,
                analyzer_config,
                analyzer_history,
                analyzer_symbol,
                analyzer_pair_key,
                analyzer_cancel,
            ).await;
        }));

        let ws_symbol = raw_symbol.to_uppercase();
        let ws_tx = snapshot_tx.clone();
        let ws_cancel = cancel.clone();
        let ws_url = hl_ws_url.clone();
        handles.push(tokio::spawn(async move {
            adapters::hyperliquid::run_for_symbol(ws_symbol, ws_tx, ws_cancel, &ws_url).await;
        }));
    }

    handles.push(server_handle);

    let _ = futures_util::future::join_all(handles).await;
}
