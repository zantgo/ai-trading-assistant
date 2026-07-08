//! Online learning — incremental frequency tables and feature-outcome
//! history updated after each candle (Phase 7).
//!
//! The learner accumulates event-counts for all 8 probability events and
//! maintains a rolling window of (feature-vector, forward-return) pairs
//! for the feature-importance tracker.

use std::collections::{HashMap, VecDeque};

/// One (feature_vector, forward_return) training example.
#[derive(Debug, Clone)]
pub struct FeatureOutcome {
    pub features: Vec<f64>,
    pub forward_return: f64,
}

/// Incremental learner tracking event frequencies and feature-outcome pairs.
#[derive(Debug, Clone)]
pub struct OnlineLearner {
    /// event_name → (successes, failures)
    pub event_counts: HashMap<String, (usize, usize)>,
    /// Rolling history of (feature_vector, forward_outcome) for MI estimation.
    pub outcome_history: VecDeque<FeatureOutcome>,
    history_cap: usize,
}

impl OnlineLearner {
    pub fn new(history_cap: usize) -> Self {
        Self {
            event_counts: HashMap::new(),
            outcome_history: VecDeque::with_capacity(history_cap),
            history_cap,
        }
    }

    /// Record a binary event outcome.
    pub fn observe(&mut self, event: &str, outcome: bool) {
        let entry = self.event_counts.entry(event.to_string()).or_insert((0, 0));
        if outcome { entry.0 += 1; } else { entry.1 += 1; }
    }

    /// Return the empirical probability of an event, or `None` if not
    /// enough observations have been recorded (< 5 total).
    pub fn probability(&self, event: &str) -> Option<f64> {
        let (s, f) = self.event_counts.get(event).copied().unwrap_or((0, 0));
        let total = s + f;
        if total < 5 { None }
        else { Some(s as f64 / total as f64) }
    }

    /// Record a feature vector and the forward return that followed N bars
    /// later.  Used by `FeatureImportanceTracker`.
    pub fn observe_features(&mut self, features: Vec<f64>, forward_return: f64) {
        if self.outcome_history.len() >= self.history_cap {
            self.outcome_history.pop_front();
        }
        self.outcome_history.push_back(FeatureOutcome { features, forward_return });
    }

    /// Number of feature-outcome pairs recorded.
    pub fn feature_count(&self) -> usize {
        self.outcome_history.len()
    }
}

impl Default for OnlineLearner {
    fn default() -> Self {
        Self::new(500)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observe_and_probability() {
        let mut learner = OnlineLearner::new(100);
        assert!(learner.probability("test").is_none());
        learner.observe("test", true);
        learner.observe("test", true);
        learner.observe("test", false);
        learner.observe("test", true);
        learner.observe("test", true);
        let p = learner.probability("test").unwrap();
        assert!((p - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_feature_history_capped() {
        let mut learner = OnlineLearner::new(10);
        for i in 0..20 {
            learner.observe_features(vec![i as f64], 0.0);
        }
        assert_eq!(learner.feature_count(), 10);
    }
}
