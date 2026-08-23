use core_domain::performance::RiskAnalyticsRow;
use sqlx::SqlitePool;

const TRADING_DAYS_PER_YEAR: f64 = 365.0;

/// v10: pure risk-metrics computation over an arbitrary equity curve
/// `(ts_ms, value)` — shared by the live PAE path and the BTE per-run
/// metrics enrichment.
pub fn compute_risk_metrics_from_curve(equity: &[(i64, f64)]) -> RiskAnalyticsRow {
    if equity.len() < 2 {
        return RiskAnalyticsRow {
            maximum_drawdown_pct: 0.0,
            max_drawdown_duration_days: 0.0,
            average_drawdown_pct: 0.0,
            drawdown_count: 0,
            sharpe_ratio: None,
            sortino_ratio: None,
            ulcer_index: 0.0,
            calmar_ratio: None,
            daily_volatility: 0.0,
            downside_deviation: 0.0,
            value_at_risk_95: 0.0,
            expected_shortfall_95: 0.0,
        };
    }

    let values: Vec<f64> = equity.iter().map(|(_, v)| *v).collect();

    let (max_dd_pct, max_dd_days, avg_dd_pct, dd_count) = compute_drawdowns(&values, equity);

    let daily_returns = compute_daily_returns(equity);
    let mean_return = if !daily_returns.is_empty() {
        daily_returns.iter().sum::<f64>() / daily_returns.len() as f64
    } else {
        0.0
    };

    let daily_vol = std_dev(&daily_returns);
    let downside_returns: Vec<f64> = daily_returns
        .iter()
        .filter(|&&r| r < 0.0)
        .copied()
        .collect();
    let downside_dev = std_dev(&downside_returns);

    let sharpe = if daily_vol > 0.0 {
        Some((mean_return / daily_vol) * (TRADING_DAYS_PER_YEAR.sqrt()))
    } else {
        None
    };

    let sortino = if downside_dev > 0.0 {
        Some((mean_return / downside_dev) * (TRADING_DAYS_PER_YEAR.sqrt()))
    } else {
        None
    };

    let annualized_return = mean_return * TRADING_DAYS_PER_YEAR;
    let calmar = if max_dd_pct > 0.0 {
        Some(annualized_return / (max_dd_pct / 100.0))
    } else {
        None
    };

    let ulcer = compute_ulcer_index(&values);
    let (var_95, es_95) = compute_var_es(&daily_returns);

    RiskAnalyticsRow {
        maximum_drawdown_pct: max_dd_pct,
        max_drawdown_duration_days: max_dd_days,
        average_drawdown_pct: avg_dd_pct,
        drawdown_count: dd_count,
        sharpe_ratio: sharpe,
        sortino_ratio: sortino,
        ulcer_index: ulcer,
        calmar_ratio: calmar,
        daily_volatility: daily_vol,
        downside_deviation: downside_dev,
        value_at_risk_95: var_95,
        expected_shortfall_95: es_95,
    }
}

/// Compute risk-adjusted performance metrics from the equity history.
/// Implements docs:03-05-04-pae-layer3-risk-analytics.md
pub async fn compute_risk_analytics(pool: &SqlitePool) -> RiskAnalyticsRow {
    let equity = portfolio_supervisor::portfolio_equity::fetch_equity_history(pool, None, None).await;
    compute_risk_metrics_from_curve(&equity)
}

fn compute_drawdowns(values: &[f64], equity: &[(i64, f64)]) -> (f64, f64, f64, u32) {
    let mut peak = values[0];
    let mut max_dd_pct = 0.0;
    let mut max_dd_duration_ms: i64 = 0;
    let mut dd_events: Vec<f64> = vec![];
    let mut in_drawdown = false;
    let mut dd_start_ts: i64 = 0;
    let mut current_dd_pct: f64 = 0.0;

    for i in 0..values.len() {
        let v = values[i];
        let ts = equity[i].0;

        if v > peak {
            if in_drawdown {
                let duration_ms = ts - dd_start_ts;
                if duration_ms > max_dd_duration_ms {
                    max_dd_duration_ms = duration_ms;
                }
                dd_events.push(current_dd_pct);
                in_drawdown = false;
            }
            peak = v;
        }

        let dd_pct = (peak - v) / peak * 100.0;
        if dd_pct > max_dd_pct {
            max_dd_pct = dd_pct;
        }

        if dd_pct > 0.0 && !in_drawdown {
            in_drawdown = true;
            dd_start_ts = ts;
            current_dd_pct = dd_pct;
        } else if dd_pct > 0.0 && in_drawdown {
            current_dd_pct = current_dd_pct.max(dd_pct);
        }
    }

    if in_drawdown {
        let last_ts = equity[equity.len() - 1].0;
        let duration_ms = last_ts - dd_start_ts;
        if duration_ms > max_dd_duration_ms {
            max_dd_duration_ms = duration_ms;
        }
        dd_events.push(current_dd_pct);
    }

    let dd_count = dd_events.len() as u32;
    let avg_dd_pct = if !dd_events.is_empty() {
        dd_events.iter().sum::<f64>() / dd_events.len() as f64
    } else {
        0.0
    };

    let max_dd_days = max_dd_duration_ms as f64 / (1000.0 * 60.0 * 60.0 * 24.0);

    (max_dd_pct, max_dd_days, avg_dd_pct, dd_count)
}

fn compute_daily_returns(equity: &[(i64, f64)]) -> Vec<f64> {
    if equity.len() < 2 {
        return vec![];
    }

    let mut returns = Vec::new();
    let day_ms: i64 = 24 * 60 * 60 * 1000;
    let mut i = 0;

    while i < equity.len() {
        let current_day_start = (equity[i].0 / day_ms) * day_ms;
        let mut day_end = i;
        while day_end + 1 < equity.len() && equity[day_end + 1].0 < current_day_start + day_ms {
            day_end += 1;
        }

        if day_end > i && equity[i].1 > 0.0 {
            let r = (equity[day_end].1 - equity[i].1) / equity[i].1;
            returns.push(r);
        }

        i = day_end + 1;
    }

    returns
}

fn std_dev(data: &[f64]) -> f64 {
    if data.is_empty() || data.len() == 1 {
        return 0.0;
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (data.len() - 1) as f64;
    variance.sqrt()
}

fn compute_ulcer_index(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut peak = values[0];
    let mut sum_sq = 0.0;

    for &v in values {
        if v > peak {
            peak = v;
        }
        let dd_pct = (peak - v) / peak * 100.0;
        sum_sq += dd_pct * dd_pct;
    }

    (sum_sq / values.len() as f64).sqrt()
}

fn compute_var_es(returns: &[f64]) -> (f64, f64) {
    if returns.len() < 2 {
        return (0.0, 0.0);
    }

    let mut sorted = returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let var_idx = (0.05 * sorted.len() as f64).ceil() as usize;
    let var_idx = var_idx.min(sorted.len().saturating_sub(1));

    let var_95 = sorted[var_idx];

    let es_slice: Vec<f64> = sorted.iter().take(var_idx + 1).copied().collect();
    let es_95 = if !es_slice.is_empty() {
        es_slice.iter().sum::<f64>() / es_slice.len() as f64
    } else {
        var_95
    };

    (var_95, es_95)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawdown_single_dive() {
        let values = vec![100.0, 90.0, 80.0, 95.0, 110.0];
        let equity: Vec<(i64, f64)> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as i64 * 86_400_000, v))
            .collect();
        let (max_dd, dd_days, _avg_dd, _count) = compute_drawdowns(&values, &equity);
        assert!((max_dd - 20.0).abs() < 0.1);
        assert!(dd_days > 0.0);
    }

    #[test]
    fn test_drawdown_duration_recovery() {
        let values = vec![100.0, 90.0, 85.0, 88.0, 92.0, 100.0, 110.0];
        let equity: Vec<(i64, f64)> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as i64 * 86_400_000, v))
            .collect();
        let (_max_dd, dd_days, _avg_dd, _count) = compute_drawdowns(&values, &equity);
        assert!((dd_days - 5.0).abs() < 0.5);
    }

    #[test]
    fn test_drawdown_unrecovered() {
        let values = vec![100.0, 90.0, 85.0, 80.0];
        let equity: Vec<(i64, f64)> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as i64 * 86_400_000, v))
            .collect();
        let (_max_dd, dd_days, _avg_dd, _count) = compute_drawdowns(&values, &equity);
        assert!(dd_days > 0.0);
    }

    #[test]
    fn test_drawdown_no_loss() {
        let values = vec![100.0, 110.0, 120.0, 130.0];
        let equity: Vec<(i64, f64)> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as i64 * 86_400_000, v))
            .collect();
        let (max_dd, _, _, _) = compute_drawdowns(&values, &equity);
        assert_eq!(max_dd, 0.0);
    }

    #[test]
    fn test_std_dev_zero_for_single_value() {
        assert_eq!(std_dev(&[5.0]), 0.0);
    }

    #[test]
    fn test_std_dev_positive() {
        let v = std_dev(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(v > 0.0);
    }

    #[test]
    fn test_ulcer_all_time_high() {
        let values = vec![100.0, 110.0, 120.0, 130.0, 140.0];
        let ulcer = compute_ulcer_index(&values);
        assert_eq!(ulcer, 0.0);
    }

    #[test]
    fn test_ulcer_with_drawdown() {
        let values = vec![100.0, 90.0, 85.0, 95.0, 110.0];
        let ulcer = compute_ulcer_index(&values);
        assert!(ulcer > 0.0);
    }

    #[test]
    fn test_var_es_sorted() {
        let returns = vec![-0.05, -0.03, -0.01, 0.0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06];
        let (var, es) = compute_var_es(&returns);
        assert!(var <= -0.03);
        assert!(es <= var);
    }

    #[test]
    fn test_daily_returns_empty() {
        let equity: Vec<(i64, f64)> = vec![];
        assert!(compute_daily_returns(&equity).is_empty());
    }

    #[test]
    fn test_daily_returns_single() {
        let equity: Vec<(i64, f64)> = vec![(0, 100.0)];
        assert!(compute_daily_returns(&equity).is_empty());
    }
}
