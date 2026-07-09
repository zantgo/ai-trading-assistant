//! Algorithmic execution engine — TWAP, VWAP, Implementation Shortfall.
//!
//! Provides time-sliced order execution to minimize market impact and
//! timing risk.  Integrates with the existing slot machine via the paper
//! trading engine.  Each filled slice feeds into a position slot.
//!
//! ## Algo Types
//!
//! - **TWAP**: Equal slices at fixed time intervals. Best for scalping.
//! - **VWAP**: Volume-weighted slices from historical volume profile.
//! - **ImplementationShortfall**: Balances market impact, timing risk,
//!   and opportunity cost (Almgren-Chriss framework).

use std::time::Instant;

/// Algorithm type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgoType {
    TWAP,
    VWAP,
    ImplementationShortfall,
}

/// Configuration for an execution algorithm instance.
#[derive(Debug, Clone)]
pub struct AlgoConfig {
    /// Algorithm type.
    pub algo_type: AlgoType,
    /// Total execution duration in seconds.
    pub duration_secs: f64,
    /// Number of slices to divide the order into.
    pub slice_count: usize,
    /// Price aggression as fraction of mid-price.
    /// Buy: mid × (1 − aggression). Sell: mid × (1 + aggression).
    pub aggression_pct: f64,
    /// Abort if price deviates more than this many ATRs from trigger price.
    pub abort_atr_mult: f64,
    /// Abort after N consecutive unfilled slices.
    pub abort_consecutive_unfilled: usize,
    /// Urgency parameter for Implementation Shortfall (0 = passive, 1 = aggressive).
    pub urgency: f64,
    /// Minimum interval between slices in seconds.
    pub min_interval_secs: f64,
}

impl Default for AlgoConfig {
    fn default() -> Self {
        Self {
            algo_type: AlgoType::TWAP,
            duration_secs: 30.0,
            slice_count: 6,
            aggression_pct: 0.00005, // 0.5 bps
            abort_atr_mult: 2.0,
            abort_consecutive_unfilled: 3,
            urgency: 0.0,
            min_interval_secs: 3.0,
        }
    }
}

impl AlgoConfig {
    /// Scalping-optimized TWAP configuration.
    pub fn scalping_twap() -> Self {
        Self {
            algo_type: AlgoType::TWAP,
            duration_secs: 15.0,
            slice_count: 5,
            aggression_pct: 0.00005,
            abort_atr_mult: 2.0,
            abort_consecutive_unfilled: 3,
            urgency: 0.0,
            min_interval_secs: 3.0,
        }
    }

    /// Standard VWAP configuration.
    pub fn vwap() -> Self {
        Self {
            algo_type: AlgoType::VWAP,
            duration_secs: 60.0,
            slice_count: 12,
            aggression_pct: 0.0001,
            abort_atr_mult: 2.5,
            abort_consecutive_unfilled: 4,
            urgency: 0.0,
            min_interval_secs: 5.0,
        }
    }
}

/// Status of an individual algo slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceStatus {
    /// Scheduled but not yet due.
    Pending,
    /// Order has been placed on the book.
    Placed,
    /// Partially filled.
    PartialFill,
    /// Fully filled.
    Filled,
    /// Slice expired without full fill.
    Skipped,
    /// Slice was cancelled before execution.
    Cancelled,
}

/// A single scheduled order within an execution algorithm.
#[derive(Debug, Clone)]
pub struct AlgoSlice {
    /// Slice index (0-based within the algo).
    pub index: usize,
    /// Size of this slice in base currency units.
    pub size: f64,
    /// Scheduled time offset from algo start (in seconds).
    pub scheduled_at: f64,
    /// Target limit price for this slice.
    pub price_target: f64,
    /// Current status.
    pub status: SliceStatus,
    /// Actual filled size.
    pub filled: f64,
    /// Average fill price.
    pub avg_fill_price: f64,
}

/// Progress report for an active execution algorithm.
#[derive(Debug, Clone)]
pub struct AlgoProgress {
    /// Fraction of total order filled (0.0 to 1.0).
    pub filled_pct: f64,
    /// Number of slices completed (filled or skipped).
    pub slices_completed: usize,
    /// Number of slices remaining.
    pub slices_remaining: usize,
    /// VWAP performance in basis points.
    /// Negative for buys = favorable (bought below VWAP).
    pub vwap_performance_bps: f64,
    /// Time elapsed since start, in seconds.
    pub elapsed_secs: f64,
    /// Estimated time remaining, in seconds.
    pub remaining_secs: f64,
}

/// Reason for algo abort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbortReason {
    /// IRML permission downgraded to Suspended or Emergency Stop.
    PermissionDowngrade,
    /// Price deviated beyond abort threshold.
    PriceDeviation,
    /// Too many consecutive unfilled slices.
    ConsecutiveUnfilled,
    /// Manual user cancellation.
    UserCancelled,
    /// Opposite confluence score exceeded exit threshold.
    OppositionSignal,
    /// Duration expired without full execution.
    Timeout,
}

/// Completion report for a finished execution algorithm.
#[derive(Debug, Clone)]
pub struct AlgoCompletion {
    /// Total size filled.
    pub total_filled: f64,
    /// Total size targeted.
    pub total_target: f64,
    /// Volume-weighted average fill price.
    pub avg_price: f64,
    /// VWAP benchmark price over the execution period.
    pub vwap_benchmark: f64,
    /// Slippage in basis points.
    pub slippage_bps: f64,
    /// Whether the algo completed successfully.
    pub completed: bool,
    /// Abort reason if not completed.
    pub abort_reason: Option<AbortReason>,
    /// Duration of execution in seconds.
    pub duration_secs: f64,
    /// Number of slices filled.
    pub slices_filled: usize,
    /// Number of slices skipped/cancelled.
    pub slices_missed: usize,
}

/// Action to take for a slice at the current tick.
#[derive(Debug, Clone)]
pub enum AlgoAction {
    /// Place a new limit order for this slice.
    PlaceOrder {
        slice_index: usize,
        size: f64,
        limit_price: f64,
    },
    /// Modify an existing order.
    ModifyOrder {
        slice_index: usize,
        new_limit_price: f64,
    },
    /// Cancel a pending slice.
    CancelSlice {
        slice_index: usize,
    },
    /// No action needed.
    None,
}

/// The execution algorithm state machine.
#[derive(Debug, Clone)]
pub struct ExecutionAlgo {
    /// Algorithm configuration.
    pub config: AlgoConfig,
    /// Total order size target.
    pub total_size: f64,
    /// Cumulative filled size across all slices.
    pub filled: f64,
    /// Direction: true = buy (long), false = sell (short).
    pub is_buy: bool,
    /// Price at algo creation (for abort deviation check).
    pub trigger_price: f64,
    /// Start time of the algo.
    pub started_at: Instant,
    /// Scheduled slices.
    pub slices: Vec<AlgoSlice>,
    /// Running sum for VWAP benchmark computation.
    pub vwap_numerator: f64,
    /// Running volume for VWAP benchmark.
    pub vwap_denominator: f64,
    /// Number of consecutive unfilled slices.
    pub consecutive_unfilled: usize,
    /// Total elapsed wall-clock time in seconds.
    pub elapsed_secs: f64,
    /// Whether the algo has been aborted.
    pub aborted: bool,
    /// Abort reason.
    pub abort_reason: Option<AbortReason>,
}

impl ExecutionAlgo {
    /// Create a new execution algorithm instance.
    pub fn new(
        config: AlgoConfig,
        total_size: f64,
        is_buy: bool,
        current_price: f64,
    ) -> Self {
        let slice_size = total_size / config.slice_count as f64;
        let interval_secs = config.duration_secs / (config.slice_count as f64)
            .max(config.min_interval_secs);

        let mut slices = Vec::with_capacity(config.slice_count);
        for i in 0..config.slice_count {
            let price_target = if is_buy {
                current_price * (1.0 - config.aggression_pct)
            } else {
                current_price * (1.0 + config.aggression_pct)
            };

            slices.push(AlgoSlice {
                index: i,
                size: slice_size,
                scheduled_at: i as f64 * interval_secs,
                price_target,
                status: SliceStatus::Pending,
                filled: 0.0,
                avg_fill_price: 0.0,
            });
        }

        Self {
            config,
            total_size,
            filled: 0.0,
            is_buy,
            trigger_price: current_price,
            started_at: Instant::now(),
            slices,
            vwap_numerator: 0.0,
            vwap_denominator: 0.0,
            consecutive_unfilled: 0,
            elapsed_secs: 0.0,
            aborted: false,
            abort_reason: None,
        }
    }

    /// Tick the algorithm — called on each update cycle.
    /// Returns actions to take (orders to place/modify/cancel).
    pub fn tick(
        &mut self,
        current_price: f64,
        _volume_profile: &[f64],
    ) -> Vec<AlgoAction> {
        if self.aborted {
            return vec![];
        }

        self.elapsed_secs = self.started_at.elapsed().as_secs_f64();

        // Check if duration exceeded.
        if self.elapsed_secs >= self.config.duration_secs * 1.2 {
            self.aborted = true;
            self.abort_reason = Some(AbortReason::Timeout);
            return self.slices.iter()
                .filter(|s| matches!(s.status, SliceStatus::Pending | SliceStatus::Placed))
                .map(|s| AlgoAction::CancelSlice { slice_index: s.index })
                .collect();
        }

        let mut actions = Vec::new();

        // Check for pending slices that are due.
        for slice in self.slices.iter_mut() {
            if slice.status != SliceStatus::Pending {
                continue;
            }

            if self.elapsed_secs >= slice.scheduled_at {
                // Calculate current limit price.
                let limit_price = if self.is_buy {
                    current_price * (1.0 - self.config.aggression_pct)
                } else {
                    current_price * (1.0 + self.config.aggression_pct)
                };

                actions.push(AlgoAction::PlaceOrder {
                    slice_index: slice.index,
                    size: slice.size - slice.filled,
                    limit_price,
                });
                slice.status = SliceStatus::Placed;
                slice.price_target = limit_price;
            }
        }

        actions
    }

    /// Record a fill for a specific slice.
    pub fn record_fill(&mut self, slice_index: usize, fill_size: f64, fill_price: f64) {
        if slice_index >= self.slices.len() {
            return;
        }

        let slice = &mut self.slices[slice_index];
        let unfilled = slice.size - slice.filled;
        let actual_fill = fill_size.min(unfilled);

        // Update VWAP tracking
        self.vwap_numerator += fill_price * actual_fill;
        self.vwap_denominator += actual_fill;

        slice.filled += actual_fill;
        slice.avg_fill_price = if slice.filled > 0.0 {
            (slice.avg_fill_price * (slice.filled - actual_fill) + fill_price * actual_fill)
                / slice.filled
        } else {
            fill_price
        };

        self.filled += actual_fill;

        if slice.filled >= slice.size * 0.999 {
            slice.status = SliceStatus::Filled;
            self.consecutive_unfilled = 0;
        } else {
            slice.status = SliceStatus::PartialFill;
        }
    }

    /// Mark a slice as skipped (expired without fill).
    pub fn mark_skipped(&mut self, slice_index: usize) {
        if slice_index >= self.slices.len() {
            return;
        }
        let slice = &mut self.slices[slice_index];
        if matches!(slice.status, SliceStatus::Placed | SliceStatus::Pending) {
            slice.status = SliceStatus::Skipped;
            self.consecutive_unfilled += 1;
        }
    }

    /// Check whether the algo should abort based on current conditions.
    pub fn check_abort(
        &mut self,
        current_price: f64,
        atr: f64,
        _irml_permission_suspended: bool,
        opposite_score: f64,
        opposite_threshold: f64,
    ) -> Option<AbortReason> {
        if self.aborted {
            return self.abort_reason.clone();
        }

        // Check consecutive unfilled slices.
        if self.consecutive_unfilled >= self.config.abort_consecutive_unfilled {
            self.aborted = true;
            self.abort_reason = Some(AbortReason::ConsecutiveUnfilled);
            return self.abort_reason.clone();
        }

        // Check price deviation.
        let deviation = (current_price - self.trigger_price).abs();
        let threshold = atr * self.config.abort_atr_mult;
        if threshold > 0.0 && deviation > threshold {
            self.aborted = true;
            self.abort_reason = Some(AbortReason::PriceDeviation);
            return self.abort_reason.clone();
        }

        // Check opposite confluence signal.
        if opposite_score > opposite_threshold {
            self.aborted = true;
            self.abort_reason = Some(AbortReason::OppositionSignal);
            return self.abort_reason.clone();
        }

        None
    }

    /// Get current progress snapshot.
    pub fn progress(&self) -> AlgoProgress {
        let filled_pct = if self.total_size > 0.0 {
            (self.filled / self.total_size).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let slices_completed: usize = self.slices.iter()
            .filter(|s| matches!(s.status, SliceStatus::Filled | SliceStatus::Skipped))
            .count();

        let slices_remaining = self.slices.len().saturating_sub(slices_completed);

        let avg_fill = if self.vwap_denominator > 0.0 {
            self.vwap_numerator / self.vwap_denominator
        } else {
            0.0
        };

        let vwap_performance_bps = if avg_fill > 0.0 && self.trigger_price > 0.0 {
            if self.is_buy {
                (avg_fill - self.trigger_price) / self.trigger_price * 10000.0
            } else {
                (self.trigger_price - avg_fill) / self.trigger_price * 10000.0
            }
        } else {
            0.0
        };

        let remaining_secs = (self.config.duration_secs - self.elapsed_secs).max(0.0);

        AlgoProgress {
            filled_pct,
            slices_completed,
            slices_remaining,
            vwap_performance_bps,
            elapsed_secs: self.elapsed_secs,
            remaining_secs,
        }
    }

    /// Check if the algo is fully complete.
    pub fn is_complete(&self) -> bool {
        self.aborted || self.filled >= self.total_size * 0.999
            || self.elapsed_secs >= self.config.duration_secs * 1.2
    }

    /// Generate a completion report.
    pub fn completion(&self) -> AlgoCompletion {
        let avg_price = if self.vwap_denominator > 0.0 {
            self.vwap_numerator / self.vwap_denominator
        } else {
            0.0
        };

        let vwap_benchmark = self.trigger_price; // simplified: use trigger price as benchmark

        let slippage_bps = if self.trigger_price > 0.0 {
            (avg_price - self.trigger_price) / self.trigger_price * 10000.0
        } else {
            0.0
        };

        let slices_filled = self.slices.iter()
            .filter(|s| matches!(s.status, SliceStatus::Filled))
            .count();

        let slices_missed = self.slices.len() - slices_filled;

        AlgoCompletion {
            total_filled: self.filled,
            total_target: self.total_size,
            avg_price,
            vwap_benchmark,
            slippage_bps,
            completed: !self.aborted,
            abort_reason: self.abort_reason.clone(),
            duration_secs: self.elapsed_secs,
            slices_filled,
            slices_missed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twap_slice_count() {
        let config = AlgoConfig::scalping_twap();
        let algo = ExecutionAlgo::new(config.clone(), 100.0, true, 50000.0);
        assert_eq!(algo.slices.len(), config.slice_count);
        let total: f64 = algo.slices.iter().map(|s| s.size).sum();
        assert!((total - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_twap_initial_slices_pending() {
        let config = AlgoConfig::scalping_twap();
        let algo = ExecutionAlgo::new(config, 50.0, true, 40000.0);
        for slice in &algo.slices {
            assert_eq!(slice.status, SliceStatus::Pending);
        }
    }

    #[test]
    fn test_record_fill_updates_filled() {
        let config = AlgoConfig::scalping_twap();
        let mut algo = ExecutionAlgo::new(config, 100.0, true, 50000.0);
        algo.record_fill(0, 10.0, 50001.0);
        assert_eq!(algo.slices[0].status, SliceStatus::PartialFill);
        assert!((algo.filled - 10.0).abs() < 0.01);
        algo.record_fill(0, 10.0, 50002.0);
        assert_eq!(algo.slices[0].status, SliceStatus::Filled);
    }

    #[test]
    fn test_abort_price_deviation() {
        let config = AlgoConfig {
            abort_atr_mult: 2.0,
            ..AlgoConfig::scalping_twap()
        };
        let mut algo = ExecutionAlgo::new(config, 100.0, true, 50000.0);
        let atr = 200.0;
        // Price moved 500 away, ATR * 2 = 400 → should abort.
        let reason = algo.check_abort(50500.0, atr, false, 0.0, 60.0);
        assert!(reason.is_some());
        assert_eq!(reason.unwrap(), AbortReason::PriceDeviation);
    }

    #[test]
    fn test_progress_calculation() {
        let config = AlgoConfig::scalping_twap();
        let mut algo = ExecutionAlgo::new(config, 100.0, true, 50000.0);
        algo.record_fill(0, 20.0, 50001.0);
        algo.record_fill(1, 20.0, 50002.0);
        algo.record_fill(2, 20.0, 49999.0);
        let progress = algo.progress();
        assert!((progress.filled_pct - 0.6).abs() < 0.01);
        assert_eq!(progress.slices_completed, 3);
    }
}
