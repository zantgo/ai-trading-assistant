use crate::config::AppConfig;
use crate::instance::Instance;
use shared::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

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
    pub fn raw_symbol(&self, base: &str, quote: &Currency) -> String {
        match self {
            ExchangeChoice::Hyperliquid => base.to_string(),
            ExchangeChoice::Bitget => match quote {
                Currency::USDT => format!("{}USDT", base),
                Currency::USDC => format!("{}USD", base),
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
            ExchangeChoice::Hyperliquid => *quote == Currency::USDC,
            ExchangeChoice::Bitget => matches!(quote, Currency::USDT | Currency::USDC),
        }
    }
}

pub struct SessionState {
    pub active: AtomicBool,
    pub base_currency: RwLock<Option<Currency>>,
    pub exchange: RwLock<Option<ExchangeChoice>>,
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
            base_currency: RwLock::new(None),
            exchange: RwLock::new(None),
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
    pub ws_url: String,
    pub bitget_ws_url: String,
}

impl Workspace {
    pub fn new(
        config: Arc<RwLock<AppConfig>>,
        pool: SqlitePool,
        symbol_mapper: Arc<SymbolMapper>,
        telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
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
            ws_url,
            bitget_ws_url,
        }
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
        currency: Currency,
        exchange: ExchangeChoice,
    ) -> Result<(), String> {
        if exchange != ExchangeChoice::Hyperliquid && exchange != ExchangeChoice::Bitget {
            return Err("Unsupported exchange selected.".to_string());
        }
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

        *self.session.base_currency.write().await = Some(currency.clone());
        *self.session.exchange.write().await = Some(exchange.clone());
        self.session
            .active
            .store(true, std::sync::atomic::Ordering::Relaxed);

        println!(
            "✅ Session initialized: {} on {}",
            currency.as_str(),
            exchange.as_str(),
        );
        Ok(())
    }

    pub async fn quit_session(&self) -> Result<(), String> {
        println!("🛑 Initiating graceful shutdown of all instances...");

        let instance_ids: Vec<String> = {
            let instances = self.instances.read().await;
            instances.values().map(|i| i.id.clone()).collect()
        };

        for instance_id in &instance_ids {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.values().find(|i| &i.id == instance_id) {
                instance.cancel.cancel();
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        self.instances.write().await.clear();

        self.session
            .active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        *self.session.base_currency.write().await = None;
        *self.session.exchange.write().await = None;

        println!("✅ Session terminated. All instances stopped.");
        Ok(())
    }
}
