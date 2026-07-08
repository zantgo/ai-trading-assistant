//! Signal lifecycle tracker — enables Potential → Confirmed → Active transitions
//! for non-divergence signal types. Currently only divergences have stateful tracking;
//! this module extends lifecycle support to all 12 SignalKind types.

use crate::indicators::normalized::{IndicatorSignal, SignalKind, SignalStatus};
use std::collections::HashMap;

/// Tracks signal state across bars for a single indicator key.
#[derive(Debug, Clone)]
struct PerKeyTracker {
    /// Active signals that have been emitted but not yet confirmed.
    pending: Vec<PendingSignal>,
    /// Counter for how many bars since last signal emission.
    bars_since_emission: u32,
}

#[derive(Debug, Clone)]
struct PendingSignal {
    label: String,
    kind: SignalKind,
    direction: String,
    bars_pending: u32,
}

/// Global signal lifecycle tracker. Manages per-key state across candles.
#[derive(Debug, Clone, Default)]
pub struct SignalLifecycleTracker {
    keys: HashMap<String, PerKeyTracker>,
}

impl SignalLifecycleTracker {
    pub fn new() -> Self {
        Self { keys: HashMap::new() }
    }

    /// Advance all pending signals by one bar. Signals pending for more than
    /// 3 bars without confirmation are expired (discarded).
    pub fn advance_bar(&mut self) {
        for tracker in self.keys.values_mut() {
            tracker.bars_since_emission = tracker.bars_since_emission.saturating_add(1);
            tracker.pending.retain_mut(|ps| {
                ps.bars_pending = ps.bars_pending.saturating_add(1);
                ps.bars_pending <= 3
            });
        }
    }

    /// Process new signals for a given indicator key. Returns the potentially
    /// upgraded signal list with lifecycle statuses applied.
    pub fn process_signals(
        &mut self,
        key: &str,
        signals: &[IndicatorSignal],
        confirm_condition: bool,
    ) -> Vec<IndicatorSignal> {
        let tracker = self.keys.entry(key.to_string()).or_insert_with(|| PerKeyTracker {
            pending: Vec::new(),
            bars_since_emission: 0,
        });

        tracker.bars_since_emission = 0;
        let mut output = Vec::new();

        // Check if any pending signals should be confirmed
        if confirm_condition && !tracker.pending.is_empty() {
            for ps in tracker.pending.drain(..) {
                output.push(IndicatorSignal::new(
                    ps.kind,
                    parse_direction(&ps.direction),
                    SignalStatus::Confirmed,
                    &format!("CONFIRMED_{}", ps.label),
                ));
            }
        }

        // Add current signals with appropriate status
        for sig in signals {
            if sig.status == SignalStatus::Confirmed {
                output.push(sig.clone());
                continue;
            }
            let is_new = !tracker
                .pending
                .iter()
                .any(|ps| ps.label == sig.label);
            if is_new {
                tracker.pending.push(PendingSignal {
                    label: sig.label.clone(),
                    kind: sig.kind,
                    direction: format!("{:?}", sig.direction),
                    bars_pending: 0,
                });
            }
            output.push(IndicatorSignal {
                status: SignalStatus::Potential,
                ..sig.clone()
            });
        }

        output
    }
}

fn parse_direction(d: &str) -> crate::indicators::normalized::SignalDirection {
    match d {
        "Bullish" => crate::indicators::normalized::SignalDirection::Bullish,
        "Bearish" => crate::indicators::normalized::SignalDirection::Bearish,
        _ => crate::indicators::normalized::SignalDirection::Neutral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::normalized::{IndicatorSignal, SignalDirection, SignalKind, SignalStatus};

    fn make_signal(label: &str, kind: SignalKind) -> IndicatorSignal {
        IndicatorSignal {
            kind,
            direction: SignalDirection::Bullish,
            status: SignalStatus::Active,
            label: label.to_string(),
            strength: 0.5,
            age_bars: 0,
            points: None,
        }
    }

    #[test]
    fn test_potential_to_confirmed_lifecycle() {
        let mut tracker = SignalLifecycleTracker::new();
        let signals = vec![make_signal("BREAKOUT_TEST", SignalKind::Breakout)];
        let out = tracker.process_signals("test_key", &signals, false);
        assert_eq!(out[0].status, SignalStatus::Potential);

        tracker.advance_bar();
        let out2 = tracker.process_signals("test_key", &[], true);
        assert!(!out2.is_empty());
        assert_eq!(out2[0].status, SignalStatus::Confirmed);
    }

    #[test]
    fn test_expiry_after_3_bars() {
        let mut tracker = SignalLifecycleTracker::new();
        let signals = vec![make_signal("EXPIRY_TEST", SignalKind::Crossover)];
        tracker.process_signals("test_key", &signals, false);
        for _ in 0..4 {
            tracker.advance_bar();
        }
        let out = tracker.process_signals("test_key", &[], false);
        assert!(out.is_empty());
    }
}
