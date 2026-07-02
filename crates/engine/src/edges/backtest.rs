use sqlx::SqlitePool;

use crate::edges::types::{
    BacktestOutput, EdgeConfig, EquityPoint, HistoricalMetrics, TradeLog,
};

/// Stateless historical row for the continuous confluence backtester. Queries
/// the pre-computed, indexed `_normalized` / `_state_label` columns produced by
/// the live normalization engine, guaranteeing backtest ↔ live parity. Only
/// `close` and `atr_14` remain raw (needed for entry/exit prices + stop/TP
/// distances).
#[derive(Debug, Clone, sqlx::FromRow)]
struct BacktestRow {
    #[allow(dead_code)]
    timestamp: i64,
    close: f64,
    atr_14: Option<f64>,
    rsi_normalized: Option<f64>,
    macd_normalized: Option<f64>,
    squeeze_normalized: Option<f64>,
    adx_normalized: Option<f64>,
    bbwp_normalized: Option<f64>,
    rvol_normalized: Option<f64>,
    ema_stack_normalized: Option<f64>,
    vwap_normalized: Option<f64>,
    squeeze_state_label: Option<String>,
    bbwp_state_label: Option<String>,
    ema_stack_state_label: Option<String>,
    rvol_state_label: Option<String>,
    adx_state_label: Option<String>,
}

/// Fetch the normalized `[-1.0, 1.0]` value for a config indicator name.
/// `atr` (and any unmapped indicator) has no normalized column → 0.0.
fn normalized_for(row: &BacktestRow, name: &str) -> f64 {
    match name {
        "rsi" => row.rsi_normalized,
        "macd" => row.macd_normalized,
        "squeeze" => row.squeeze_normalized,
        "adx" => row.adx_normalized,
        "bbwp" => row.bbwp_normalized,
        "rvol" => row.rvol_normalized,
        "ema" => row.ema_stack_normalized,
        "vwap" => row.vwap_normalized,
        _ => None,
    }
    .unwrap_or(0.0)
}

/// ADX congestion (TRENDLESS / ~0 normalized) dampens trend-following factors.
fn adx_congestion(row: &BacktestRow) -> bool {
    row.adx_state_label.as_deref() == Some("TRENDLESS_CONGESTION")
        || row.adx_normalized.is_none_or(|v| v.abs() < 0.05)
}

/// First-bar squeeze volatility release (breakout setup).
fn squeeze_release(row: &BacktestRow) -> bool {
    row.squeeze_state_label
        .as_deref()
        .is_some_and(|l| l.ends_with("VOLATILITY_RELEASE"))
}

/// Institutional-grade relative volume (rvol 1.5–3.0 band).
fn rvol_institutional(row: &BacktestRow) -> bool {
    matches!(
        row.rvol_state_label.as_deref(),
        Some("INSTITUTIONAL_BREAKOUT_VOLUME")
    )
}

/// BBWP volatility-exhaustion climax (bbwp > 90%) → mean-reversion drag.
fn bbwp_climax(row: &BacktestRow) -> bool {
    row.bbwp_state_label.as_deref() == Some("VOLATILITY_EXHAUSTION_REVERSION_WARNING")
}


fn classify_regime(row: &BacktestRow) -> String {
    let squeeze = row.squeeze_state_label.as_deref().unwrap_or("");
    let bbwp = row.bbwp_state_label.as_deref().unwrap_or("");
    let ema = row.ema_stack_state_label.as_deref().unwrap_or("");
    let adx_norm = row.adx_normalized.unwrap_or(0.0);
    let stacked = ema.contains("BULLISH") || ema.contains("BEARISH");

    if squeeze == "COMPRESSION_COILING" || bbwp == "MAX_VOLATILITY_COMPRESSION" {
        "compression".to_string()
    } else if squeeze.ends_with("VOLATILITY_RELEASE") || bbwp == "VOLATILITY_EXHAUSTION_REVERSION_WARNING" {
        "expansion".to_string()
    } else if adx_norm.abs() >= 0.5 && stacked {
        "trending".to_string()
    } else {
        "range".to_string()
    }
}

/// Continuous confluence score matching the live Layer-2 scoring model:
/// `Σ (normalized_i × config_weight_i)` with the same ADX / RVOL / BBWP gating
/// multipliers, projected onto `[-90, +90]`. Weights are sourced from the
/// active `EdgeConfig` (user-tunable in the Edge Builder), guaranteeing that a
/// backtested strategy's entries/exits mirror live execution on identical data.
fn compute_confluence_score(row: &BacktestRow, config: &EdgeConfig) -> f64 {
    let congested = adx_congestion(row);
    let breakout_active = squeeze_release(row);
    let rvol_confirmed = rvol_institutional(row);

    let mut score = 0.0_f64;
    let mut ema_weight = 0.0_f64;
    let mut ema_norm = 0.0_f64;

    for ind in &config.indicators {
        if !ind.enabled {
            continue;
        }
        let name = ind.name.to_lowercase();
        let norm = normalized_for(row, &name);
        let mut contrib = norm * ind.weight;

        match name.as_str() {
            // ADX congestion gate: dampen trend-following (EMA stack) to zero.
            "ema" => {
                ema_weight = ind.weight;
                ema_norm = norm;
                if congested {
                    contrib = 0.0;
                }
            }
            // RVOL breakout gate: unconfirmed breakout bars are dampened ×0.3.
            "squeeze" | "macd" if breakout_active && !rvol_confirmed => {
                contrib *= 0.3;
            }
            _ => {}
        }
        score += contrib;
    }

    // BBWP volatility-climax drag against the prevailing trend bias.
    if bbwp_climax(row) && ema_weight > 0.0 {
        score += -0.1 * ema_norm.signum() * ema_weight;
    }

    score.clamp(-90.0, 90.0)
}

fn regime_allowed(regime: &str, config: &EdgeConfig) -> bool {
    match regime {
        "trending" => config.regime_gates.trending,
        "compression" => config.regime_gates.compression,
        "expansion" => config.regime_gates.expansion,
        "range" => config.regime_gates.range,
        _ => true,
    }
}

fn check_execution_gates(row: &BacktestRow, config: &EdgeConfig) -> bool {
    // Raw RVOL is never persisted; gate against the normalized RVOL bands
    // (CONSOLIDATION < 1.0, NORMAL 1.0–1.5, INSTITUTIONAL 1.5–3.0, CLIMAX ≥ 3.0).
    let label = row.rvol_state_label.as_deref().unwrap_or("");

    // Volume-confirmation gate.
    if config.execution.min_rvol >= 1.5 {
        // Require institutional-grade participation.
        if label != "INSTITUTIONAL_BREAKOUT_VOLUME" {
            return false;
        }
    } else if config.execution.min_rvol >= 1.0 && label == "CONSOLIDATION_VOLUME" {
        return false;
    }

    // Exhaustion-climax block.
    if config.execution.climax_rvol > 0.0 && label == "EXHAUSTION_CLIMAX_VOLUME" {
        return false;
    }

    true
}

fn check_mtf_quorum(
    _row: &BacktestRow,
    _config: &EdgeConfig,
) -> bool {
    true
}

fn clamp_config(config: &EdgeConfig) -> EdgeConfig {
    let mut cfg = config.clone();
    cfg.sizing.daily_vol_target_pct = cfg.sizing.daily_vol_target_pct.clamp(0.1, 5.0);
    cfg.sizing.max_leverage = cfg.sizing.max_leverage.clamp(1.0, 20.0);
    cfg.stop_loss.atr_multiplier = cfg.stop_loss.atr_multiplier.clamp(0.5, 5.0);
    cfg.backtest_depth = cfg.backtest_depth.clamp(100, 50000);
    cfg
}

fn compute_position_size(config: &EdgeConfig, _atr: f64, _price: f64) -> f64 {
    match config.sizing.model {
        crate::edges::types::SizingModel::Fixed => 0.25,
        crate::edges::types::SizingModel::VolatilityTargeting => {
            let vol_pct = config.sizing.daily_vol_target_pct / 100.0;
            let lev = config.sizing.max_leverage;
            (vol_pct * 100.0 / 2.0).min(1.0 / lev).max(0.01)
        }
    }
}

fn compute_stop_loss(
    config: &EdgeConfig,
    entry_price: f64,
    direction: &str,
    atr: f64,
) -> f64 {
    let distance = atr * config.stop_loss.atr_multiplier;
    if direction == "LONG" {
        entry_price - distance
    } else {
        entry_price + distance
    }
}

fn compute_take_profits(
    config: &EdgeConfig,
    entry_price: f64,
    direction: &str,
    atr: f64,
) -> Vec<f64> {
    let multipliers = [
        config.take_profit.tp1_multiplier,
        config.take_profit.tp2_multiplier,
        config.take_profit.tp3_multiplier,
    ];
    multipliers
        .iter()
        .filter(|&&m| m > 0.0)
        .map(|&m| {
            let distance = atr * m;
            if direction == "LONG" {
                entry_price + distance
            } else {
                entry_price - distance
            }
        })
        .collect()
}

fn compute_metrics(trade_returns: &[f64], equity_curve: &[EquityPoint]) -> HistoricalMetrics {
    let total_trades = trade_returns.len();
    if total_trades == 0 {
        return HistoricalMetrics {
            total_trades: 0,
            win_rate: 0.0,
            profit_factor: 0.0,
            net_sharpe_ratio: 0.0,
            max_drawdown_pct: 0.0,
            max_drawdown_duration: 0,
            total_return_pct: 0.0,
            avg_trade_return_pct: 0.0,
            avg_win_pct: 0.0,
            avg_loss_pct: 0.0,
        };
    }

    let wins: Vec<f64> = trade_returns.iter().filter(|&&r| r > 0.0).copied().collect();
    let losses: Vec<f64> = trade_returns.iter().filter(|&&r| r <= 0.0).copied().collect();

    let win_rate = wins.len() as f64 / total_trades as f64;

    let gross_profit: f64 = wins.iter().sum();
    let gross_loss: f64 = losses.iter().map(|l| l.abs()).sum();
    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else if gross_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    let avg_return: f64 = trade_returns.iter().sum::<f64>() / total_trades as f64;
    let variance: f64 = trade_returns
        .iter()
        .map(|r| (r - avg_return).powi(2))
        .sum::<f64>()
        / total_trades as f64;
    let std_dev = variance.sqrt();
    let sharpe = if std_dev > 0.0 {
        avg_return / std_dev * (252.0_f64).sqrt()
    } else {
        0.0
    };

    let mut peak = 0.0_f64;
    let mut max_dd = 0.0_f64;
    let mut max_dd_duration = 0;
    let mut current_dd_duration = 0;

    for pt in equity_curve {
        if pt.cumulative_return_pct > peak {
            peak = pt.cumulative_return_pct;
            current_dd_duration = 0;
        } else {
            current_dd_duration += 1;
            let dd = peak - pt.cumulative_return_pct;
            if dd > max_dd {
                max_dd = dd;
            }
            if current_dd_duration > max_dd_duration {
                max_dd_duration = current_dd_duration;
            }
        }
    }

    let total_return = equity_curve.last().map(|p| p.cumulative_return_pct).unwrap_or(0.0);
    let avg_win = if wins.is_empty() { 0.0 } else { wins.iter().sum::<f64>() / wins.len() as f64 };
    let avg_loss = if losses.is_empty() { 0.0 } else { losses.iter().sum::<f64>() / losses.len() as f64 };

    HistoricalMetrics {
        total_trades,
        win_rate,
        profit_factor,
        net_sharpe_ratio: sharpe,
        max_drawdown_pct: max_dd,
        max_drawdown_duration: max_dd_duration,
        total_return_pct: total_return,
        avg_trade_return_pct: avg_return,
        avg_win_pct: avg_win,
        avg_loss_pct: avg_loss,
    }
}

pub async fn run_backtest(
    pool: &SqlitePool,
    config: &EdgeConfig,
    symbol: &str,
    timeframe_secs: u64,
) -> Result<BacktestOutput, String> {
    let config = clamp_config(config);

    let rows: Vec<BacktestRow> = sqlx::query_as(
        "SELECT timestamp,
                CAST(close AS REAL) as close,
                CAST(atr_14 AS REAL) as atr_14,
                rsi_normalized,
                macd_normalized,
                squeeze_normalized,
                adx_normalized,
                bbwp_normalized,
                rvol_normalized,
                ema_stack_normalized,
                vwap_normalized,
                squeeze_state_label,
                bbwp_state_label,
                ema_stack_state_label,
                rvol_state_label,
                adx_state_label
         FROM market_snapshots
         WHERE symbol = ?1
           AND timeframe_secs = ?2
           AND close IS NOT NULL
         ORDER BY timestamp ASC
         LIMIT ?3",
    )
    .bind(symbol)
    .bind(timeframe_secs as i64)
    .bind(config.backtest_depth as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to query snapshots: {}", e))?;

    if rows.is_empty() {
        return Err("No historical data available for the selected symbol and timeframe".to_string());
    }

    let mut trade_logs: Vec<TradeLog> = Vec::new();
    let mut trade_returns: Vec<f64> = Vec::new();
    let mut equity_curve: Vec<EquityPoint> = vec![EquityPoint {
        trade_index: 0,
        cumulative_return_pct: 0.0,
        regime: String::new(),
    }];
    let mut cumulative_return = 0.0_f64;

    let mut in_position = false;
    let mut current_direction = String::new();
    let mut entry_price = 0.0_f64;
    let mut entry_index = 0_usize;
    let mut stop_loss = 0.0_f64;
    let mut take_profits: Vec<f64> = Vec::new();
    let mut position_regime = String::new();
    let mut position_size = 0.0_f64;
    let _tp_fill_count = 0_usize;

    let split_idx = (rows.len() as f64 * 0.7) as usize;

    for (i, row) in rows.iter().enumerate() {
        if !in_position {
            let regime = classify_regime(row);
            if !regime_allowed(&regime, &config) {
                continue;
            }
            if !check_execution_gates(row, &config) {
                continue;
            }
            if !check_mtf_quorum(row, &config) {
                continue;
            }

            let score = compute_confluence_score(row, &config);
            let abs_score = score.abs();

            if abs_score < config.quorum_threshold {
                continue;
            }

            let direction = if score > 0.0 { "LONG" } else { "SHORT" };

            entry_price = row.close;
            entry_index = i;
            current_direction = direction.to_string();
            position_regime = regime;

            let atr = row.atr_14.unwrap_or(row.close * 0.01);
            stop_loss = compute_stop_loss(&config, entry_price, direction, atr);
            take_profits = compute_take_profits(&config, entry_price, direction, atr);
            position_size = compute_position_size(&config, atr, entry_price);

            in_position = true;
        } else {
            let mut exit_reason = String::new();
            let mut exit = false;

            if current_direction == "LONG" {
                if row.close <= stop_loss {
                    exit_reason = "stop_loss".to_string();
                    exit = true;
                } else {
                    for &tp in &take_profits {
                        if row.close >= tp {
                            exit_reason = format!("take_profit_{}", tp);
                            exit = true;
                            break;
                        }
                    }
                }
            } else {
                if row.close >= stop_loss {
                    exit_reason = "stop_loss".to_string();
                    exit = true;
                } else {
                    for &tp in &take_profits {
                        if row.close <= tp {
                            exit_reason = format!("take_profit_{}", tp);
                            exit = true;
                            break;
                        }
                    }
                }
            }

            if i >= rows.len() - 1 && !exit {
                exit = true;
                exit_reason = "end_of_data".to_string();
            }

            if exit {
                let pnl_pct = if current_direction == "LONG" {
                    ((row.close - entry_price) / entry_price) * 100.0 * position_size
                } else {
                    ((entry_price - row.close) / entry_price) * 100.0 * position_size
                };

                let fee_pct = 0.06 * 2.0 * position_size;
                let net_pnl_pct = pnl_pct - fee_pct;

                let pnl_absolute = net_pnl_pct;

                cumulative_return += net_pnl_pct;

                trade_logs.push(TradeLog {
                    entry_index,
                    exit_index: i,
                    direction: current_direction.clone(),
                    entry_price,
                    exit_price: row.close,
                    pnl_pct: net_pnl_pct,
                    pnl_absolute,
                    exit_reason: exit_reason.clone(),
                    regime_at_entry: position_regime.clone(),
                });

                trade_returns.push(net_pnl_pct);

                equity_curve.push(EquityPoint {
                    trade_index: trade_logs.len(),
                    cumulative_return_pct: cumulative_return,
                    regime: position_regime.clone(),
                });

                in_position = false;
            }
        }
    }

    let metrics = compute_metrics(&trade_returns, &equity_curve);

    let in_sample_equity: Vec<EquityPoint> = equity_curve
        .iter()
        .filter(|pt| {
            if pt.trade_index == 0 {
                return true;
            }
            if let Some(trade) = trade_logs.get(pt.trade_index - 1) {
                trade.exit_index < split_idx
            } else {
                false
            }
        })
        .cloned()
        .collect();

    let out_of_sample_equity: Vec<EquityPoint> = equity_curve
        .iter()
        .filter(|pt| {
            if pt.trade_index == 0 {
                return true;
            }
            if let Some(trade) = trade_logs.get(pt.trade_index - 1) {
                trade.exit_index >= split_idx
            } else {
                false
            }
        })
        .cloned()
        .collect();

    Ok(BacktestOutput {
        trade_logs,
        equity_curve: equity_curve.clone(),
        in_sample_equity: in_sample_equity.clone(),
        out_of_sample_equity: out_of_sample_equity.clone(),
        metrics,
        trade_returns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edges::types::{IndicatorConfig, RegimeGates, TriggerRule};

    /// Build a neutral row (all indicators at equilibrium / unknown).
    fn row() -> BacktestRow {
        BacktestRow {
            timestamp: 1,
            close: 100.0,
            atr_14: Some(1.0),
            rsi_normalized: None,
            macd_normalized: None,
            squeeze_normalized: None,
            adx_normalized: None,
            bbwp_normalized: None,
            rvol_normalized: None,
            ema_stack_normalized: None,
            vwap_normalized: None,
            squeeze_state_label: None,
            bbwp_state_label: None,
            ema_stack_state_label: None,
            rvol_state_label: None,
            adx_state_label: None,
        }
    }

    #[test]
    fn test_classify_regime_trending() {
        let r = BacktestRow {
            adx_normalized: Some(0.7),
            ema_stack_state_label: Some("ESTABLISHED_BULLISH_STACK".to_string()),
            ..row()
        };
        assert_eq!(classify_regime(&r), "trending");
    }

    #[test]
    fn test_classify_regime_compression() {
        let r = BacktestRow {
            squeeze_state_label: Some("COMPRESSION_COILING".to_string()),
            ..row()
        };
        assert_eq!(classify_regime(&r), "compression");
    }

    #[test]
    fn test_classify_regime_expansion() {
        let r = BacktestRow {
            squeeze_state_label: Some("BULLISH_VOLATILITY_RELEASE".to_string()),
            ..row()
        };
        assert_eq!(classify_regime(&r), "expansion");
    }

    #[test]
    fn test_compute_metrics_empty() {
        let metrics = compute_metrics(&[], &[]);
        assert_eq!(metrics.total_trades, 0);
        assert_eq!(metrics.win_rate, 0.0);
    }

    #[test]
    fn test_compute_metrics_basic() {
        let returns = vec![2.0, -1.0, 3.0, -0.5, 1.0];
        let eq = |i: usize, c: f64| EquityPoint {
            trade_index: i,
            cumulative_return_pct: c,
            regime: String::new(),
        };
        let equity = vec![
            eq(0, 0.0),
            eq(1, 2.0),
            eq(2, 1.0),
            eq(3, 4.0),
            eq(4, 3.5),
            eq(5, 4.5),
        ];
        let metrics = compute_metrics(&returns, &equity);
        assert_eq!(metrics.total_trades, 5);
        assert!(metrics.win_rate > 0.0);
        assert!(metrics.profit_factor > 0.0);
    }

    #[test]
    fn test_clamp_config_bounds() {
        let mut config = EdgeConfig::default();
        config.sizing.max_leverage = 100.0;
        config.stop_loss.atr_multiplier = 0.1;
        config.backtest_depth = 10;
        let clamped = clamp_config(&config);
        assert_eq!(clamped.sizing.max_leverage, 20.0);
        assert_eq!(clamped.stop_loss.atr_multiplier, 0.5);
        assert_eq!(clamped.backtest_depth, 100);
    }

    fn ind(name: &str, weight: f64) -> IndicatorConfig {
        IndicatorConfig {
            name: name.to_string(),
            weight,
            trigger_rule: TriggerRule::OverboughtOversold,
            enabled: true,
        }
    }

    #[test]
    fn test_continuous_confluence_weighted_sum() {
        // Bullish alignment: rsi +0.8, macd +0.9, ema_stack +1.0.
        let r = BacktestRow {
            rsi_normalized: Some(0.8),
            macd_normalized: Some(0.9),
            ema_stack_normalized: Some(1.0),
            ema_stack_state_label: Some("ESTABLISHED_BULLISH_STACK".to_string()),
            adx_normalized: Some(0.7),
            adx_state_label: Some("STRONG_BULL_TREND".to_string()),
            ..row()
        };
        let config = EdgeConfig {
            indicators: vec![ind("rsi", 10.0), ind("macd", 10.0), ind("ema", 20.0)],
            ..Default::default()
        };
        // 0.8*10 + 0.9*10 + 1.0*20 = 37.0
        let score = compute_confluence_score(&r, &config);
        assert!((score - 37.0).abs() < 1e-6, "got {}", score);
        assert!(score > 0.0);
    }

    #[test]
    fn test_adx_congestion_gate_zeroes_trend() {
        let r = BacktestRow {
            ema_stack_normalized: Some(1.0),
            ema_stack_state_label: Some("ESTABLISHED_BULLISH_STACK".to_string()),
            adx_normalized: Some(0.0),
            adx_state_label: Some("TRENDLESS_CONGESTION".to_string()),
            ..row()
        };
        let config = EdgeConfig {
            indicators: vec![ind("ema", 20.0)],
            ..Default::default()
        };
        // Trend contribution gated to 0 under congestion.
        assert_eq!(compute_confluence_score(&r, &config), 0.0);
    }

    #[test]
    fn test_rvol_gate_dampens_unconfirmed_breakout() {
        let base = BacktestRow {
            squeeze_normalized: Some(1.0),
            squeeze_state_label: Some("BULLISH_VOLATILITY_RELEASE".to_string()),
            ..row()
        };
        let config = EdgeConfig {
            indicators: vec![ind("squeeze", 10.0)],
            ..Default::default()
        };
        // Unconfirmed volume → ×0.3 dampening.
        let unconfirmed = BacktestRow {
            rvol_state_label: Some("CONSOLIDATION_VOLUME".to_string()),
            ..base.clone()
        };
        assert!((compute_confluence_score(&unconfirmed, &config) - 3.0).abs() < 1e-6);
        // Institutional volume → full weight.
        let confirmed = BacktestRow {
            rvol_state_label: Some("INSTITUTIONAL_BREAKOUT_VOLUME".to_string()),
            ..base
        };
        assert!((compute_confluence_score(&confirmed, &config) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_confluence_clamps_to_90() {
        let r = BacktestRow {
            rsi_normalized: Some(1.0),
            macd_normalized: Some(1.0),
            ..row()
        };
        let config = EdgeConfig {
            indicators: vec![ind("rsi", 60.0), ind("macd", 60.0)],
            ..Default::default()
        };
        assert_eq!(compute_confluence_score(&r, &config), 90.0);
    }

    #[test]
    fn test_regime_allowed() {
        let config = EdgeConfig {
            regime_gates: RegimeGates {
                trending: true,
                compression: false,
                expansion: false,
                range: false,
            },
            ..Default::default()
        };
        assert!(regime_allowed("trending", &config));
        assert!(!regime_allowed("compression", &config));
    }
}
