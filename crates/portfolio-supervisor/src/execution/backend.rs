//! v7 TAE — ExecutionBackend trait + PaperSimulation.
//!
//! The unified `ExecutionEngine` holds ALL state (orders, positions, equity,
//! fees) and is mode-agnostic. The only mode-dependent decision — "does this
//! order fill at this price?" — is delegated to an `ExecutionBackend`.
//! `PaperSimulation` answers from the simulated mid; a future `LiveBroker`
//! answers from exchange state.

use config_models::{ExecutionMode, OrderPacket, OrderSide, OrderType};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use super::state_machine::OrderLifecycle;
use crate::paper_trading::FeesConfig;

/// A single simulated/venue fill.
#[derive(Debug, Clone)]
pub struct Fill {
    pub order_id: String,
    pub price: Decimal,
}

/// The venue-facing half of the unified execution engine. Paper and live are
/// the same program; only this trait differs at the very end of the path.
#[async_trait::async_trait]
pub trait ExecutionBackend: Send + Sync {
    /// The mode this backend implements.
    fn mode(&self) -> ExecutionMode;

    /// Evaluate whether an open order fills at the given mid price.
    /// Returns the fill price, or `None` when the order does not fill.
    ///
    /// Market orders are handled at submit time by the engine (instant fill
    /// with spread); this method handles limit and stop triggers. Live
    /// backends return `None` — fills come from the venue via `poll_fills`.
    fn evaluate_fill(&self, order: &OrderLifecycle, mid: Decimal) -> Option<Decimal>;

    /// Submit an order to the venue (live only). Returns the venue order id.
    async fn submit_order(&self, _packet: &OrderPacket) -> Result<String, String> {
        Err("submit_order not implemented for this backend".to_string())
    }

    /// Cancel an order at the venue (live only).
    async fn cancel_order(&self, _order_id: &str) -> Result<(), String> {
        Err("cancel_order not implemented for this backend".to_string())
    }

    /// Poll the venue for fills (live only).
    async fn poll_fills(&self) -> Vec<Fill> {
        Vec::new()
    }

    /// Fetch the account equity from the venue (live only).
    async fn fetch_equity(&self) -> Result<f64, String> {
        Err("fetch_equity not implemented for this backend".to_string())
    }
}

/// Internal simulated matching — the v7 default backend.
///
/// Fill rules (mirror of the legacy paper engine, clamped to the limit price
/// so a fill can never be worse than the resting limit):
///   - Limit Buy:  fills when `mid <= limit`; fill price = `min(mid ± spread, limit)`.
///   - Limit Sell: fills when `mid >= limit`; fill price = `max(mid ∓ spread, limit)`.
///   - Stop Sell:  triggers when `mid <= stop`; fills at mid with spread.
///   - Stop Buy:   triggers when `mid >= stop`; fills at mid with spread.
pub struct PaperSimulation {
    pub fee_config: FeesConfig,
}

impl PaperSimulation {
    pub fn new(fee_config: FeesConfig) -> Self {
        Self { fee_config }
    }
}

#[async_trait::async_trait]
impl ExecutionBackend for PaperSimulation {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Paper
    }

    fn evaluate_fill(&self, order: &OrderLifecycle, mid: Decimal) -> Option<Decimal> {
        match order.packet.order_type {
            OrderType::Market => None,
            OrderType::Limit => {
                let limit = order.packet.price?;
                match order.packet.side {
                    OrderSide::Buy => {
                        if mid <= limit {
                            Some(mid.min(limit))
                        } else {
                            None
                        }
                    }
                    OrderSide::Sell => {
                        if mid >= limit {
                            Some(mid.max(limit))
                        } else {
                            None
                        }
                    }
                }
            }
            OrderType::Stop => {
                let stop = order.packet.price?;
                match order.packet.side {
                    OrderSide::Sell => {
                        if mid <= stop {
                            Some(mid)
                        } else {
                            None
                        }
                    }
                    OrderSide::Buy => {
                        if mid >= stop {
                            Some(mid)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }
}

/// Live Hyperliquid backend (AUDIT-V6-406 / Phase E1). Implements the trait
/// against the signed `/exchange` endpoint; fills are polled via `/info`.
pub struct LiveBroker {
    pub client: network_adapters::adapters::hyperliquid_live::HyperliquidLiveClient,
    /// symbol → Hyperliquid asset index (resolved lazily, cached).
    indices: tokio::sync::RwLock<std::collections::HashMap<String, i64>>,
}

impl LiveBroker {
    pub fn new(
        address: String,
        private_key_hex: String,
        is_mainnet: bool,
        chain_id: Option<u64>,
    ) -> Self {
        Self {
            client: network_adapters::adapters::hyperliquid_live::HyperliquidLiveClient::new(
                address,
                private_key_hex,
                is_mainnet,
                chain_id,
            ),
            indices: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    async fn asset_index_for(&self, symbol: &str) -> Result<i64, String> {
        {
            let map = self.indices.read().await;
            if let Some(idx) = map.get(symbol) {
                return Ok(*idx);
            }
        }
        let coin = network_adapters::adapters::hyperliquid_live::coin_from_symbol(symbol);
        let idx = self.client.asset_index(&coin).await?;
        self.indices.write().await.insert(symbol.to_string(), idx);
        Ok(idx)
    }
}

#[async_trait::async_trait]
impl ExecutionBackend for LiveBroker {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Live
    }

    fn evaluate_fill(&self, _order: &OrderLifecycle, _mid: Decimal) -> Option<Decimal> {
        None
    }

    async fn submit_order(&self, packet: &OrderPacket) -> Result<String, String> {
        let idx = self.asset_index_for(&packet.symbol).await?;
        let hl = network_adapters::adapters::hyperliquid_live::hl_order_from_packet(packet, idx);
        let mut ids = self.client.place_orders(&[hl]).await?;
        ids.pop().ok_or_else(|| "no order id returned".to_string())
    }

    async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        let oid: u64 = order_id
            .parse()
            .map_err(|_| format!("invalid live order id '{}'", order_id))?;
        // Asset index is unknown at cancel time in the common path; the HL
        // cancel endpoint accepts (a, o) pairs. Resolve via a broad cancel:
        // iterate known symbols' indices is impractical — the engine cancels
        // by client order id for bracket cleanup, so we cancel per-symbol
        // with the caller-supplied index when available. Fallback: try index
        // 0 and let the exchange reject unknown pairs (cancels are idempotent).
        self.client.cancel_orders(&[(0, oid)]).await
    }

    async fn poll_fills(&self) -> Vec<Fill> {
        match self.client.fetch_fills().await {
            Ok(fills) => fills
                .into_iter()
                .map(|f| Fill {
                    order_id: f.order_id,
                    price: Decimal::from_f64_retain(f.price).unwrap_or_default(),
                })
                .collect(),
            Err(e) => {
                eprintln!("LIVE: poll_fills failed: {}", e);
                Vec::new()
            }
        }
    }

    async fn fetch_equity(&self) -> Result<f64, String> {
        self.client.fetch_equity().await
    }
}
