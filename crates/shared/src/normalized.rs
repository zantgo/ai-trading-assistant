use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    Hyperliquid,
    EdgeX,
    Bybit,
    Bitget,
    Kraken,
    Coinbase,
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
pub enum NormalizedEvent {
    Trade(NormalizedTrade),
    OrderBook(NormalizedOrderBook),
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
            return Err(format!("Open ({}) falls outside Low/High bounds", self.open));
        }
        if self.close < self.low || self.close > self.high {
            return Err(format!("Close ({}) falls outside Low/High bounds", self.close));
        }
        if self.volume < Decimal::ZERO {
            return Err("Volume cannot be negative".into());
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SymbolMapper {
    to_normalized: RwLock<HashMap<(Exchange, String), String>>,
    to_raw: RwLock<HashMap<(Exchange, String), String>>,
}

impl SymbolMapper {
    pub fn new() -> Self {
        Self {
            to_normalized: RwLock::new(HashMap::new()),
            to_raw: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, exchange: Exchange, raw: &str, normalized: &str) {
        let mut to_norm = self.to_normalized.write().await;
        let mut to_r = self.to_raw.write().await;
        to_norm.insert((exchange, raw.to_string()), normalized.to_string());
        to_r.insert((exchange, normalized.to_string()), raw.to_string());
    }

    pub async fn normalize(&self, exchange: Exchange, raw: &str) -> Option<String> {
        let to_norm = self.to_normalized.read().await;
        to_norm.get(&(exchange, raw.to_string())).cloned()
    }

    pub async fn get_raw(&self, exchange: Exchange, normalized: &str) -> Option<String> {
        let to_r = self.to_raw.read().await;
        to_r.get(&(exchange, normalized.to_string())).cloned()
    }

    pub async fn get_normalized_for_exchange(&self, exchange: Exchange) -> Vec<String> {
        let to_norm = self.to_normalized.read().await;
        let mut result = Vec::new();
        for ((ex, _raw), normalized) in to_norm.iter() {
            if *ex == exchange {
                result.push(normalized.clone());
            }
        }
        result
    }

    pub async fn load_default_mappings(&self) {
        self.register(Exchange::Hyperliquid, "BTC", "BTC-USD").await;
        self.register(Exchange::Hyperliquid, "ETH", "ETH-USD").await;
        self.register(Exchange::Hyperliquid, "SOL", "SOL-USD").await;
        self.register(Exchange::EdgeX, "BTCUSD", "BTC-USD").await;
        self.register(Exchange::EdgeX, "ETHUSD", "ETH-USD").await;
        self.register(Exchange::Bybit, "BTCUSDT", "BTC-USD").await;
        self.register(Exchange::Bybit, "ETHUSDT", "ETH-USD").await;
        self.register(Exchange::Bitget, "BTCUSDT", "BTC-USD").await;
        self.register(Exchange::Bitget, "ETHUSDT", "ETH-USD").await;
        self.register(Exchange::Kraken, "BTC/USD", "BTC-USD").await;
        self.register(Exchange::Kraken, "ETH/USD", "ETH-USD").await;
        self.register(Exchange::Coinbase, "BTC-USD", "BTC-USD").await;
        self.register(Exchange::Coinbase, "ETH-USD", "ETH-USD").await;
    }
}

pub struct CandleGenerator {
    pub symbol: String,
    pub duration_ms: u64,
    pub current_candle: Option<NormalizedCandle>,
    pub current_close: Decimal,
    pub current_high: Decimal,
    pub current_low: Decimal,
    pub current_volume: Decimal,
    pub current_trades: u64,
    pub current_start_ms: u64,
    pub current_open: Decimal,
}

impl CandleGenerator {
    pub fn new(symbol: &str, duration_seconds: u64) -> Self {
        Self {
            symbol: symbol.to_string(),
            duration_ms: duration_seconds * 1000,
            current_candle: None,
            current_close: Decimal::ZERO,
            current_high: Decimal::ZERO,
            current_low: Decimal::ZERO,
            current_volume: Decimal::ZERO,
            current_trades: 0,
            current_start_ms: 0,
            current_open: Decimal::ZERO,
        }
    }

    pub fn process_trade(&mut self, trade: &NormalizedTrade) -> (Option<NormalizedCandle>, NormalizedCandle) {
        let interval_start = (trade.timestamp_ms / self.duration_ms) * self.duration_ms;

        if self.current_candle.is_none() {
            self.current_start_ms = interval_start;
            self.current_open = trade.price;
            self.current_high = trade.price;
            self.current_low = trade.price;
            self.current_close = trade.price;
            self.current_volume = trade.size;
            self.current_trades = 1;

            let live = self.make_live();
            self.current_candle = Some(live.clone());
            (None, live)
        } else if interval_start > self.current_start_ms {
            let completed = self.current_candle.take().unwrap();

            self.current_start_ms = interval_start;
            self.current_open = trade.price;
            self.current_high = trade.price;
            self.current_low = trade.price;
            self.current_close = trade.price;
            self.current_volume = trade.size;
            self.current_trades = 1;

            let live = self.make_live();
            self.current_candle = Some(live.clone());
            (Some(completed), live)
        } else {
            self.current_high = self.current_high.max(trade.price);
            self.current_low = self.current_low.min(trade.price);
            self.current_close = trade.price;
            self.current_volume += trade.size;
            self.current_trades += 1;

            let live = self.make_live();
            self.current_candle = Some(live.clone());
            (None, live)
        }
    }

    fn make_live(&self) -> NormalizedCandle {
        NormalizedCandle {
            symbol: self.symbol.clone(),
            start_time_ms: self.current_start_ms,
            duration_ms: self.duration_ms,
            open: self.current_open,
            high: self.current_high,
            low: self.current_low,
            close: self.current_close,
            volume: self.current_volume,
            trades_count: self.current_trades,
        }
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

#[cfg(test)]
mod symbol_mapper_tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_normalize() {
        let mapper = SymbolMapper::new();
        mapper.register(Exchange::Bybit, "BTCUSDT", "BTC-USD").await;
        assert_eq!(
            mapper.normalize(Exchange::Bybit, "BTCUSDT").await,
            Some("BTC-USD".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_raw_reverse_mapping() {
        let mapper = SymbolMapper::new();
        mapper.register(Exchange::Coinbase, "BTC-USD", "BTC-USD").await;
        assert_eq!(
            mapper.get_raw(Exchange::Coinbase, "BTC-USD").await,
            Some("BTC-USD".to_string())
        );
    }

    #[tokio::test]
    async fn test_unknown_mapping_returns_none() {
        let mapper = SymbolMapper::new();
        assert_eq!(mapper.normalize(Exchange::Kraken, "UNKNOWN").await, None);
    }

    #[tokio::test]
    async fn test_case_sensitive_keys() {
        let mapper = SymbolMapper::new();
        mapper.register(Exchange::Bybit, "BTCUSDT", "BTC-USD").await;
        assert_eq!(
            mapper.normalize(Exchange::Bybit, "btcusdt").await,
            None,
            "SymbolMapper keys are case-sensitive; lowercase should not match"
        );
    }

    #[tokio::test]
    async fn test_load_default_mappings_covers_all_exchanges() {
        let mapper = SymbolMapper::new();
        mapper.load_default_mappings().await;

        assert_eq!(mapper.normalize(Exchange::Hyperliquid, "BTC").await, Some("BTC-USD".to_string()));
        assert_eq!(mapper.normalize(Exchange::Bybit, "BTCUSDT").await, Some("BTC-USD".to_string()));
        assert_eq!(mapper.normalize(Exchange::Coinbase, "BTC-USD").await, Some("BTC-USD".to_string()));
        assert_eq!(mapper.normalize(Exchange::Kraken, "BTC/USD").await, Some("BTC-USD".to_string()));
    }
}

#[cfg(test)]
mod candle_tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_candle_boundaries_and_rollover() {
        let mut generator = CandleGenerator::new("BTC-USD", 60);

        let t1 = NormalizedTrade {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USD".to_string(),
            price: dec!(50000.00),
            size: dec!(1.5),
            side: TradeSide::Buy,
            timestamp_ms: 60500,
            trade_id: "trade_1".to_string(),
        };

        let (closed, live) = generator.process_trade(&t1);
        assert!(closed.is_none(), "First tick inside timeframe must not trigger a closed candle.");
        assert_eq!(live.open, dec!(50000.00));
        assert_eq!(live.high, dec!(50000.00));
        assert_eq!(live.low, dec!(50000.00));
        assert_eq!(live.close, dec!(50000.00));
        assert_eq!(live.volume, dec!(1.5));
        assert_eq!(live.trades_count, 1);

        let t2 = NormalizedTrade {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USD".to_string(),
            price: dec!(51000.00),
            size: dec!(0.5),
            side: TradeSide::Buy,
            timestamp_ms: 61200,
            trade_id: "trade_2".to_string(),
        };

        let (closed, live) = generator.process_trade(&t2);
        assert!(closed.is_none());
        assert_eq!(live.high, dec!(51000.00));
        assert_eq!(live.low, dec!(50000.00));
        assert_eq!(live.close, dec!(51000.00));
        assert_eq!(live.volume, dec!(2.0));
        assert_eq!(live.trades_count, 2);

        let t3 = NormalizedTrade {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USD".to_string(),
            price: dec!(49500.00),
            size: dec!(2.0),
            side: TradeSide::Sell,
            timestamp_ms: 120500,
            trade_id: "trade_3".to_string(),
        };

        let (closed, live) = generator.process_trade(&t3);

        assert!(closed.is_some(), "Exceeding timeframe window must return the completed candle.");
        let closed_candle = closed.unwrap();
        assert_eq!(closed_candle.start_time_ms, 60000);
        assert_eq!(closed_candle.open, dec!(50000.00));
        assert_eq!(closed_candle.high, dec!(51000.00));
        assert_eq!(closed_candle.low, dec!(50000.00));
        assert_eq!(closed_candle.close, dec!(51000.00));
        assert_eq!(closed_candle.volume, dec!(2.0));
        assert_eq!(closed_candle.trades_count, 2);

        assert_eq!(live.start_time_ms, 120000);
        assert_eq!(live.open, dec!(49500.00));
        assert_eq!(live.volume, dec!(2.0));
        assert_eq!(live.trades_count, 1);
    }

    #[test]
    fn test_first_trade_initializes_candle() {
        let mut generator = CandleGenerator::new("ETH-USD", 30);
        let trade = NormalizedTrade {
            exchange: Exchange::Hyperliquid,
            symbol: "ETH-USD".to_string(),
            price: dec!(3000.00),
            size: dec!(10.0),
            side: TradeSide::Buy,
            timestamp_ms: 15000,
            trade_id: "id_1".to_string(),
        };

        let (closed, live) = generator.process_trade(&trade);
        assert!(closed.is_none());
        assert_eq!(live.open, dec!(3000.00));
        assert_eq!(live.high, dec!(3000.00));
        assert_eq!(live.low, dec!(3000.00));
        assert_eq!(live.close, dec!(3000.00));
        assert_eq!(live.volume, dec!(10.0));
        assert_eq!(live.trades_count, 1);
        assert_eq!(live.start_time_ms, 0, "30s candle for timestamp 15000ms aligns to epoch bucket 0");
    }

    #[test]
    fn test_interval_alignment() {
        let mut generator = CandleGenerator::new("SOL-USD", 60);
        let trade = NormalizedTrade {
            exchange: Exchange::Bybit,
            symbol: "SOL-USD".to_string(),
            price: dec!(100.00),
            size: dec!(5.0),
            side: TradeSide::Buy,
            timestamp_ms: 123456,
            trade_id: "sol_1".to_string(),
        };

        let (_closed, live) = generator.process_trade(&trade);
        assert_eq!(live.start_time_ms, 120000, "60s candle should align to epoch boundary");
    }

    #[test]
    fn test_trade_count_increments_correctly() {
        let mut generator = CandleGenerator::new("BTC-USD", 60);

        for i in 0..5 {
            let trade = NormalizedTrade {
                exchange: Exchange::Hyperliquid,
                symbol: "BTC-USD".to_string(),
                price: dec!(50000.00) + Decimal::from(i),
                size: dec!(1.0),
                side: TradeSide::Buy,
                timestamp_ms: 1000 + (i * 100),
                trade_id: format!("t_{}", i),
            };
            let (_closed, live) = generator.process_trade(&trade);
            assert_eq!(live.trades_count, (i + 1) as u64);
        }
    }
}
