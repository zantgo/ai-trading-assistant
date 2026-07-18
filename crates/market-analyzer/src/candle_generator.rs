#[cfg(test)]
use core_domain::normalized::TradeSide;
use core_domain::normalized::{Exchange, NormalizedCandle, NormalizedTrade};
use rust_decimal::Decimal;

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
    pub exchange: Exchange,
}

impl CandleGenerator {
    pub fn new(symbol: &str, duration_seconds: u64, exchange: Exchange) -> Self {
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
            exchange,
        }
    }

    pub fn set_exchange(&mut self, exchange: Exchange) {
        self.exchange = exchange;
    }

    pub fn is_late_tick(&self, timestamp_ms: u64) -> bool {
        if self.current_candle.is_none() {
            return false;
        }
        let interval_start = (timestamp_ms / self.duration_ms) * self.duration_ms;
        interval_start < self.current_start_ms
    }

    pub fn process_trade(
        &mut self,
        trade: &NormalizedTrade,
    ) -> (Option<NormalizedCandle>, NormalizedCandle) {
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
            exchange: self.exchange,
            symbol: self.symbol.clone(),
            start_time_ms: self.current_start_ms,
            duration_ms: self.duration_ms,
            open: self.current_open,
            high: self.current_high,
            low: self.current_low,
            close: self.current_close,
            volume: self.current_volume,
            trades_count: self.current_trades,
            reconstructed: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_candle_boundaries_and_rollover() {
        let mut generator = CandleGenerator::new("BTC-USD", 60, Exchange::Hyperliquid);

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
        assert!(
            closed.is_none(),
            "First tick inside timeframe must not trigger a closed candle."
        );
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

        assert!(
            closed.is_some(),
            "Exceeding timeframe window must return the completed candle."
        );
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
        let mut generator = CandleGenerator::new("ETH-USD", 30, Exchange::Hyperliquid);
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
        assert_eq!(
            live.start_time_ms, 0,
            "30s candle for timestamp 15000ms aligns to epoch bucket 0"
        );
    }

    #[test]
    fn test_interval_alignment() {
        let mut generator = CandleGenerator::new("SOL-USD", 60, Exchange::Hyperliquid);
        let trade = NormalizedTrade {
            exchange: Exchange::Hyperliquid,
            symbol: "SOL-USD".to_string(),
            price: dec!(100.00),
            size: dec!(5.0),
            side: TradeSide::Buy,
            timestamp_ms: 123456,
            trade_id: "sol_1".to_string(),
        };

        let (_closed, live) = generator.process_trade(&trade);
        assert_eq!(
            live.start_time_ms, 120000,
            "60s candle should align to epoch boundary"
        );
    }

    #[test]
    fn test_trade_count_increments_correctly() {
        let mut generator = CandleGenerator::new("BTC-USD", 60, Exchange::Hyperliquid);

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
            assert_eq!(live.trades_count, i + 1);
        }
    }
}
