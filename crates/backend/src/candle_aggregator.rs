use shared::normalized::NormalizedCandle;
use tokio::sync::{broadcast, mpsc};

// TODO(doc-followup): The platform specification (see
//   docs/conceptual-foundations/01-global-architecture.md §2.1,
//   docs/conceptual-foundations/03-timeframe-model.md §3.1,
//   docs/engines/data-infrastructure-engine/die-layer2-market-data.md §3.1)
//   requires:
//
//     1. All candles (micro/fast/slow/macro) to close at the exact UTC interval
//        boundary (e.g. macro900 closes at `:14:59.999`, `:29:59.999`,
//        `:44:59.999`, `:59:59.999`).
//     2. The local host clock to maintain drift of <= 50 microseconds against
//        UTC via continuous NTP polling.
//
//   Current behaviour in this module: interval alignment is already UTC-correct
//   via epoch-bucket flooring (see `process_1m_candle`). What is NOT yet
//   implemented is the <=50µs clock-drift monitoring/assertion. The candle
//   close itself is event-driven (it fires when the next source candle crosses
//   the boundary), not by a wall-clock timer at the exact millisecond.
//
//   Action items to align code with spec:
//     - Add a `ClockMonitor` (startup + periodic) that reads the OS clock and
//       compares against an NTP peer; warn / panic / slow ingestion if drift
//       exceeds 50µs over the monitoring window.
//     - Optionally add a wall-clock-timer flush so the boundary close emits at
//       the exact `interval_start + duration_ms` even if no source trade
//       triggers a crossing (currently waits for the next source candle).
//     - Hook the `ClockMonitor` into the engine `main.rs` startup so NTP drift
//       is enforced before live trading.
//
//   This must be addressed before NTP-strict UTC alignment can be claimed as an
//   enforced guarantee rather than a documented aspiration.

/// Aggregated macro candle event from 1-minute source candles.
#[derive(Debug, Clone)]
pub struct AggregatedCandle {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub candle: NormalizedCandle,
    pub source_count: u64,
}

/// Candle aggregator that builds 4h and 1d candles from 1-minute candle closes.
///
/// Listens to 1-minute candle close events via a broadcast channel and
/// combines them into macro-scale candles (4h = 240 × 1m, 1d = 1440 × 1m).
pub struct CandleAggregator {
    symbol: String,
    duration_4h: u64,
    duration_1d: u64,
    pending_4h: Option<AggregatedCandle>,
    pending_1d: Option<AggregatedCandle>,
    count_4h: u64,
    count_1d: u64,
}

impl CandleAggregator {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            duration_4h: 14400,
            duration_1d: 86400,
            pending_4h: None,
            pending_1d: None,
            count_4h: 0,
            count_1d: 0,
        }
    }

    /// Process a 1-minute closed candle. Returns completed macro candles if any.
    pub fn process_1m_candle(
        &mut self,
        candle: &NormalizedCandle,
    ) -> (Option<AggregatedCandle>, Option<AggregatedCandle>) {
        let mut completed_4h = None;
        let mut completed_1d = None;

        // Aggregate 4h candle (240 × 1m)
        let interval_start_4h =
            (candle.start_time_ms / (self.duration_4h * 1000)) * (self.duration_4h * 1000);

        if let Some(ref pending) = self.pending_4h {
            let pending_start = pending.candle.start_time_ms;
            if interval_start_4h > pending_start {
                completed_4h = self.pending_4h.take();
                self.count_4h = 0;
            }
        }

        match self.pending_4h.as_mut() {
            Some(pending) => {
                pending.candle.high = pending.candle.high.max(candle.high);
                pending.candle.low = pending.candle.low.min(candle.low);
                pending.candle.close = candle.close;
                pending.candle.volume += candle.volume;
                pending.candle.trades_count += candle.trades_count;
                pending.source_count += 1;
                self.count_4h += 1;
            }
            None => {
                self.pending_4h = Some(AggregatedCandle {
                    symbol: self.symbol.clone(),
                    timeframe_secs: self.duration_4h,
                    candle: NormalizedCandle {
                        symbol: self.symbol.clone(),
                        start_time_ms: interval_start_4h,
                        duration_ms: self.duration_4h * 1000,
                        open: candle.open,
                        high: candle.high,
                        low: candle.low,
                        close: candle.close,
                        volume: candle.volume,
                        trades_count: candle.trades_count,
                    },
                    source_count: 1,
                });
                self.count_4h = 1;
            }
        }

        // Aggregate 1d candle (1440 × 1m)
        let interval_start_1d =
            (candle.start_time_ms / (self.duration_1d * 1000)) * (self.duration_1d * 1000);

        if let Some(ref pending) = self.pending_1d {
            let pending_start = pending.candle.start_time_ms;
            if interval_start_1d > pending_start {
                completed_1d = self.pending_1d.take();
                self.count_1d = 0;
            }
        }

        match self.pending_1d.as_mut() {
            Some(pending) => {
                pending.candle.high = pending.candle.high.max(candle.high);
                pending.candle.low = pending.candle.low.min(candle.low);
                pending.candle.close = candle.close;
                pending.candle.volume += candle.volume;
                pending.candle.trades_count += candle.trades_count;
                pending.source_count += 1;
                self.count_1d += 1;
            }
            None => {
                self.pending_1d = Some(AggregatedCandle {
                    symbol: self.symbol.clone(),
                    timeframe_secs: self.duration_1d,
                    candle: NormalizedCandle {
                        symbol: self.symbol.clone(),
                        start_time_ms: interval_start_1d,
                        duration_ms: self.duration_1d * 1000,
                        open: candle.open,
                        high: candle.high,
                        low: candle.low,
                        close: candle.close,
                        volume: candle.volume,
                        trades_count: candle.trades_count,
                    },
                    source_count: 1,
                });
                self.count_1d = 1;
            }
        }

        (completed_4h, completed_1d)
    }
}

/// Spawn a background task that listens for 1-minute candle closes
/// and aggregates 4h / 1d macro candles.
pub fn spawn_candle_aggregator(
    symbol: String,
    mut rx_1m: broadcast::Receiver<NormalizedCandle>,
    tx_4h: mpsc::Sender<AggregatedCandle>,
    tx_1d: mpsc::Sender<AggregatedCandle>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut aggregator = CandleAggregator::new(&symbol);
        loop {
            match rx_1m.recv().await {
                Ok(candle) => {
                    let (c4h, c1d) = aggregator.process_1m_candle(&candle);
                    if let Some(ac) = c4h {
                        let _ = tx_4h.send(ac).await;
                    }
                    if let Some(ac) = c1d {
                        let _ = tx_1d.send(ac).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!(
                        "⚠️ Candle Aggregator [{}]: Lagged by {} messages, resetting",
                        symbol, n
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    eprintln!(
                        "📭 Candle Aggregator [{}]: 1m channel closed, shutting down",
                        symbol
                    );
                    break;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    fn make_candle(start_ms: u64, open: f64, close: f64, high: f64, low: f64) -> NormalizedCandle {
        NormalizedCandle {
            symbol: "TEST".to_string(),
            start_time_ms: start_ms,
            duration_ms: 60000,
            open: Decimal::from_f64(open).unwrap(),
            high: Decimal::from_f64(high).unwrap(),
            low: Decimal::from_f64(low).unwrap(),
            close: Decimal::from_f64(close).unwrap(),
            volume: Decimal::from(1),
            trades_count: 10,
        }
    }

    #[test]
    fn test_aggregation_respects_interval_boundaries() {
        let mut agg = CandleAggregator::new("TEST");

        let c1 = make_candle(0, 100.0, 101.0, 102.0, 99.0);
        let c2 = make_candle(60000, 101.0, 103.0, 104.0, 100.0);

        let (r4h, r1d) = agg.process_1m_candle(&c1);
        assert!(r4h.is_none());
        assert!(r1d.is_none());

        let (r4h, _r1d) = agg.process_1m_candle(&c2);
        assert!(r4h.is_none());

        let pending = agg.pending_4h.as_ref().unwrap();
        assert_eq!(pending.candle.high.to_f64().unwrap(), 104.0);
        assert_eq!(pending.candle.low.to_f64().unwrap(), 99.0);
        assert_eq!(pending.source_count, 2);
    }
}
