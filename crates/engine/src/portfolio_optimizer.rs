//! Portfolio Optimization (Phase 5).
//!
//! Layers Kelly Criterion sizing and Risk Parity allocation on top of the
//! deterministic IRML exposure tiers. Risk Parity (inverse-volatility) decides
//! *how much of the risk budget each pair receives*; Kelly decides *how much
//! capital to deploy per pair* given its realized edge (win rate + reward/risk).
//! When per-pair data is insufficient the optimizer falls back to the existing
//! IRML static base-allocation tiers.
//!
//! Pure combination math lives in [`PortfolioOptimizer::optimize`]; the async
//! [`PortfolioOptimizer::compute_allocation`] gathers volatilities from close
//! histories and win rates from the [`RiskEngine`].

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use shared::risk::kelly::kelly_fractional;
use shared::risk::risk_parity::equal_risk_contribution;

use crate::config::PortfolioConfig;
use crate::portfolio_risk::PortfolioRiskState;
use crate::risk_engine::RiskEngine;

/// Minimum number of close observations required to estimate volatility.
const MIN_VOL_SAMPLES: usize = 20;

/// Per-pair inputs to the portfolio optimizer.
#[derive(Debug, Clone)]
pub struct PairRiskInput {
    pub pair: String,
    /// Return volatility (std-dev of simple returns) for the pair.
    pub volatility: f64,
    /// Beta-smoothed win rate `[0,1]` from IRML.
    pub win_rate: f64,
    /// Reward/risk ratio `R` (avg win / avg loss) from IRML.
    pub reward_risk_ratio: f64,
    /// IRML static base allocation percentage (fallback source).
    pub base_allocation_pct: f64,
}

/// Recommended per-pair capital allocation for the portfolio.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortfolioAllocation {
    /// pair -> allocation percentage of total capital.
    pub allocations: HashMap<String, f64>,
    /// "kelly_risk_parity" | "static".
    pub method: String,
}

pub struct PortfolioOptimizer {
    cfg: PortfolioConfig,
}

impl PortfolioOptimizer {
    pub fn new(cfg: PortfolioConfig) -> Self {
        Self { cfg }
    }

    /// Pure combination core: given per-pair risk inputs, produce allocations.
    ///
    /// Uses Kelly-weighted Risk Parity when the configured method requests it
    /// and inputs are sufficient; otherwise falls back to IRML static tiers.
    pub fn optimize(
        &self,
        pairs: &[PairRiskInput],
        risk_state: &PortfolioRiskState,
    ) -> PortfolioAllocation {
        // Per-pair cap is the tighter of the portfolio config and IRML state.
        let max_cap = self
            .cfg
            .max_allocation_pct
            .min(risk_state.max_single_pair_exposure_pct.max(0.0));
        let min_cap = self.cfg.min_allocation_pct.min(max_cap).max(0.0);

        let use_kelly = self.cfg.allocation_method == "kelly_risk_parity";
        let has_volatility = !pairs.is_empty()
            && pairs.iter().all(|p| p.volatility > 0.0);

        if !use_kelly || !has_volatility {
            return self.static_allocation(pairs, min_cap, max_cap);
        }

        // Risk Parity: inverse-volatility weights (sum to 1).
        let vols: Vec<f64> = pairs.iter().map(|p| p.volatility).collect();
        let rp_weights = equal_risk_contribution(&vols);

        // Kelly-scale each risk-parity weight by the pair's edge.
        let raw: Vec<f64> = pairs
            .iter()
            .zip(rp_weights.iter())
            .map(|(p, &w)| {
                let k = kelly_fractional(
                    p.win_rate,
                    p.reward_risk_ratio,
                    self.cfg.kelly_fraction,
                );
                w * k
            })
            .collect();

        let max_raw = raw.iter().cloned().fold(0.0_f64, f64::max);
        if max_raw <= 0.0 {
            // No pair carries a positive edge — defer to static tiers.
            return self.static_allocation(pairs, min_cap, max_cap);
        }

        // Scale so the strongest pair receives the per-pair cap; floor the rest.
        let mut allocations = HashMap::new();
        for (p, &r) in pairs.iter().zip(raw.iter()) {
            let pct = ((r / max_raw) * max_cap).clamp(min_cap, max_cap);
            allocations.insert(p.pair.clone(), pct);
        }

        PortfolioAllocation {
            allocations,
            method: "kelly_risk_parity".to_string(),
        }
    }

    /// IRML static-tier fallback: clamp each pair's base allocation to bounds.
    fn static_allocation(
        &self,
        pairs: &[PairRiskInput],
        min_cap: f64,
        max_cap: f64,
    ) -> PortfolioAllocation {
        let allocations = pairs
            .iter()
            .map(|p| (p.pair.clone(), p.base_allocation_pct.clamp(min_cap, max_cap)))
            .collect();
        PortfolioAllocation {
            allocations,
            method: "static".to_string(),
        }
    }

    /// Gather per-pair volatilities (from close histories) and win rates (from
    /// the `RiskEngine`), then optimize. `pairs` maps pair_key -> symbol.
    pub async fn compute_allocation(
        &self,
        pool: &SqlitePool,
        risk_engine: &RiskEngine,
        pairs: &[(String, String)],
        pair_close_histories: &Arc<RwLock<HashMap<String, Vec<f64>>>>,
        risk_state: &PortfolioRiskState,
        base_allocation_pct: f64,
    ) -> PortfolioAllocation {
        let histories = pair_close_histories.read().await;
        let mut inputs = Vec::with_capacity(pairs.len());
        for (pair_key, symbol) in pairs {
            let volatility = histories
                .get(symbol)
                .or_else(|| histories.get(pair_key))
                .and_then(|closes| returns_volatility(closes))
                .unwrap_or(0.0);
            let rr = risk_engine.compute_reward_risk(pool, symbol).await;
            inputs.push(PairRiskInput {
                pair: pair_key.clone(),
                volatility,
                win_rate: rr.win_rate_estimate,
                reward_risk_ratio: rr.recommended_ratio,
                base_allocation_pct,
            });
        }
        drop(histories);
        self.optimize(&inputs, risk_state)
    }
}

/// Std-dev of simple returns from a close-price series. Returns `None` when
/// there are too few samples to estimate volatility reliably.
pub fn returns_volatility(closes: &[f64]) -> Option<f64> {
    if closes.len() < MIN_VOL_SAMPLES {
        return None;
    }
    let returns: Vec<f64> = closes
        .windows(2)
        .filter_map(|w| {
            let prev = w[0];
            if prev != 0.0 {
                Some((w[1] - prev) / prev)
            } else {
                None
            }
        })
        .collect();
    if returns.len() < 2 {
        return None;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    let sd = var.sqrt();
    if sd.is_finite() && sd > 0.0 {
        Some(sd)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> PortfolioConfig {
        PortfolioConfig {
            kelly_fraction: 0.5,
            allocation_method: "kelly_risk_parity".to_string(),
            min_allocation_pct: 0.5,
            max_allocation_pct: 5.0,
        }
    }

    fn input(pair: &str, vol: f64, wr: f64, rr: f64) -> PairRiskInput {
        PairRiskInput {
            pair: pair.to_string(),
            volatility: vol,
            win_rate: wr,
            reward_risk_ratio: rr,
            base_allocation_pct: 1.0,
        }
    }

    #[test]
    fn kelly_risk_parity_allocates_within_bounds() {
        let opt = PortfolioOptimizer::new(cfg());
        let rs = PortfolioRiskState::default();
        let pairs = vec![
            input("BTC", 0.02, 0.6, 2.0),
            input("ETH", 0.04, 0.55, 1.8),
        ];
        let alloc = opt.optimize(&pairs, &rs);
        assert_eq!(alloc.method, "kelly_risk_parity");
        assert_eq!(alloc.allocations.len(), 2);
        for pct in alloc.allocations.values() {
            assert!(*pct >= 0.5 - 1e-9 && *pct <= 5.0 + 1e-9);
        }
    }

    #[test]
    fn lower_volatility_pair_gets_more() {
        let opt = PortfolioOptimizer::new(cfg());
        let rs = PortfolioRiskState::default();
        // Same edge, BTC lower vol -> higher allocation.
        let pairs = vec![
            input("BTC", 0.01, 0.6, 2.0),
            input("ETH", 0.05, 0.6, 2.0),
        ];
        let alloc = opt.optimize(&pairs, &rs);
        assert!(alloc.allocations["BTC"] > alloc.allocations["ETH"]);
    }

    #[test]
    fn missing_volatility_falls_back_to_static() {
        let opt = PortfolioOptimizer::new(cfg());
        let rs = PortfolioRiskState::default();
        let pairs = vec![input("BTC", 0.0, 0.6, 2.0)];
        let alloc = opt.optimize(&pairs, &rs);
        assert_eq!(alloc.method, "static");
    }

    #[test]
    fn negative_edge_falls_back_to_static() {
        let opt = PortfolioOptimizer::new(cfg());
        let rs = PortfolioRiskState::default();
        // Losing edge -> Kelly clamps to 0 for all -> static fallback.
        let pairs = vec![
            input("BTC", 0.02, 0.3, 1.0),
            input("ETH", 0.03, 0.2, 0.8),
        ];
        let alloc = opt.optimize(&pairs, &rs);
        assert_eq!(alloc.method, "static");
    }

    #[test]
    fn static_method_config_forces_static() {
        let mut c = cfg();
        c.allocation_method = "static".to_string();
        let opt = PortfolioOptimizer::new(c);
        let rs = PortfolioRiskState::default();
        let pairs = vec![input("BTC", 0.02, 0.6, 2.0)];
        let alloc = opt.optimize(&pairs, &rs);
        assert_eq!(alloc.method, "static");
    }

    #[test]
    fn empty_pairs_is_static_empty() {
        let opt = PortfolioOptimizer::new(cfg());
        let rs = PortfolioRiskState::default();
        let alloc = opt.optimize(&[], &rs);
        assert_eq!(alloc.method, "static");
        assert!(alloc.allocations.is_empty());
    }

    #[test]
    fn volatility_of_flat_series_is_none() {
        let flat = vec![100.0; 50];
        assert!(returns_volatility(&flat).is_none());
    }

    #[test]
    fn volatility_of_short_series_is_none() {
        assert!(returns_volatility(&[1.0, 2.0, 3.0]).is_none());
    }

    #[test]
    fn volatility_is_positive_for_varying_series() {
        let series: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64 * 0.7).sin() * 5.0).collect();
        let v = returns_volatility(&series);
        assert!(v.is_some());
        assert!(v.unwrap() > 0.0);
    }
}
