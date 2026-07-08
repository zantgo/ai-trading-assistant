use crate::config::AppConfig;
use crate::instance::Instance;
use shared::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

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
    Bitget,
}

impl ExchangeChoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeChoice::Hyperliquid => "Hyperliquid",
            ExchangeChoice::Bitget => "Bitget",
        }
    }

    /// Native exchange symbol for REST/WS requests (perpetual futures).
    ///
    /// - Hyperliquid: the bare coin (e.g. `BTC`); collateral is always USDC.
    /// - Bitget USDT-M futures: `BASEUSDT` (e.g. `BTCUSDT`).
    /// - Bitget USDC-M futures: `BASEUSDC` (e.g. `BTCUSDC`).
    pub fn raw_symbol(&self, base: &str, quote: &Currency) -> String {
        match self {
            ExchangeChoice::Hyperliquid => base.to_string(),
            ExchangeChoice::Bitget => match quote {
                Currency::USDT => format!("{}USDT", base),
                Currency::USDC => format!("{}USDC", base),
            },
        }
    }

    /// Unified internal symbol used across the workspace (e.g. `BTC-USDT`,
    /// `BTC-USDC`). Independent of exchange-native dialects.
    pub fn internal_symbol(&self, base: &str, quote: &Currency) -> String {
        format!("{}-{}", base, quote.as_str())
    }

    /// Bitget V2 mix (perpetual futures) `productType` for the given quote.
    /// Not applicable to Hyperliquid (returns `None`).
    pub fn bitget_product_type(&self, quote: &Currency) -> Option<&'static str> {
        match self {
            ExchangeChoice::Bitget => Some(match quote {
                Currency::USDT => "USDT-FUTURES",
                Currency::USDC => "USDC-FUTURES",
            }),
            ExchangeChoice::Hyperliquid => None,
        }
    }

    /// Whether the given settlement/quote currency is supported for this
    /// exchange's perpetual futures.
    pub fn supports_currency(&self, quote: &Currency) -> bool {
        match self {
            // Hyperliquid perpetuals settle exclusively in USDC.
            ExchangeChoice::Hyperliquid => *quote == Currency::USDC,
            // Bitget offers both USDT-M and USDC-M futures.
            ExchangeChoice::Bitget => matches!(quote, Currency::USDT | Currency::USDC),
        }
    }
}

pub struct SessionState {
    pub active: AtomicBool,
    pub trading_mode: RwLock<Option<TradingMode>>,
    pub base_currency: RwLock<Option<Currency>>,
    pub exchange: RwLock<Option<ExchangeChoice>>,
    pub initial_capital: RwLock<Option<f64>>,
    pub user_name: RwLock<Option<String>>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            trading_mode: RwLock::new(None),
            base_currency: RwLock::new(None),
            exchange: RwLock::new(None),
            initial_capital: RwLock::new(None),
            user_name: RwLock::new(None),
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
    pub bitget_ws_url: String,
}

impl Workspace {
    pub fn new(
        config: Arc<RwLock<AppConfig>>,
        pool: SqlitePool,
        symbol_mapper: Arc<SymbolMapper>,
        telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
        api_key_configured: Arc<AtomicBool>,
        ws_url: String,
        bitget_ws_url: String,
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
            bitget_ws_url,
        }
    }

    pub async fn max_instances(&self) -> usize {
        self.config.read().await.workspace.max_instances
    }

    pub async fn instance_count(&self) -> usize {
        self.instances.read().await.len()
    }

    pub async fn get_all_instances(&self) -> Vec<Arc<Instance>> {
        self.instances.read().await.values().cloned().collect()
    }

    pub async fn get_active_pair(
        &self,
        pair_key: &str,
    ) -> Option<Arc<crate::analyzer::ActivePair>> {
        self.instances
            .read()
            .await
            .get(pair_key)
            .map(|inst| inst.active_pair.clone())
    }

    pub async fn get_instance_by_pair_key(
        &self,
        pair_key: &str,
    ) -> Option<Arc<Instance>> {
        self.instances.read().await.get(pair_key).cloned()
    }

    pub async fn get_instance_by_id(&self, id: &str) -> Option<Arc<Instance>> {
        self.instances
            .read()
            .await
            .values()
            .find(|i| i.id == id)
            .cloned()
    }

    pub async fn init_session(
        &self,
        trading_mode: TradingMode,
        currency: Currency,
        exchange: ExchangeChoice,
        initial_capital: f64,
        user_name: Option<String>,
    ) -> Result<(), String> {
        if trading_mode == TradingMode::Live {
            return Err(
                "Live trading is not available. Please select Paper Trading.".to_string(),
            );
        }
        if initial_capital <= 0.0 {
            return Err("Initial capital must be greater than 0.".to_string());
        }
        if exchange != ExchangeChoice::Hyperliquid && exchange != ExchangeChoice::Bitget {
            return Err("Unsupported exchange selected.".to_string());
        }
        // Enforce the exchange <-> settlement-currency rules for perpetual
        // futures. Hyperliquid settles only in USDC; Bitget supports USDT-M and
        // USDC-M futures.
        if !exchange.supports_currency(&currency) {
            return Err(format!(
                "{} does not support {} settlement. {}",
                exchange.as_str(),
                currency.as_str(),
                match exchange {
                    ExchangeChoice::Hyperliquid =>
                        "Hyperliquid perpetuals settle in USDC only.",
                    ExchangeChoice::Bitget => "Select USDT or USDC.",
                }
            ));
        }

        *self.session.trading_mode.write().await = Some(trading_mode.clone());
        *self.session.base_currency.write().await = Some(currency.clone());
        *self.session.exchange.write().await = Some(exchange.clone());
        *self.session.initial_capital.write().await = Some(initial_capital);
        *self.session.user_name.write().await = user_name.clone();
        self.session
            .active
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Persist profile to config.toml for restart survival
        {
            let mut config = self.config.write().await;
            config.profile.user_name = user_name.clone().filter(|n| !n.trim().is_empty());
            config.profile.session_mode = Some(trading_mode.as_str().to_string());
            config.profile.session_currency = Some(currency.as_str().to_string());
            config.profile.session_exchange = Some(exchange.as_str().to_string());
            config.profile.initial_capital = Some(initial_capital);
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = std::fs::write("config.toml", toml_str);
            }
        }

        {
            let instances: Vec<_> = self.instances.read().await.keys().cloned().collect();
            if instances.len() == 1 {
                let _ = crate::db::paper_set_advanced_config(
                    &self.pool,
                    &instances[0],
                    initial_capital,
                    10.0,
                    false,
                    2.0,
                    20,
                    15,
                    10,
                    false,
                )
                .await;
            }
        }

        println!(
            "✅ Session initialized: Paper Trading, {:.2} {} on {}",
            initial_capital,
            currency.as_str(),
            exchange.as_str(),
        );
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
                    let _ = self
                        .telemetry_tx
                        .send(crate::db::TelemetryMsg::PaperClosePosition {
                            symbol: symbol.clone(),
                            exit_price,
                            exit_timestamp: now,
                            trigger: "SESSION_QUIT".to_string(),
                        })
                        .await;
                }
                instance.cancel.cancel();
            }
        }

        // Brief wait for tasks to wind down
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Clear the instance registry
        self.instances.write().await.clear();

        // Reset session state
        self.session
            .active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        *self.session.trading_mode.write().await = None;
        *self.session.base_currency.write().await = None;
        *self.session.exchange.write().await = None;
        *self.session.initial_capital.write().await = None;
        *self.session.user_name.write().await = None;

        // Clear session fields from config but keep profile name/wallet
        {
            let mut config = self.config.write().await;
            config.profile.session_mode = None;
            config.profile.session_currency = None;
            config.profile.session_exchange = None;
            config.profile.initial_capital = None;
            if let Ok(toml_str) = toml::to_string_pretty(&*config) {
                let _ = std::fs::write("config.toml", toml_str);
            }
        }

        println!("✅ Session terminated. All instances stopped.");
        Ok(())
    }
}
