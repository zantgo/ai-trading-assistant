use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{RwLock, mpsc};
use sqlx::SqlitePool;
use shared::normalized::SymbolMapper;
use crate::config::AppConfig;
use crate::instance::Instance;

#[derive(Debug, Clone, PartialEq)]
pub enum TradingMode {
    Paper,
    Live,
}

impl TradingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradingMode::Paper => "paper",
            TradingMode::Live => "live",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Currency {
    USDT,
    USDC,
}

impl Currency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::USDT => "USDT",
            Currency::USDC => "USDC",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExchangeChoice {
    Hyperliquid,
}

impl ExchangeChoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeChoice::Hyperliquid => "Hyperliquid",
        }
    }
}

pub struct SessionState {
    pub active: AtomicBool,
    pub trading_mode: RwLock<Option<TradingMode>>,
    pub base_currency: RwLock<Option<Currency>>,
    pub exchange: RwLock<Option<ExchangeChoice>>,
    pub initial_capital: RwLock<Option<f64>>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            trading_mode: RwLock::new(None),
            base_currency: RwLock::new(None),
            exchange: RwLock::new(None),
            initial_capital: RwLock::new(None),
        }
    }
}

pub struct Workspace {
    pub instances: Arc<RwLock<HashMap<String, Arc<Instance>>>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub session: SessionState,
    pub pool: SqlitePool,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
    pub api_key_configured: Arc<AtomicBool>,
    pub ws_url: String,
}

impl Workspace {
    pub fn new(
        config: Arc<RwLock<AppConfig>>,
        pool: SqlitePool,
        symbol_mapper: Arc<SymbolMapper>,
        telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
        api_key_configured: Arc<AtomicBool>,
        ws_url: String,
    ) -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            config,
            session: SessionState::new(),
            pool,
            symbol_mapper,
            telemetry_tx,
            api_key_configured,
            ws_url,
        }
    }

    pub async fn max_instances(&self) -> usize {
        self.config.read().await.workspace.max_instances
    }

    pub async fn instance_count(&self) -> usize {
        self.instances.read().await.len()
    }

    pub async fn init_session(
        &self,
        trading_mode: TradingMode,
        currency: Currency,
        exchange: ExchangeChoice,
        initial_capital: f64,
    ) -> Result<(), String> {
        if trading_mode == TradingMode::Live {
            return Err("Live trading is not yet available. Please select Paper Trading.".to_string());
        }
        if initial_capital <= 0.0 {
            return Err("Initial capital must be greater than 0.".to_string());
        }
        if currency != Currency::USDT {
            return Err("Only USDT is currently supported as base currency.".to_string());
        }
        if exchange != ExchangeChoice::Hyperliquid {
            return Err("Only Hyperliquid is currently available.".to_string());
        }

        *self.session.trading_mode.write().await = Some(trading_mode);
        *self.session.base_currency.write().await = Some(currency);
        *self.session.exchange.write().await = Some(exchange);
        *self.session.initial_capital.write().await = Some(initial_capital);
        self.session.active.store(true, std::sync::atomic::Ordering::Relaxed);

        println!("✅ Session initialized: Paper Trading, {:.2} USDT on Hyperliquid", initial_capital);
        Ok(())
    }

    pub async fn quit_session(&self) -> Result<(), String> {
        println!("🛑 Initiating graceful shutdown of all instances...");

        // Collect all instance IDs and cancel them
        let instance_ids: Vec<String> = {
            let instances = self.instances.read().await;
            instances.values().map(|i| i.id.clone()).collect()
        };

        for instance_id in &instance_ids {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.values().find(|i| &i.id == instance_id) {
                // Close any open paper positions at current market price
                let symbol = instance.symbol();
                let pos = crate::db::paper_get_active_position(&instance.pool, &symbol).await;
                if pos.is_some() {
                    let exit_price = 0.0; // 0 signals "use current market price"
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64;
                    let _ = self.telemetry_tx.send(crate::db::TelemetryMsg::PaperClosePosition {
                        symbol: symbol.clone(),
                        exit_price,
                        exit_timestamp: now,
                        trigger: "SESSION_QUIT".to_string(),
                    }).await;
                }
                instance.cancel.cancel();
            }
        }

        // Brief wait for tasks to wind down
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Clear the instance registry
        self.instances.write().await.clear();

        // Reset session state
        self.session.active.store(false, std::sync::atomic::Ordering::Relaxed);
        *self.session.trading_mode.write().await = None;
        *self.session.base_currency.write().await = None;
        *self.session.exchange.write().await = None;
        *self.session.initial_capital.write().await = None;

        println!("✅ Session terminated. All instances stopped.");
        Ok(())
    }
}
