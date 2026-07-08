//! Stateful engine for the Institutional Risk Management Layer (IRML).
//!
//! Wraps the deterministic compute core (`shared::risk`) with DB-backed
//! behavioral state, the drawdown state machine, and the per-pair adaptive
//! Reward/Risk block calibrator. Persists `risk_events` and `rr_calibration`.
//!
//! Scope is **per-pair** (behavioral + R:R aggregate per trading pair). R:R is
//! **advisory** — it is surfaced to the AI but does not hard-block execution.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use sqlx::SqlitePool;

use shared::decision_context::DecisionContext;
use shared::indicators::normalized::NormalizedIndicatorValue;
use shared::market_context::MarketContext;
use shared::risk::{
    BehavioralInputs, DrawdownState, LiquidityInputs, RewardRiskRecommendation, RiskComputeParams,
    RiskLevel, RiskProfile, RiskTrend,
};
use shared::statistics::statistical_context::StatisticalContext;

use crate::config::RiskConfig;
use crate::safety::SafetyManager;

pub struct RiskEngine {
    cfg: RiskConfig,
    suspend_threshold: u32,
    drawdown_limit_pct: f64,
}

/// Win/loss aggregates derived from a pair's realized-PnL series.
struct Outcomes {
    wins: u32,
    losses: u32,
    losing_streak: u32,
    winning_streak: u32,
    recent_win_rate: f64,
    recent_count: u32,
    drawdown_pct: f64,
}

impl RiskEngine {
    pub fn new(cfg: RiskConfig, suspend_threshold: u32, drawdown_limit_pct: f64) -> Self {
        Self {
            cfg,
            suspend_threshold,
            drawdown_limit_pct,
        }
    }

    fn category_weights(&self) -> [f64; 6] {
        let w = &self.cfg.category_weights;
        [w.market, w.structural, w.momentum, w.volatility, w.liquidity, w.behavioral]
    }

    /// Adaptive Reward/Risk recommendation for a pair (Section 12).
    pub async fn compute_reward_risk(&self, pool: &SqlitePool, symbol: &str) -> RewardRiskRecommendation {
        let pnls = crate::db::pair_realized_pnls(pool, symbol, self.cfg.rr_lookback_trades).await;
        let (wins, losses) = count_wins_losses(&pnls);
        RewardRiskRecommendation::compute(
            wins,
            losses,
            self.cfg.rr_prior_wins,
            self.cfg.rr_prior_losses,
            self.cfg.rr_safety_margin,
        )
    }

    /// Map a drawdown percentage to a drawdown state (Section 9).
    fn drawdown_state(&self, drawdown_pct: f64) -> DrawdownState {
        let limit = if self.drawdown_limit_pct > 0.0 { self.drawdown_limit_pct } else { 30.0 };
        if drawdown_pct >= limit {
            DrawdownState::Shutdown
        } else if drawdown_pct >= 20.0 {
            DrawdownState::Critical
        } else if drawdown_pct >= 10.0 {
            DrawdownState::Defensive
        } else if drawdown_pct >= 5.0 {
            DrawdownState::Recovery
        } else {
            DrawdownState::Normal
        }
    }

    async fn build_outcomes(&self, pool: &SqlitePool, symbol: &str) -> Outcomes {
        let pnls = crate::db::pair_realized_pnls(pool, symbol, 0).await;
        let (wins, losses) = count_wins_losses(&pnls);
        let (losing_streak, winning_streak) = tail_streaks(&pnls);

        // Recent window win rate (last 20 or fewer).
        let recent_n = pnls.len().min(20);
        let recent_slice = &pnls[pnls.len() - recent_n..];
        let (rw, rl) = count_wins_losses(recent_slice);
        let recent_total = rw + rl;
        let recent_win_rate = if recent_total > 0 {
            rw as f64 / recent_total as f64
        } else {
            0.5
        };

        // Per-pair drawdown from cumulative realized PnL vs peak equity.
        let initial = crate::db::pair_initial_capital(pool, symbol).await.unwrap_or(0.0);
        let drawdown_pct = compute_drawdown_pct(&pnls, initial);

        Outcomes {
            wins,
            losses,
            losing_streak,
            winning_streak,
            recent_win_rate,
            recent_count: recent_total,
            drawdown_pct,
        }
    }

    /// Full risk evaluation for a pair: computes the profile with DB-backed
    /// behavioral / R:R / drawdown inputs, applies trend enrichment + level
    /// hysteresis from the previous persisted event, then persists a new
    /// `risk_event` and reconciles the R:R block ledger.
    #[allow(clippy::too_many_arguments)]
    pub async fn evaluate(
        &self,
        pool: &SqlitePool,
        pair_key: &str,
        symbol: &str,
        timeframe_secs: i64,
        indicators: &HashMap<String, NormalizedIndicatorValue>,
        market: Option<&MarketContext>,
        decision: Option<&DecisionContext>,
        stats: Option<&StatisticalContext>,
        safety: Option<&SafetyManager>,
    ) -> RiskProfile {
        // Liquidity proxy from recent candle geometry.
        let ohlc = crate::db::pair_recent_ohlc(pool, symbol, timeframe_secs, 100).await;
        let atr = indicators.get("atr").map(|v| v.raw_value).unwrap_or(0.0);
        let liquidity = LiquidityInputs::from_ohlc(&ohlc, atr);

        let outcomes = self.build_outcomes(pool, symbol).await;
        let drawdown_state = self.drawdown_state(outcomes.drawdown_pct);

        let suspended = safety
            .map(|s| {
                let level = s.consecutive_losses.load(Ordering::Relaxed);
                level >= self.suspend_threshold
            })
            .unwrap_or(false);
        let consecutive_losses = safety
            .map(|s| s.consecutive_losses.load(Ordering::Relaxed))
            .unwrap_or(outcomes.losing_streak);

        let behavioral = BehavioralInputs {
            consecutive_losses,
            consecutive_wins: outcomes.winning_streak,
            recent_win_rate: outcomes.recent_win_rate,
            recent_trade_count: outcomes.recent_count,
            drawdown_pct: outcomes.drawdown_pct,
            suspend_threshold: self.suspend_threshold,
            drawdown_limit_pct: self.drawdown_limit_pct,
            suspended,
        };

        let reward_risk = RewardRiskRecommendation::compute(
            outcomes.wins,
            outcomes.losses,
            self.cfg.rr_prior_wins,
            self.cfg.rr_prior_losses,
            self.cfg.rr_safety_margin,
        );

        let params = RiskComputeParams {
            indicators,
            market,
            decision,
            stats,
            liquidity: &liquidity,
            behavioral: &behavioral,
            drawdown_state,
            reward_risk,
            category_weights: self.category_weights(),
            worst_case_lambda: self.cfg.worst_case_lambda,
            base_allocation_pct: 4.0,
        };
        let mut profile = RiskProfile::compute(&params);

        // Enrich overall level with hysteresis + trend from previous event.
        if let Some(prev) = crate::db::latest_risk_event(pool, pair_key).await {
            profile.overall_level = RiskLevel::with_hysteresis(
                profile.overall_risk,
                RiskLevel::from_score(prev.overall_risk),
                self.cfg.transition_hysteresis,
            );
            let trend = RiskTrend::from_delta(profile.overall_risk, prev.overall_risk);
            profile.market.trend = trend;
        }

        // Persist the new state event (best-effort).
        let now = chrono::Utc::now().timestamp_millis();
        crate::db::insert_risk_event(
            pool,
            pair_key,
            now,
            profile.overall_risk,
            profile.overall_level.as_str(),
            profile.drawdown_state.as_str(),
            profile.permission.as_str(),
            outcomes.losing_streak as i64,
            outcomes.winning_streak as i64,
            &profile.explanation,
        )
        .await;

        // Reconcile the adaptive R:R block ledger (incl. historical backfill).
        self.reconcile_blocks(pool, pair_key, symbol).await;

        profile
    }

    /// Append any newly-completed R:R calibration blocks. On first run this
    /// backfills every completed block from existing historical trades so old
    /// trades are taken into account (Section 12.5).
    pub async fn reconcile_blocks(&self, pool: &SqlitePool, pair_key: &str, symbol: &str) {
        let block_size = self.cfg.rr_block_size.max(1) as usize;
        let pnls = crate::db::pair_realized_pnls(pool, symbol, 0).await;
        let completed_blocks = (pnls.len() / block_size) as i64;
        let last_recorded = crate::db::latest_rr_block_index(pool, pair_key).await;

        if completed_blocks - 1 <= last_recorded {
            return;
        }

        let now = chrono::Utc::now().timestamp_millis();
        // Record every block from last_recorded+1 .. completed_blocks-1.
        for block_index in (last_recorded + 1)..completed_blocks {
            let start = block_index as usize * block_size;
            let end = start + block_size;
            let block = &pnls[start..end];
            let (wins, losses) = count_wins_losses(block);
            let net_block_pnl: f64 = block.iter().sum();
            // Cumulative-through-this-block win rate anchors the recommendation.
            let cum = &pnls[..end];
            let (cw, cl) = count_wins_losses(cum);
            let rr = RewardRiskRecommendation::compute(
                cw,
                cl,
                self.cfg.rr_prior_wins,
                self.cfg.rr_prior_losses,
                self.cfg.rr_safety_margin,
            );
            crate::db::insert_rr_calibration(
                pool,
                pair_key,
                block_index,
                wins as i64,
                losses as i64,
                rr.win_rate_estimate,
                rr.breakeven_ratio,
                rr.recommended_ratio,
                rr.confidence,
                net_block_pnl,
                now,
            )
            .await;
        }
    }
}

/// win = realized_pnl > 0, loss = realized_pnl < 0 (breakeven ignored).
fn count_wins_losses(pnls: &[f64]) -> (u32, u32) {
    let mut w = 0;
    let mut l = 0;
    for &p in pnls {
        if p > 0.0 {
            w += 1;
        } else if p < 0.0 {
            l += 1;
        }
    }
    (w, l)
}

/// Consecutive losing / winning streak at the tail (most recent trades).
fn tail_streaks(pnls: &[f64]) -> (u32, u32) {
    let mut losing = 0;
    let mut winning = 0;
    for &p in pnls.iter().rev() {
        if p < 0.0 {
            if winning > 0 {
                break;
            }
            losing += 1;
        } else if p > 0.0 {
            if losing > 0 {
                break;
            }
            winning += 1;
        } else {
            break;
        }
    }
    (losing, winning)
}

/// Peak-to-current drawdown percentage from a realized-PnL series and an
/// optional initial capital baseline.
fn compute_drawdown_pct(pnls: &[f64], initial_capital: f64) -> f64 {
    if pnls.is_empty() {
        return 0.0;
    }
    let base = if initial_capital > 0.0 { initial_capital } else { 0.0 };
    let mut equity = base;
    let mut peak = base;
    let mut max_dd = 0.0f64;
    for &p in pnls {
        equity += p;
        if equity > peak {
            peak = equity;
        }
        if peak > 0.0 {
            let dd = (peak - equity) / peak * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    // Current drawdown from the running peak.
    if peak > 0.0 {
        ((peak - equity) / peak * 100.0).max(0.0)
    } else {
        max_dd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wins_losses_counts() {
        let (w, l) = count_wins_losses(&[1.0, -2.0, 0.0, 3.0, -1.0]);
        assert_eq!(w, 2);
        assert_eq!(l, 2);
    }

    #[test]
    fn tail_streak_losing() {
        let (losing, winning) = tail_streaks(&[1.0, 1.0, -1.0, -1.0, -1.0]);
        assert_eq!(losing, 3);
        assert_eq!(winning, 0);
    }

    #[test]
    fn tail_streak_winning() {
        let (losing, winning) = tail_streaks(&[-1.0, 1.0, 1.0]);
        assert_eq!(losing, 0);
        assert_eq!(winning, 2);
    }

    #[test]
    fn drawdown_from_pnls() {
        // initial 100; +20 -> 120 (peak); -30 -> 90; dd = (120-90)/120 = 25%.
        let dd = compute_drawdown_pct(&[20.0, -30.0], 100.0);
        assert!((dd - 25.0).abs() < 1e-6);
    }
}
