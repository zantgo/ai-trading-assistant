use core_domain::performance::{
    OverallRating, PerformanceMatrixRow, PerformanceMatrixSummary, RegimeCompatibility,
    RegimeStrengthEntry, TradeAnalyticsRecord,
};
use sqlx::SqlitePool;

const TRADING_DAYS_PER_YEAR: f64 = 365.0;

pub async fn compute_performance_matrix(
    pool: &SqlitePool,
    trades: &[TradeAnalyticsRecord],
) -> Vec<PerformanceMatrixRow> {
    if trades.is_empty() {
        return vec![];
    }

    let mut by_policy: std::collections::HashMap<String, Vec<&TradeAnalyticsRecord>> =
        std::collections::HashMap::new();
    for t in trades {
        by_policy
            .entry(t.trigger_source.clone())
            .or_default()
            .push(t);
    }

    let mut results = Vec::new();

    for (policy_id, policy_trades) in &by_policy {
        let mut by_regime: std::collections::HashMap<String, Vec<&TradeAnalyticsRecord>> =
            std::collections::HashMap::new();
        for t in policy_trades {
            let regime = resolve_regime_for_trade(pool, t).await;
            by_regime.entry(regime).or_default().push(*t);
        }

        for (regime, regime_trades) in &by_regime {
            let trade_count = regime_trades.len() as u32;
            let wins = regime_trades.iter().filter(|t| t.net_pnl > 0.0).count();
            let win_rate = if trade_count > 0 {
                wins as f64 / trade_count as f64
            } else {
                0.0
            };

            let gross_profit: f64 = regime_trades
                .iter()
                .filter(|t| t.net_pnl > 0.0)
                .map(|t| t.net_pnl)
                .sum();
            let gross_loss: f64 = regime_trades
                .iter()
                .filter(|t| t.net_pnl < 0.0)
                .map(|t| t.net_pnl.abs())
                .sum();
            let profit_factor = if gross_loss > 0.0 {
                let v = gross_profit / gross_loss;
                if v.is_finite() {
                    Some(v)
                } else {
                    None
                }
            } else if gross_profit > 0.0 {
                None
            } else {
                Some(0.0)
            };

            let total_pnl: f64 = regime_trades.iter().map(|t| t.net_pnl).sum();
            let r_multiples: Vec<f64> = regime_trades
                .iter()
                .filter_map(|t| {
                    let allocated = t.entry_price * t.size;
                    if allocated > 0.0 {
                        Some(t.net_pnl / allocated)
                    } else {
                        None
                    }
                })
                .collect();
            let avg_r_multiple = if !r_multiples.is_empty() {
                r_multiples.iter().sum::<f64>() / r_multiples.len() as f64
            } else {
                0.0
            };

            let compatibility_label =
                classify_regime_compatibility(profit_factor, win_rate, trade_count);

            results.push(PerformanceMatrixRow {
                setup_type: policy_id.clone(),
                regime: regime.clone(),
                trade_count,
                win_rate,
                profit_factor,
                avg_r_multiple,
                total_pnl,
                compatibility_label,
            });
        }
    }

    results
}

pub async fn compute_performance_matrix_summary(
    pool: &SqlitePool,
    trades: &[TradeAnalyticsRecord],
) -> Vec<PerformanceMatrixSummary> {
    if trades.is_empty() {
        return vec![];
    }

    let per_regime_rows = compute_performance_matrix(pool, trades).await;
    let mut rows_by_policy: std::collections::HashMap<String, Vec<PerformanceMatrixRow>> =
        std::collections::HashMap::new();
    for row in &per_regime_rows {
        rows_by_policy
            .entry(row.setup_type.clone())
            .or_default()
            .push(row.clone());
    }

    let mut by_policy: std::collections::HashMap<String, Vec<&TradeAnalyticsRecord>> =
        std::collections::HashMap::new();
    for t in trades {
        by_policy
            .entry(t.trigger_source.clone())
            .or_default()
            .push(t);
    }

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut summaries = Vec::new();

    for (policy_id, policy_trades) in &by_policy {
        let regime_rows = rows_by_policy.get(policy_id).cloned().unwrap_or_default();

        let wins: Vec<_> = policy_trades.iter().filter(|t| t.net_pnl > 0.0).collect();
        let losses: Vec<_> = policy_trades.iter().filter(|t| t.net_pnl < 0.0).collect();

        let gross_profit: f64 = wins.iter().map(|t| t.net_pnl).sum();
        let gross_loss: f64 = losses.iter().map(|t| t.net_pnl.abs()).sum();

        let overall_profit_factor = if gross_loss > 0.0 {
            let v = gross_profit / gross_loss;
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        } else if gross_profit > 0.0 {
            None
        } else {
            Some(0.0)
        };

        let avg_win = if !wins.is_empty() {
            wins.iter().map(|t| t.net_pnl).sum::<f64>() / wins.len() as f64
        } else {
            0.0
        };
        let avg_loss = if !losses.is_empty() {
            losses.iter().map(|t| t.net_pnl.abs()).sum::<f64>() / losses.len() as f64
        } else {
            0.0
        };
        let win_rate = if !policy_trades.is_empty() {
            wins.len() as f64 / policy_trades.len() as f64
        } else {
            0.0
        };

        let overall_expectancy = (win_rate * avg_win) - ((1.0 - win_rate) * avg_loss);

        let daily_returns = compute_policy_daily_returns(policy_trades);
        let (overall_sharpe, overall_sortino) = compute_policy_risk_metrics(&daily_returns);

        let max_drawdown_pct = compute_policy_drawdown(policy_trades);

        let regime_strength_summary = build_regime_strength(&regime_rows);
        let optimization_recommendations = generate_recommendations(&regime_rows, policy_trades);

        let overall_rating = classify_overall_rating(
            overall_profit_factor,
            win_rate,
            overall_sharpe,
            policy_trades.len() as u32,
        );

        summaries.push(PerformanceMatrixSummary {
            setup_type: policy_id.clone(),
            total_trades: policy_trades.len() as u32,
            overall_profit_factor,
            overall_expectancy,
            overall_sharpe,
            overall_sortino,
            max_drawdown_pct,
            regime_compatibility: regime_rows,
            regime_strength_summary,
            optimization_recommendations,
            overall_rating,
            last_evaluated_at: now_ts,
        });
    }

    summaries
}

fn compute_policy_daily_returns(trades: &[&TradeAnalyticsRecord]) -> Vec<f64> {
    let mut daily: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();

    for t in trades {
        let secs = normalize_ts(t.exit_timestamp);
        let date = chrono::DateTime::from_timestamp(secs, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        *daily.entry(date).or_insert(0.0) += t.net_pnl;
    }

    daily.values().copied().collect()
}

fn compute_policy_risk_metrics(daily_returns: &[f64]) -> (Option<f64>, Option<f64>) {
    if daily_returns.len() < 2 {
        return (None, None);
    }

    let n = daily_returns.len() as f64;
    let mean = daily_returns.iter().sum::<f64>() / n;
    let variance = if n > 1.0 {
        daily_returns
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / (n - 1.0)
    } else {
        0.0
    };
    let daily_vol = variance.sqrt();

    let downside: Vec<f64> = daily_returns
        .iter()
        .filter(|&&r| r < 0.0)
        .copied()
        .collect();
    let dn = downside.len() as f64;
    let down_mean = if dn > 0.0 {
        downside.iter().sum::<f64>() / dn
    } else {
        0.0
    };
    let down_var = if dn > 1.0 {
        downside
            .iter()
            .map(|x| (x - down_mean).powi(2))
            .sum::<f64>()
            / (dn - 1.0)
    } else {
        0.0
    };
    let downside_dev = down_var.sqrt();

    let sharpe = if daily_vol > 0.0 {
        Some((mean / daily_vol) * TRADING_DAYS_PER_YEAR.sqrt())
    } else {
        None
    };

    let sortino = if downside_dev > 0.0 {
        Some((mean / downside_dev) * TRADING_DAYS_PER_YEAR.sqrt())
    } else {
        None
    };

    (sharpe, sortino)
}

fn compute_policy_drawdown(trades: &[&TradeAnalyticsRecord]) -> f64 {
    if trades.is_empty() {
        return 0.0;
    }

    let mut sorted = trades.to_vec();
    sorted.sort_by_key(|t| t.exit_timestamp);

    let mut peak = 0.0f64;
    let mut cumulative = 0.0f64;
    let mut max_dd_pct = 0.0f64;

    let initial_capital = sorted[0].entry_price * sorted[0].size;
    let base = if initial_capital > 0.0 {
        initial_capital
    } else {
        1.0
    };

    for t in &sorted {
        cumulative += t.net_pnl;
        let equity = base + cumulative;
        if equity > peak {
            peak = equity;
        }
        if peak > 0.0 {
            let dd = (peak - equity) / peak * 100.0;
            if dd > max_dd_pct {
                max_dd_pct = dd;
            }
        }
    }

    max_dd_pct
}

fn classify_regime_compatibility(
    profit_factor: Option<f64>,
    win_rate: f64,
    trade_count: u32,
) -> RegimeCompatibility {
    if trade_count < 5 {
        return RegimeCompatibility::Avoid;
    }
    let pf = profit_factor.unwrap_or(f64::INFINITY);
    if pf >= 1.5 && win_rate >= 0.55 {
        RegimeCompatibility::Strong
    } else if pf >= 1.2 && win_rate >= 0.45 {
        RegimeCompatibility::Favorable
    } else if pf >= 1.0 {
        RegimeCompatibility::Marginal
    } else {
        RegimeCompatibility::Avoid
    }
}

fn build_regime_strength(rows: &[PerformanceMatrixRow]) -> Vec<RegimeStrengthEntry> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|a, b| {
        let rank_a = regime_rank_score(&a.compatibility_label);
        let rank_b = regime_rank_score(&b.compatibility_label);
        rank_b.cmp(&rank_a).then_with(|| {
            b.profit_factor
                .unwrap_or(0.0)
                .partial_cmp(&a.profit_factor.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    sorted
        .iter()
        .enumerate()
        .map(|(i, row)| RegimeStrengthEntry {
            regime: row.regime.clone(),
            rank: (i + 1) as u32,
            strength: row.compatibility_label,
        })
        .collect()
}

fn regime_rank_score(label: &RegimeCompatibility) -> u32 {
    match label {
        RegimeCompatibility::Strong => 4,
        RegimeCompatibility::Favorable => 3,
        RegimeCompatibility::Marginal => 2,
        RegimeCompatibility::Avoid => 1,
    }
}

fn generate_recommendations(
    rows: &[PerformanceMatrixRow],
    trades: &[&TradeAnalyticsRecord],
) -> Vec<String> {
    let mut recs = Vec::new();

    let avoid_regimes: Vec<_> = rows
        .iter()
        .filter(|r| r.compatibility_label == RegimeCompatibility::Avoid && r.trade_count >= 5)
        .collect();
    for r in &avoid_regimes {
        recs.push(format!(
            "REGIME {}: Consistently unprofitable (PF={:.2}, WR={:.1}%) — consider disabling this regime",
            r.regime,
            r.profit_factor.unwrap_or(0.0),
            r.win_rate * 100.0,
        ));
    }

    let strong_regimes: Vec<_> = rows
        .iter()
        .filter(|r| r.compatibility_label == RegimeCompatibility::Strong)
        .collect();
    if let Some(strong) = strong_regimes.first() {
        let weak_alloc: Vec<_> = rows
            .iter()
            .filter(|r| r.compatibility_label != RegimeCompatibility::Strong && r.trade_count >= 5)
            .collect();
        if !weak_alloc.is_empty() {
            recs.push(format!(
                "ALLOCATION: Shift capital towards {} regime (PF={:.2}) from weaker regimes",
                strong.regime,
                strong.profit_factor.unwrap_or(0.0),
            ));
        }
    }

    if trades.len() >= 30 {
        let losing_trades = trades.iter().filter(|t| t.net_pnl < 0.0).count();
        let loss_rate = losing_trades as f64 / trades.len() as f64;
        if loss_rate > 0.6 {
            recs.push("HIGH LOSS RATE: Review entry criteria and stop-loss placement".into());
        }
    }

    let total_slippage: f64 = trades.iter().map(|t| t.execution_slippage.abs()).sum();
    let total_gross: f64 = trades.iter().map(|t| t.gross_pnl.abs()).sum();
    if total_gross > 0.0 && total_slippage / total_gross > 0.1 {
        recs.push(format!(
            "SLIPPAGE: High slippage overhead ({:.1}%) — consider limit orders or lower-latency execution",
            (total_slippage / total_gross) * 100.0,
        ));
    }

    recs
}

fn classify_overall_rating(
    profit_factor: Option<f64>,
    win_rate: f64,
    sharpe: Option<f64>,
    total_trades: u32,
) -> OverallRating {
    if total_trades < 10 {
        return OverallRating::Unrated;
    }

    let pf = profit_factor.unwrap_or(0.0);
    let sh = sharpe.unwrap_or(0.0);

    if pf >= 2.0 && win_rate >= 0.55 && sh >= 2.0 {
        OverallRating::Excellent
    } else if pf >= 1.5 && win_rate >= 0.50 && sh >= 1.0 {
        OverallRating::Good
    } else if pf >= 1.0 && win_rate >= 0.40 {
        OverallRating::Fair
    } else {
        OverallRating::Poor
    }
}

async fn resolve_regime_for_trade(pool: &SqlitePool, trade: &TradeAnalyticsRecord) -> String {
    let row: Result<Option<String>, _> = sqlx::query_scalar(
        "SELECT market_regime FROM market_snapshots
         WHERE symbol = ?1 AND timestamp <= ?2
         ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(&trade.symbol)
    .bind(trade.entry_timestamp)
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(regime)) => regime,
        _ => "UNKNOWN".to_string(),
    }
}

fn normalize_ts(ts: i64) -> i64 {
    if ts > 9_000_000_000 {
        ts / 1000
    } else {
        ts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::performance::{RegimeCompatibility, TradeAnalyticsRecord};

    fn make_trade(pnl: f64, trigger: &str) -> TradeAnalyticsRecord {
        TradeAnalyticsRecord {
            trade_id: format!("T-{}", rand_id()),
            symbol: "BTC-USDT".into(),
            direction: if pnl > 0.0 { "LONG" } else { "SHORT" }.into(),
            entry_timestamp: 1000,
            exit_timestamp: 2000,
            hold_time_seconds: 1,
            entry_price: 100.0,
            exit_price: 100.0 + pnl,
            size: 1.0,
            gross_pnl: pnl,
            net_pnl: pnl,
            roi_pct: pnl,
            execution_slippage: 0.0,
            mfe: pnl.max(0.0),
            mae: pnl.min(0.0),
            trigger_source: trigger.to_string(),
            exit_reason: "MANUAL".into(),
            flat_trade: pnl.abs() < 1e-10,
        }
    }

    fn rand_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn test_classify_strong_regime() {
        let label = classify_regime_compatibility(Some(2.0), 0.60, 10);
        assert_eq!(label, RegimeCompatibility::Strong);
    }

    #[test]
    fn test_classify_favorable_regime() {
        let label = classify_regime_compatibility(Some(1.3), 0.48, 8);
        assert_eq!(label, RegimeCompatibility::Favorable);
    }

    #[test]
    fn test_classify_marginal_regime() {
        let label = classify_regime_compatibility(Some(1.0), 0.50, 10);
        assert_eq!(label, RegimeCompatibility::Marginal);
    }

    #[test]
    fn test_classify_avoid_regime() {
        let label = classify_regime_compatibility(Some(0.8), 0.30, 10);
        assert_eq!(label, RegimeCompatibility::Avoid);
    }

    #[test]
    fn test_insufficient_sample_below_5() {
        let label = classify_regime_compatibility(Some(3.0), 0.80, 4);
        assert_eq!(label, RegimeCompatibility::Avoid);
    }

    #[test]
    fn test_insufficient_sample_exactly_5() {
        let label = classify_regime_compatibility(Some(2.0), 0.60, 5);
        assert_eq!(label, RegimeCompatibility::Strong);
    }

    #[test]
    fn test_regime_strength_sorting() {
        let rows = vec![
            PerformanceMatrixRow {
                setup_type: "P1".into(),
                regime: "AVOID_ME".into(),
                trade_count: 10,
                win_rate: 0.3,
                profit_factor: Some(0.5),
                avg_r_multiple: -0.1,
                total_pnl: -100.0,
                compatibility_label: RegimeCompatibility::Avoid,
            },
            PerformanceMatrixRow {
                setup_type: "P1".into(),
                regime: "STRONG_ME".into(),
                trade_count: 20,
                win_rate: 0.65,
                profit_factor: Some(2.5),
                avg_r_multiple: 0.5,
                total_pnl: 500.0,
                compatibility_label: RegimeCompatibility::Strong,
            },
            PerformanceMatrixRow {
                setup_type: "P1".into(),
                regime: "FAVORABLE_ME".into(),
                trade_count: 15,
                win_rate: 0.50,
                profit_factor: Some(1.4),
                avg_r_multiple: 0.2,
                total_pnl: 200.0,
                compatibility_label: RegimeCompatibility::Favorable,
            },
        ];
        let strength = build_regime_strength(&rows);
        assert_eq!(strength.len(), 3);
        assert_eq!(strength[0].regime, "STRONG_ME");
        assert_eq!(strength[0].rank, 1);
        assert_eq!(strength[1].regime, "FAVORABLE_ME");
        assert_eq!(strength[2].regime, "AVOID_ME");
    }

    #[test]
    fn test_overall_rating_excellent() {
        let rating = classify_overall_rating(Some(2.5), 0.60, Some(2.5), 50);
        assert_eq!(rating, OverallRating::Excellent);
    }

    #[test]
    fn test_overall_rating_good() {
        let rating = classify_overall_rating(Some(1.6), 0.55, Some(1.2), 30);
        assert_eq!(rating, OverallRating::Good);
    }

    #[test]
    fn test_overall_rating_fair() {
        let rating = classify_overall_rating(Some(1.1), 0.45, None, 20);
        assert_eq!(rating, OverallRating::Fair);
    }

    #[test]
    fn test_overall_rating_poor() {
        let rating = classify_overall_rating(Some(0.8), 0.30, None, 15);
        assert_eq!(rating, OverallRating::Poor);
    }

    #[test]
    fn test_overall_rating_unrated_few_trades() {
        let rating = classify_overall_rating(Some(3.0), 0.80, Some(5.0), 5);
        assert_eq!(rating, OverallRating::Unrated);
    }

    #[test]
    fn test_policy_drawdown_no_loss() {
        let trades: Vec<_> = (0..5).map(|_| make_trade(100.0, "P1")).collect();
        let refs: Vec<&TradeAnalyticsRecord> = trades.iter().collect();
        let dd = compute_policy_drawdown(&refs);
        assert_eq!(dd, 0.0);
    }

    #[test]
    fn test_policy_drawdown_with_dive() {
        let t1 = make_trade(200.0, "P1");
        let t2 = make_trade(-150.0, "P1");
        let t3 = make_trade(50.0, "P1");
        let trades = vec![t1, t2, t3];
        let refs: Vec<&TradeAnalyticsRecord> = trades.iter().collect();
        let dd = compute_policy_drawdown(&refs);
        assert!(dd > 0.0);
    }

    #[test]
    fn test_generate_recommendations_avoid_regime() {
        let rows = vec![PerformanceMatrixRow {
            setup_type: "P1".into(),
            regime: "BAD_REGIME".into(),
            trade_count: 10,
            win_rate: 0.25,
            profit_factor: Some(0.5),
            avg_r_multiple: -0.2,
            total_pnl: -200.0,
            compatibility_label: RegimeCompatibility::Avoid,
        }];
        let trades: Vec<_> = (0..10).map(|_| make_trade(-10.0, "P1")).collect();
        let refs: Vec<_> = trades.iter().collect();
        let recs = generate_recommendations(&rows, &refs);
        assert!(recs.iter().any(|r| r.contains("disabling")));
    }

    #[test]
    fn test_policy_daily_returns() {
        let t1 = make_trade(100.0, "P1");
        let t2 = make_trade(-50.0, "P1");
        let trades = vec![t1, t2];
        let refs: Vec<_> = trades.iter().collect();
        let returns = compute_policy_daily_returns(&refs);
        assert!(!returns.is_empty());
        assert!((returns.iter().sum::<f64>() - 50.0).abs() < 0.01);
    }
}
