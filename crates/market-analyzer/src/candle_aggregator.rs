use core_domain::normalized::NormalizedCandle;
use tokio::sync::{broadcast, mpsc};

/// Aggregated candle built from source (micro) candles via multi-timeframe rollup.
#[derive(Debug, Clone)]
pub struct AggregatedCandle {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub candle: NormalizedCandle,
    pub source_count: u64,
}

/// Per-target pending state for a single aggregation timeframe.
struct TargetAggregator {
    duration_secs: u64,
    duration_ms: u64,
    pending: Option<AggregatedCandle>,
}

impl TargetAggregator {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration_secs,
            duration_ms: duration_secs * 1000,
            pending: None,
        }
    }

    /// Process one source candle. Returns a completed `AggregatedCandle`
    /// when the interval rolls over, or `None` otherwise.
    fn process(&mut self, symbol: &str, candle: &NormalizedCandle) -> Option<AggregatedCandle> {
        let interval_start = (candle.start_time_ms / self.duration_ms) * self.duration_ms;

        let mut completed = None;

        if let Some(ref pending) = self.pending {
            if interval_start > pending.candle.start_time_ms {
                completed = self.pending.take();
            }
        }

        if let Some(ref mut pending) = self.pending {
            pending.candle.high = pending.candle.high.max(candle.high);
            pending.candle.low = pending.candle.low.min(candle.low);
            pending.candle.close = candle.close;
            pending.candle.volume += candle.volume;
            pending.candle.trades_count += candle.trades_count;
            pending.source_count += 1;
            if candle.reconstructed.is_some() && pending.candle.reconstructed.is_none() {
                pending.candle.reconstructed = candle.reconstructed;
            }
        } else {
            self.pending = Some(AggregatedCandle {
                symbol: symbol.to_string(),
                timeframe_secs: self.duration_secs,
                candle: NormalizedCandle {
                    exchange: candle.exchange,
                    symbol: symbol.to_string(),
                    start_time_ms: interval_start,
                    duration_ms: self.duration_ms,
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: candle.volume,
                    trades_count: candle.trades_count,
                    reconstructed: candle.reconstructed,
                },
                source_count: 1,
            });
        }

        completed
    }
}

/// Multi-timeframe candle aggregator that rolls a source (e.g. micro) candle
/// stream into one or more higher-timeframe candles.
///
/// Target durations are configured at construction time. The aggregator
/// maintains independent pending-candle state per target and emits completed
/// candles when source ticks cross the target's interval boundary.
pub struct CandleAggregator {
    symbol: String,
    targets: Vec<TargetAggregator>,
}

impl CandleAggregator {
    /// Create an aggregator for `symbol` with the given target durations (seconds).
    ///
    /// At least one target must be provided. Duplicate durations are de-duplicated.
    /// Targets are sorted so the shortest duration is processed first, which
    /// guarantees that a source candle that crosses multiple target intervals
    /// produces completed candles in chronological order.
    pub fn new(symbol: &str, target_durations_secs: &[u64]) -> Self {
        Self::try_new(symbol, target_durations_secs)
            .expect("CandleAggregator requires at least one target duration")
    }

    pub fn try_new(symbol: &str, target_durations_secs: &[u64]) -> Result<Self, String> {
        let mut targets: Vec<u64> = target_durations_secs.to_vec();
        targets.sort_unstable();
        targets.dedup();
        if targets.is_empty() {
            return Err("CandleAggregator requires at least one target duration".to_string());
        }
        Ok(Self {
            symbol: symbol.to_string(),
            targets: targets.into_iter().map(TargetAggregator::new).collect(),
        })
    }

    /// Process a source candle. Returns zero or more completed
    /// `AggregatedCandle`s in chronological order (shortest timeframe first).
    pub fn process_candle(&mut self, candle: &NormalizedCandle) -> Vec<AggregatedCandle> {
        let mut completed = Vec::with_capacity(self.targets.len());
        for target in &mut self.targets {
            if let Some(c) = target.process(&self.symbol, candle) {
                completed.push(c);
            }
        }
        completed
    }
}

/// Spawn a background task that listens for source candle closes
/// and aggregates them into the configured higher-timeframe candles.
///
/// `target_durations_secs` is the list of aggregation targets
/// (e.g. `&[14400, 86400]` for 4h and 1d). Completed candles are sent
/// through `tx`.
pub fn spawn_candle_aggregator(
    symbol: String,
    mut rx_candles: broadcast::Receiver<NormalizedCandle>,
    tx: mpsc::Sender<AggregatedCandle>,
    target_durations_secs: Vec<u64>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut aggregator = match CandleAggregator::try_new(&symbol, &target_durations_secs) {
            Ok(a) => a,
            Err(e) => {
                eprintln!(
                    "Candle Aggregator [{}]: misconfigured targets — {e}, shutting down",
                    symbol
                );
                return;
            }
        };
        loop {
            match rx_candles.recv().await {
                Ok(candle) => {
                    for completed in aggregator.process_candle(&candle) {
                        let _ = tx.send(completed).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!(
                        "Candle Aggregator [{}]: Lagged by {} messages, resetting",
                        symbol, n
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    eprintln!(
                        "Candle Aggregator [{}]: source channel closed, shutting down",
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
            exchange: core_domain::normalized::Exchange::Hyperliquid,
            symbol: "TEST".to_string(),
            start_time_ms: start_ms,
            duration_ms: 60000,
            open: Decimal::from_f64(open).unwrap(),
            high: Decimal::from_f64(high).unwrap(),
            low: Decimal::from_f64(low).unwrap(),
            close: Decimal::from_f64(close).unwrap(),
            volume: Decimal::from(1),
            trades_count: 10,
            reconstructed: None,
        }
    }

    #[test]
    fn test_single_target_aggregation() {
        let mut agg = CandleAggregator::new("TEST", &[14400]);

        let c1 = make_candle(0, 100.0, 101.0, 102.0, 99.0);
        let c2 = make_candle(60000, 101.0, 103.0, 104.0, 100.0);

        let completed = agg.process_candle(&c1);
        assert!(completed.is_empty());

        let completed = agg.process_candle(&c2);
        assert!(completed.is_empty());

        let pending = &agg.targets[0].pending.as_ref().unwrap();
        assert_eq!(pending.candle.high.to_f64().unwrap(), 104.0);
        assert_eq!(pending.candle.low.to_f64().unwrap(), 99.0);
        assert_eq!(pending.source_count, 2);
    }

    #[test]
    fn test_multi_target_aggregation() {
        let mut agg = CandleAggregator::new("TEST", &[180, 300]);

        let c1 = make_candle(0, 100.0, 101.0, 102.0, 99.0);
        let c2 = make_candle(60000, 101.0, 103.0, 104.0, 100.0);
        let c3 = make_candle(120000, 103.0, 105.0, 106.0, 102.0);
        let c4 = make_candle(180000, 105.0, 107.0, 108.0, 104.0);

        let completed = agg.process_candle(&c1);
        assert!(completed.is_empty());
        agg.process_candle(&c2);
        agg.process_candle(&c3);
        let completed = agg.process_candle(&c4);

        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].timeframe_secs, 180);
    }

    #[test]
    fn test_rolling_interval_boundary() {
        let mut agg = CandleAggregator::new("TEST", &[300]);

        let c1 = make_candle(0, 100.0, 101.0, 101.0, 100.0);
        let c2 = make_candle(60000, 101.0, 102.0, 102.0, 100.0);
        let c3 = make_candle(120000, 102.0, 103.0, 104.0, 101.0);
        let c4 = make_candle(180000, 103.0, 105.0, 105.0, 100.0);
        let c5 = make_candle(240000, 105.0, 106.0, 106.0, 104.0);
        let c6 = make_candle(300000, 106.0, 107.0, 107.0, 105.0);

        for c in [c1, c2, c3, c4, c5] {
            let completed = agg.process_candle(&c);
            assert!(completed.is_empty(), "no completion before boundary");
        }

        let completed = agg.process_candle(&c6);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].timeframe_secs, 300);
        assert_eq!(completed[0].candle.start_time_ms, 0);
    }

    #[test]
    fn test_ohlcv_invariants_preserved() {
        let mut agg = CandleAggregator::new("TEST", &[180]);

        let c1 = make_candle(0, 100.0, 102.0, 105.0, 98.0);
        let c2 = make_candle(60000, 102.0, 101.0, 104.0, 99.0);
        let c3 = make_candle(120000, 101.0, 103.0, 106.0, 97.0);
        let c4 = make_candle(180000, 103.0, 100.0, 100.0, 96.0);

        agg.process_candle(&c1);
        agg.process_candle(&c2);
        agg.process_candle(&c3);
        let completed = agg.process_candle(&c4);

        assert_eq!(completed.len(), 1);
        let ac = &completed[0];
        assert_eq!(ac.candle.open.to_f64().unwrap(), 100.0);
        assert_eq!(ac.candle.high.to_f64().unwrap(), 106.0);
        assert_eq!(ac.candle.low.to_f64().unwrap(), 97.0);
        assert_eq!(ac.candle.close.to_f64().unwrap(), 103.0);
        assert_eq!(ac.candle.volume.to_f64().unwrap(), 3.0);
        assert_eq!(ac.candle.trades_count, 30);
    }

    #[test]
    fn test_deduplication_sorts_targets() {
        let agg = CandleAggregator::new("TEST", &[900, 180, 300, 180]);
        assert_eq!(agg.targets.len(), 3);
        assert_eq!(agg.targets[0].duration_secs, 180);
        assert_eq!(agg.targets[1].duration_secs, 300);
        assert_eq!(agg.targets[2].duration_secs, 900);
    }
}
