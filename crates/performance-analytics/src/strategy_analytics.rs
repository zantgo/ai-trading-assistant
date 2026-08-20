use core_domain::performance::{
    PerformanceClassification, StrategyAnalyticsRow, TradeAnalyticsRecord,
};
use sqlx::SqlitePool;

const MC_RUNS: u32 = 10_000;
const MC_SEED: u64 = 42;

/// Significance bar: an edge is "statistically significant" only when both
/// the t-test p-value and the Monte Carlo p-value are below this threshold.
pub const ALPHA: f64 = 0.05;

/// The significance-treatment parameters (v7.3). Defaults reproduce the
/// legacy constants exactly; operators tune them via `[workspace.analytics]`
/// in config.toml.
#[derive(Debug, Clone, Copy)]
pub struct AnalyticsParams {
    pub alpha: f64,
    pub monte_carlo_runs: u32,
    pub min_trades_for_verdict: u32,
}

impl Default for AnalyticsParams {
    fn default() -> Self {
        Self {
            alpha: ALPHA,
            monte_carlo_runs: MC_RUNS,
            min_trades_for_verdict: 30,
        }
    }
}

/// Compute strategy-level analytics grouped by execution policy.
/// Implements docs:03-05-03-pae-layer2-strategy-analytics.md
pub async fn compute_strategy_analytics(
    _pool: &SqlitePool,
    trades: &[TradeAnalyticsRecord],
    params: AnalyticsParams,
) -> Vec<StrategyAnalyticsRow> {
    if trades.is_empty() {
        return vec![];
    }

    let mut by_setup: std::collections::HashMap<String, Vec<&TradeAnalyticsRecord>> =
        std::collections::HashMap::new();
    for t in trades {
        by_setup
            .entry(t.trigger_source.clone())
            .or_default()
            .push(t);
    }

    by_setup
        .into_iter()
        .map(|(setup_type, setup_trades)| compute_setup_analytics(&setup_type, &setup_trades, params))
        .collect()
}

/// Compute the NHST statistics block for one group of trades (setup type in
/// live analytics, the whole backtest in PAE L5). Shared by both paths.
pub fn compute_setup_analytics(
    setup_type: &str,
    trades: &[&TradeAnalyticsRecord],
    params: AnalyticsParams,
) -> StrategyAnalyticsRow {
    let total = trades.len() as u32;
    let wins: Vec<&TradeAnalyticsRecord> =
        trades.iter().filter(|t| t.net_pnl > 0.0).copied().collect();
    let losses: Vec<&TradeAnalyticsRecord> =
        trades.iter().filter(|t| t.net_pnl < 0.0).copied().collect();

    let win_count = wins.len() as u32;
    let loss_count = losses.len() as u32;
    let win_rate = if total > 0 {
        win_count as f64 / total as f64
    } else {
        0.0
    };

    let gross_profit: f64 = wins.iter().map(|t| t.net_pnl).sum();
    let gross_loss: f64 = losses.iter().map(|t| t.net_pnl.abs()).sum();

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

    let average_win = if win_count > 0 {
        gross_profit / win_count as f64
    } else {
        0.0
    };
    let average_loss = if loss_count > 0 {
        gross_loss / loss_count as f64
    } else {
        0.0
    };

    let avg_win_loss_ratio = if average_loss > 0.0 {
        average_win / average_loss
    } else {
        0.0
    };

    let expectancy = if total > 0 {
        (win_rate * average_win) - ((1.0 - win_rate) * average_loss)
    } else {
        0.0
    };

    let net_pnls: Vec<f64> = trades.iter().map(|t| t.net_pnl).collect();
    let n = net_pnls.len() as f64;
    let mean = if n > 0.0 {
        net_pnls.iter().sum::<f64>() / n
    } else {
        0.0
    };
    let variance = if n > 1.0 {
        net_pnls.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    let std_dev = variance.sqrt();

    let t_statistic = if std_dev > 0.0 && n > 1.0 {
        mean / (std_dev / n.sqrt())
    } else {
        0.0
    };

    let p_value = if n > 1.0 {
        one_tailed_t_pvalue(t_statistic, (n - 1.0) as u32)
    } else {
        1.0
    };

    let p_mc = monte_carlo_sign_randomization(&net_pnls, params.monte_carlo_runs, MC_SEED);

    let is_significant = p_value < params.alpha && p_mc < params.alpha;

    let slippage_overhead = if !trades.is_empty() {
        let total_gross: f64 = trades.iter().map(|t| t.gross_pnl.abs()).sum();
        let total_slippage: f64 = trades.iter().map(|t| t.execution_slippage.abs()).sum();
        if total_gross > 0.0 {
            (total_slippage / total_gross) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    let classification =
        classify_performance_with_params(profit_factor, win_rate, p_value, p_mc, total, params);

    StrategyAnalyticsRow {
        setup_type: setup_type.to_string(),
        alpha: params.alpha,
        total_trades: total,
        win_count,
        loss_count,
        win_rate,
        gross_profit,
        gross_loss,
        profit_factor,
        average_win,
        average_loss,
        avg_win_loss_ratio,
        expectancy,
        slippage_overhead,
        t_statistic,
        p_value,
        p_mc,
        monte_carlo_runs: params.monte_carlo_runs,
        is_significant,
        classification,
    }
}

fn classify_performance(
    profit_factor: Option<f64>,
    win_rate: f64,
    p_value: f64,
    p_mc: f64,
    total_trades: u32,
) -> PerformanceClassification {
    classify_performance_with_params(
        profit_factor,
        win_rate,
        p_value,
        p_mc,
        total_trades,
        AnalyticsParams::default(),
    )
}

/// Verdict classification with tunable significance treatment (v7.3). The
/// min-trade floor and the α bar come from `[workspace.analytics]`.
fn classify_performance_with_params(
    profit_factor: Option<f64>,
    win_rate: f64,
    p_value: f64,
    p_mc: f64,
    total_trades: u32,
    params: AnalyticsParams,
) -> PerformanceClassification {
    if total_trades < params.min_trades_for_verdict {
        return PerformanceClassification::InsufficientData;
    }

    let pf = profit_factor.unwrap_or(f64::INFINITY);

    if pf > 1.2 && win_rate > 0.50 && p_value < 0.01 && p_mc < 0.01 {
        PerformanceClassification::StrongEdge
    } else if pf > 1.5 && win_rate > 0.45 && p_value < params.alpha && p_mc < params.alpha {
        PerformanceClassification::ModerateEdge
    } else if pf >= 1.0 && p_value <= 0.10 {
        PerformanceClassification::WeakMarginalEdge
    } else {
        PerformanceClassification::NoEdgeNegative
    }
}

/// One-tailed Student t p-value.
/// p = 1 − Φ_{t, df}(t) where Φ is the CDF of the t-distribution.
fn one_tailed_t_pvalue(t: f64, df: u32) -> f64 {
    let x = df as f64 / (df as f64 + t * t);
    let bt = beta_incomplete(0.5 * df as f64, 0.5, x);
    if t >= 0.0 {
        0.5 * bt
    } else {
        1.0 - 0.5 * bt
    }
}

/// Incomplete beta function via continued fraction.
fn beta_incomplete(a: f64, b: f64, x: f64) -> f64 {
    if !(0.0..=1.0).contains(&x) {
        return 0.0;
    }
    if x == 0.0 || x == 1.0 {
        return x;
    }
    let bt = x.powf(a) * (1.0 - x).powf(b) / a;

    bt * beta_cf(a, b, x) / a
}

fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    let max_iter = 200;
    let eps = 3e-12;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < 1e-30 {
        d = 1e-30;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=max_iter {
        let mf = m as f64;
        let m2 = 2.0 * mf;

        let mut aa = mf * (b - mf) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        h *= d * c;

        aa = -(a + mf) * (qab + mf) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;

        if (del - 1.0).abs() < eps {
            break;
        }
    }
    h
}

/// Monte Carlo sign-randomization test.
/// H0: zero directional edge — each trade's sign is randomized.
/// p_mc = count(randomized_mean >= actual_mean) / N
fn monte_carlo_sign_randomization(pnls: &[f64], runs: u32, seed: u64) -> f64 {
    if pnls.is_empty() {
        return 1.0;
    }

    let actual_mean: f64 = pnls.iter().sum::<f64>() / pnls.len() as f64;

    let mut rng = XorShift64::new(seed);

    let mut count_exceed = 0u32;
    for _ in 0..runs {
        let randomized_mean = pnls
            .iter()
            .map(|&pnl| if rng.next() & 1 == 0 { pnl } else { -pnl })
            .sum::<f64>()
            / pnls.len() as f64;

        if randomized_mean >= actual_mean {
            count_exceed += 1;
        }
    }

    if count_exceed == 0 && runs > 0 {
        1.0 / runs as f64
    } else {
        count_exceed as f64 / runs as f64
    }
}

/// Simple XorShift64* PRNG for deterministic reproducibility.
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let s = if seed == 0 {
            0xDEAD_BEEF_CAFE_BABE
        } else {
            seed
        };
        Self { state: s }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::performance::TradeAnalyticsRecord;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TRADE_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn make_trade(net_pnl: f64, roi_pct: f64, trigger: &str) -> TradeAnalyticsRecord {
        let id = TRADE_ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        TradeAnalyticsRecord {
            trade_id: format!("T-{id}"),
            symbol: "BTC-USDT".into(),
            direction: "LONG".into(),
            entry_timestamp: 1000,
            exit_timestamp: 2000,
            hold_time_seconds: 1,
            entry_price: 100.0,
            exit_price: 100.0 + net_pnl,
            size: 1.0,
            gross_pnl: net_pnl,
            net_pnl,
            roi_pct,
            execution_slippage: 0.0,
            mfe: net_pnl.max(0.0),
            mae: net_pnl.min(0.0),
            trigger_source: trigger.to_string(),
            exit_reason: "MANUAL".into(),
            flat_trade: net_pnl.abs() < 1e-10,
        }
    }

    fn make_win(amount: f64, trigger: &str) -> TradeAnalyticsRecord {
        make_trade(amount, amount / 100.0 * 100.0, trigger)
    }

    fn make_loss(amount: f64, trigger: &str) -> TradeAnalyticsRecord {
        make_trade(-amount, -amount / 100.0 * 100.0, trigger)
    }

    #[test]
    fn test_empty_trades_returns_empty() {
        let trades: Vec<TradeAnalyticsRecord> = vec![];
        let result = compute_setup_analytics("POLICY_A", &trades.iter().collect::<Vec<_>>(), AnalyticsParams::default());
        assert_eq!(result.total_trades, 0);
        assert_eq!(result.win_count, 0);
        assert_eq!(result.loss_count, 0);
    }

    #[test]
    fn test_all_wins_high_profit_factor() {
        let trades: Vec<_> = (0..10).map(|_| make_win(50.0, "POLICY_A")).collect();
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        assert_eq!(row.total_trades, 10);
        assert_eq!(row.win_count, 10);
        assert_eq!(row.loss_count, 0);
        assert_eq!(row.win_rate, 1.0);
        assert!(row.profit_factor.is_none());
        assert!(row.expectancy > 0.0);
    }

    #[test]
    fn test_all_losses_zero_profit_factor() {
        let trades: Vec<_> = (0..10).map(|_| make_loss(30.0, "POLICY_A")).collect();
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        assert_eq!(row.total_trades, 10);
        assert_eq!(row.win_count, 0);
        assert_eq!(row.loss_count, 10);
        assert_eq!(row.win_rate, 0.0);
        assert_eq!(row.profit_factor, Some(0.0));
        assert!(row.expectancy < 0.0);
    }

    #[test]
    fn test_mixed_50_50_win_rate() {
        let mut trades: Vec<_> = (0..5).map(|_| make_win(60.0, "POLICY_A")).collect();
        trades.extend((0..5).map(|_| make_loss(30.0, "POLICY_A")));
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        assert_eq!(row.total_trades, 10);
        assert!((row.win_rate - 0.5).abs() < 0.01);
        assert!(row.profit_factor.unwrap() > 1.5);
        assert!(row.expectancy > 0.0);
    }

    #[test]
    fn test_profit_factor_mixed_positive() {
        let trades: Vec<_> = vec![
            make_win(100.0, "POLICY_A"),
            make_win(80.0, "POLICY_A"),
            make_win(60.0, "POLICY_A"),
            make_loss(50.0, "POLICY_A"),
            make_loss(40.0, "POLICY_A"),
        ];
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        let expected_pf = (100.0 + 80.0 + 60.0) / (50.0 + 40.0);
        assert!((row.profit_factor.unwrap() - expected_pf).abs() < 0.01);
        assert!(row.win_rate > 0.5);
    }

    #[test]
    fn test_expectancy_sign_consistent() {
        let trades: Vec<_> = vec![
            make_win(20.0, "POLICY_A"),
            make_win(20.0, "POLICY_A"),
            make_loss(10.0, "POLICY_A"),
            make_loss(10.0, "POLICY_A"),
        ];
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        let expected = (0.5 * 20.0) - (0.5 * 10.0);
        assert!((row.expectancy - expected).abs() < 0.01);
    }

    #[test]
    fn test_average_loss_stored_as_positive() {
        let trades: Vec<_> = vec![make_loss(10.0, "POLICY_A"), make_loss(20.0, "POLICY_A")];
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        assert!(row.average_loss > 0.0);
        assert!((row.average_loss - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_gross_loss_stored_as_positive() {
        let trades: Vec<_> = vec![make_loss(10.0, "POLICY_A"), make_loss(30.0, "POLICY_A")];
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        assert!(row.gross_loss > 0.0);
        assert!((row.gross_loss - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_insufficient_data_with_few_trades() {
        let trades: Vec<_> = vec![make_win(100.0, "POLICY_A"), make_loss(50.0, "POLICY_A")];
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        assert_eq!(
            row.classification,
            PerformanceClassification::InsufficientData
        );
    }

    #[test]
    fn test_strong_edge_classification() {
        let mut trades: Vec<_> = (0..35).map(|_| make_win(100.0, "POLICY_A")).collect();
        trades.extend((0..5).map(|_| make_loss(30.0, "POLICY_A")));
        let results = compute_strategy_analytics_from_trades(&trades);
        let row = &results[0];
        assert_eq!(row.classification, PerformanceClassification::StrongEdge);
        assert!(row.is_significant);
    }

    #[test]
    fn test_monte_carlo_deterministic() {
        let pnls = vec![10.0, -5.0, 15.0, -3.0, 8.0];
        let p1 = monte_carlo_sign_randomization(&pnls, 1000, 42);
        let p2 = monte_carlo_sign_randomization(&pnls, 1000, 42);
        assert!((p1 - p2).abs() < 1e-10);
    }

    #[test]
    fn test_monte_carlo_p_mc_range() {
        let pnls = vec![10.0, 8.0, 12.0, 5.0, 9.0, 7.0, 11.0, 6.0, 13.0, 4.0];
        let p = monte_carlo_sign_randomization(&pnls, 10000, 42);
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn test_t_statistic_zero_variance() {
        let pnls = [5.0, 5.0, 5.0];
        let n = pnls.len() as f64;
        let mean = pnls.iter().sum::<f64>() / n;
        let std_dev = 0.0;
        let t = if std_dev > 0.0 && n > 1.0 {
            mean / (std_dev / n.sqrt())
        } else {
            0.0
        };
        assert_eq!(t, 0.0);
    }

    #[test]
    fn test_p_value_positive_edge() {
        let pnls = vec![
            5.0, 8.0, 12.0, 6.0, 9.0, 7.0, 11.0, 10.0, 13.0, 4.0, 8.0, 9.0, 7.0, 11.0, 6.0, 10.0,
            12.0, 5.0, 8.0, 9.0, 7.0, 11.0, 6.0, 10.0, 13.0, 4.0, 8.0, 9.0, 12.0, 7.0,
        ];
        let n = pnls.len() as f64;
        let mean = pnls.iter().sum::<f64>() / n;
        let variance = pnls.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();
        let t = mean / (std_dev / n.sqrt());
        let p = one_tailed_t_pvalue(t, (n - 1.0) as u32);
        assert!(p < 0.001);
    }

    fn compute_strategy_analytics_from_trades(
        trades: &[TradeAnalyticsRecord],
    ) -> Vec<StrategyAnalyticsRow> {
        let mut by_setup: std::collections::HashMap<String, Vec<&TradeAnalyticsRecord>> =
            std::collections::HashMap::new();
        for t in trades {
            by_setup
                .entry(t.trigger_source.clone())
                .or_default()
                .push(t);
        }
        by_setup
            .into_iter()
            .map(|(setup_type, setup_trades)| compute_setup_analytics(&setup_type, &setup_trades, AnalyticsParams::default()))
            .collect()
    }
}
