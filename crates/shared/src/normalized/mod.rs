pub mod candle_generator;
pub mod symbol_mapper;

pub use candle_generator::CandleGenerator;
pub use symbol_mapper::SymbolMapper;

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    Hyperliquid,
    Bitget,
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerType {
    Manual,
    Automated,
}

impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedTrade {
    pub exchange: Exchange,
    pub symbol: String,
    pub price: Decimal,
    pub size: Decimal,
    pub side: TradeSide,
    pub timestamp_ms: u64,
    pub trade_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedOpenInterest {
    pub exchange: Exchange,
    pub symbol: String,
    pub oi: Decimal,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedFundingRate {
    pub exchange: Exchange,
    pub symbol: String,
    pub rate: Decimal,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedOrderBook {
    pub exchange: Exchange,
    pub symbol: String,
    pub bids: Vec<(Decimal, Decimal)>,
    pub asks: Vec<(Decimal, Decimal)>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedAssetContext {
    pub exchange: Exchange,
    pub symbol: String,
    pub prev_day_px: Decimal,
    pub mark_px: Decimal,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormalizedEvent {
    Trade(NormalizedTrade),
    OrderBook(NormalizedOrderBook),
    AssetContext(NormalizedAssetContext),
    OpenInterest(NormalizedOpenInterest),
    FundingRate(NormalizedFundingRate),
    Status {
        exchange: Exchange,
        status: ConnectionStatus,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedCandle {
    pub symbol: String,
    pub start_time_ms: u64,
    pub duration_ms: u64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub trades_count: u64,
}

impl NormalizedCandle {
    pub fn assert_validity(&self) -> Result<(), String> {
        if self.high < self.low {
            return Err("High price cannot be less than low price".into());
        }
        if self.open < self.low || self.open > self.high {
            return Err(format!(
                "Open ({}) falls outside Low/High bounds",
                self.open
            ));
        }
        if self.close < self.low || self.close > self.high {
            return Err(format!(
                "Close ({}) falls outside Low/High bounds",
                self.close
            ));
        }
        if self.volume < Decimal::ZERO {
            return Err("Volume cannot be negative".into());
        }
        Ok(())
    }
}

#[async_trait]
pub trait ExchangeAdapter: Send + Sync {
    fn exchange(&self) -> Exchange;

    async fn start(
        &self,
        symbols: Vec<String>,
        event_tx: tokio::sync::mpsc::Sender<NormalizedEvent>,
        mapper: Arc<SymbolMapper>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(test)]
mod consistency_tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_candle_validity_passes_for_valid_data() {
        let candle = NormalizedCandle {
            symbol: "BTC-USD".to_string(),
            start_time_ms: 1000,
            duration_ms: 60000,
            open: dec!(50000.00),
            high: dec!(51000.00),
            low: dec!(49000.00),
            close: dec!(50500.00),
            volume: dec!(1.5),
            trades_count: 5,
        };
        assert!(candle.assert_validity().is_ok());
    }

    #[test]
    fn test_candle_validity_catches_inverted_high_low() {
        let candle = NormalizedCandle {
            symbol: "BTC-USD".to_string(),
            start_time_ms: 1000,
            duration_ms: 60000,
            open: dec!(50000.00),
            high: dec!(49000.00),
            low: dec!(48000.00),
            close: dec!(49500.00),
            volume: dec!(1.5),
            trades_count: 5,
        };
        assert!(candle.assert_validity().is_err());
    }

    #[test]
    fn test_candle_validity_catches_negative_volume() {
        let candle = NormalizedCandle {
            symbol: "BTC-USD".to_string(),
            start_time_ms: 1000,
            duration_ms: 60000,
            open: dec!(50000.00),
            high: dec!(51000.00),
            low: dec!(49000.00),
            close: dec!(50500.00),
            volume: dec!(-1.0),
            trades_count: 1,
        };
        assert!(candle.assert_validity().is_err());
    }

    #[test]
    fn test_candle_validity_catches_open_outside_bounds() {
        let candle = NormalizedCandle {
            symbol: "BTC-USD".to_string(),
            start_time_ms: 1000,
            duration_ms: 60000,
            open: dec!(52000.00),
            high: dec!(51000.00),
            low: dec!(49000.00),
            close: dec!(50500.00),
            volume: dec!(1.0),
            trades_count: 1,
        };
        assert!(candle.assert_validity().is_err());
    }
}
