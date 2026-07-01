use sqlx::SqlitePool;

use crate::edges::types::{
    BacktestOutput, EdgeConfig, EquityPoint, HistoricalMetrics, TradeLog,
    TriggerRule,
};

#[derive(Debug, Clone, sqlx::FromRow)]
struct BacktestRow {
    #[allow(dead_code)]
    timestamp: i64,
    close: f64,
    rsi_14: Option<f64>,
    macd_line: Option<f64>,
    macd_signal: Option<f64>,
    macd_crossover_detected: Option<bool>,
    macd_crossover_direction: Option<String>,
    rsi_divergence_status: Option<String>,
    macd_divergence_status: Option<String>,
    adx_14: Option<f64>,
    #[allow(dead_code)]
    adx_plus: Option<f64>,
    #[allow(dead_code)]
    adx_minus: Option<f64>,
    #[allow(dead_code)]
    adx_regime: Option<String>,
    adx_slope: Option<f64>,
    bbwp: Option<f64>,
    squeeze_on: Option<bool>,
    squeeze_momentum: Option<f64>,
    squeeze_release_trigger: Option<bool>,
    atr_14: Option<f64>,
    ema_fast: Option<f64>,
    ema_medium: Option<f64>,
    #[allow(dead_code)]
    ema_slow: Option<f64>,
    #[allow(dead_code)]
    ema_long: Option<f64>,
    ema_stack_state: Option<String>,
    vwap: Option<f64>,
    rvol: Option<f64>,
    #[allow(dead_code)]
    average_volume: Option<f64>,
    #[allow(dead_code)]
    volume: Option<f64>,
}


fn classify_regime(row: &BacktestRow) -> String {
    let adx = row.adx_14.unwrap_or(0.0);
    let bbwp = row.bbwp.unwrap_or(50.0);
    let squeeze_on = row.squeeze_on.unwrap_or(false);
    let stack = row.ema_stack_state.as_deref().unwrap_or("tangled");

    if bbwp < 10.0 && squeeze_on {
        "compression".to_string()
    } else if bbwp > 90.0 {
        "expansion".to_string()
    } else if adx > 25.0 && (stack == "bullish" || stack == "bearish") {
        "trending".to_string()
    } else {
        "range".to_string()
    }
}

fn evaluate_indicator_trigger(row: &BacktestRow, indicator: &crate::edges::types::IndicatorConfig) -> f64 {
    if !indicator.enabled {
        return 0.0;
    }

    let name = indicator.name.to_lowercase();
    let rule = &indicator.trigger_rule;

    match (name.as_str(), rule) {
        ("rsi", TriggerRule::OverboughtOversold) => {
            if let Some(rsi) = row.rsi_14 {
                if rsi <= 30.0 { return indicator.weight; }
                if rsi >= 70.0 { return -indicator.weight; }
            }
            0.0
        }
        ("rsi", TriggerRule::Divergence) => {
            match row.rsi_divergence_status.as_deref() {
                Some("bullish") | Some("potential_bullish") => indicator.weight,
                Some("bearish") | Some("potential_bearish") => -indicator.weight,
                _ => 0.0,
            }
        }
        ("rsi", TriggerRule::ThresholdAbove) => {
            if let Some(rsi) = row.rsi_14 {
                if rsi >= 50.0 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("rsi", TriggerRule::ThresholdBelow) => {
            if let Some(rsi) = row.rsi_14 {
                if rsi <= 50.0 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("macd", TriggerRule::Crossover) => {
            if row.macd_crossover_detected.unwrap_or(false) {
                match row.macd_crossover_direction.as_deref() {
                    Some("BULLISH") => indicator.weight,
                    Some("BEARISH") => -indicator.weight,
                    _ => 0.0,
                }
            } else {
                0.0
            }
        }
        ("macd", TriggerRule::Divergence) => {
            match row.macd_divergence_status.as_deref() {
                Some("bullish") | Some("potential_bullish") => indicator.weight,
                Some("bearish") | Some("potential_bearish") => -indicator.weight,
                _ => 0.0,
            }
        }
        ("macd", TriggerRule::SlopeDirection) => {
            if let (Some(line), Some(signal)) = (row.macd_line, row.macd_signal) {
                if line > signal { indicator.weight } else if line < signal { -indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("adx", TriggerRule::SlopeDirection) => {
            if let Some(slope) = row.adx_slope {
                if slope > 0.0 { indicator.weight } else { -indicator.weight }
            } else {
                0.0
            }
        }
        ("adx", TriggerRule::ThresholdAbove) => {
            if let Some(adx) = row.adx_14 {
                if adx >= 25.0 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("adx", TriggerRule::ThresholdBelow) => {
            if let Some(adx) = row.adx_14 {
                if adx <= 20.0 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("bbwp", TriggerRule::ThresholdBelow) => {
            if let Some(bbwp) = row.bbwp {
                if bbwp <= 10.0 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("bbwp", TriggerRule::ThresholdAbove) => {
            if let Some(bbwp) = row.bbwp {
                if bbwp >= 90.0 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("squeeze", TriggerRule::Release) => {
            if row.squeeze_release_trigger.unwrap_or(false) {
                if let Some(mom) = row.squeeze_momentum {
                    if mom > 0.0 { indicator.weight } else { -indicator.weight }
                } else {
                    0.0
                }
            } else {
                0.0
            }
        }
        ("atr", TriggerRule::ThresholdAbove) => {
            0.0
        }
        ("atr", TriggerRule::SlopeDirection) => {
            0.0
        }
        ("vwap", TriggerRule::ThresholdAbove) => {
            if let (Some(vwap), Some(close)) = (row.vwap, Some(row.close)) {
                if close > vwap { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("vwap", TriggerRule::ThresholdBelow) => {
            if let (Some(vwap), Some(close)) = (row.vwap, Some(row.close)) {
                if close < vwap { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("rvol", TriggerRule::ThresholdAbove) => {
            if let Some(rvol) = row.rvol {
                if rvol >= 1.5 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("rvol", TriggerRule::ThresholdBelow) => {
            if let Some(rvol) = row.rvol {
                if rvol <= 0.5 { indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("ema", TriggerRule::Crossover) => {
            if let (Some(fast), Some(medium)) = (row.ema_fast, row.ema_medium) {
                if fast > medium { indicator.weight } else if fast < medium { -indicator.weight } else { 0.0 }
            } else {
                0.0
            }
        }
        ("ema", TriggerRule::SlopeDirection) => {
            match row.ema_stack_state.as_deref() {
                Some("bullish") => indicator.weight,
                Some("bearish") => -indicator.weight,
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}

fn compute_confluence_score(row: &BacktestRow, config: &EdgeConfig) -> f64 {
    config
        .indicators
        .iter()
        .map(|ind| evaluate_indicator_trigger(row, ind))
        .sum()
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
    if config.execution.min_rvol > 0.0 {
        if let Some(rvol) = row.rvol {
            if rvol < config.execution.min_rvol {
                return false;
            }
        }
    }
    if config.execution.climax_rvol > 0.0 {
        if let Some(rvol) = row.rvol {
            if rvol >= config.execution.climax_rvol {
                return false;
            }
        }
    }
    if config.execution.vwap_filter {
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
    if cfg.sizing.daily_vol_target_pct < 0.1 { cfg.sizing.daily_vol_target_pct = 0.1; }
    if cfg.sizing.daily_vol_target_pct > 5.0 { cfg.sizing.daily_vol_target_pct = 5.0; }
    if cfg.sizing.max_leverage < 1.0 { cfg.sizing.max_leverage = 1.0; }
    if cfg.sizing.max_leverage > 20.0 { cfg.sizing.max_leverage = 20.0; }
    if cfg.stop_loss.atr_multiplier < 0.5 { cfg.stop_loss.atr_multiplier = 0.5; }
    if cfg.stop_loss.atr_multiplier > 5.0 { cfg.stop_loss.atr_multiplier = 5.0; }
    if cfg.backtest_depth < 100 { cfg.backtest_depth = 100; }
    if cfg.backtest_depth > 50000 { cfg.backtest_depth = 50000; }
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
                CAST(rsi_14 AS REAL) as rsi_14,
                CAST(macd_line AS REAL) as macd_line,
                CAST(macd_signal AS REAL) as macd_signal,
                macd_crossover_detected,
                macd_crossover_direction,
                rsi_divergence_status,
                macd_divergence_status,
                CAST(adx_14 AS REAL) as adx_14,
                CAST(adx_plus AS REAL) as adx_plus,
                CAST(adx_minus AS REAL) as adx_minus,
                adx_regime,
                CAST(adx_slope AS REAL) as adx_slope,
                CAST(bbwp AS REAL) as bbwp,
                squeeze_on,
                CAST(squeeze_momentum AS REAL) as squeeze_momentum,
                squeeze_release_trigger,
                CAST(atr_14 AS REAL) as atr_14,
                CAST(ema_fast AS REAL) as ema_fast,
                CAST(ema_medium AS REAL) as ema_medium,
                CAST(ema_slow AS REAL) as ema_slow,
                CAST(ema_long AS REAL) as ema_long,
                ema_stack_state,
                CAST(vwap AS REAL) as vwap,
                CAST(rvol AS REAL) as rvol,
                CAST(average_volume AS REAL) as average_volume,
                CAST(volume AS REAL) as volume
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
    .fetch_all(&*pool)
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

    if !in_sample_equity.is_empty() {
        let _last_is = in_sample_equity.last().unwrap().cumulative_return_pct;
        let _adjusted_oos: Vec<EquityPoint> = out_of_sample_equity
            .iter()
            .map(|pt| EquityPoint {
                trade_index: pt.trade_index,
                cumulative_return_pct: if pt.trade_index == 0 {
                    _last_is
                } else {
                    _last_is + (pt.cumulative_return_pct - out_of_sample_equity.first().map(|p| p.cumulative_return_pct).unwrap_or(0.0))
                },
            })
            .collect();
    }

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
    use crate::edges::types::{
        EdgeArchetype, ExecutionConfig, IndicatorConfig, RegimeGates, SizingConfig,
        SizingModel, StopLossConfig, StopLossModel, TakeProfitConfig, TriggerPhase, TriggerRule,
    };

    #[test]
    fn test_classify_regime_trending() {
        let row = BacktestRow {
            timestamp: 1,
            close: 100.0,
            rsi_14: Some(55.0),
            macd_line: Some(1.0),
            macd_signal: Some(0.5),
            macd_crossover_detected: None,
            macd_crossover_direction: None,
            rsi_divergence_status: None,
            macd_divergence_status: None,
            adx_14: Some(30.0),
            adx_plus: Some(25.0),
            adx_minus: Some(15.0),
            adx_regime: None,
            adx_slope: None,
            bbwp: Some(50.0),
            squeeze_on: Some(false),
            squeeze_momentum: None,
            squeeze_release_trigger: None,
            atr_14: Some(1.0),
            ema_fast: Some(101.0),
            ema_medium: Some(99.0),
            ema_slow: Some(97.0),
            ema_long: Some(95.0),
            ema_stack_state: Some("bullish".to_string()),
            vwap: None,
            rvol: None,
            average_volume: None,
            volume: None,
        };
        assert_eq!(classify_regime(&row), "trending");
    }

    #[test]
    fn test_classify_regime_compression() {
        let row = BacktestRow {
            timestamp: 1,
            close: 100.0,
            rsi_14: None,
            macd_line: None,
            macd_signal: None,
            macd_crossover_detected: None,
            macd_crossover_direction: None,
            rsi_divergence_status: None,
            macd_divergence_status: None,
            adx_14: None,
            adx_plus: None,
            adx_minus: None,
            adx_regime: None,
            adx_slope: None,
            bbwp: Some(5.0),
            squeeze_on: Some(true),
            squeeze_momentum: None,
            squeeze_release_trigger: None,
            atr_14: None,
            ema_fast: None,
            ema_medium: None,
            ema_slow: None,
            ema_long: None,
            ema_stack_state: None,
            vwap: None,
            rvol: None,
            average_volume: None,
            volume: None,
        };
        assert_eq!(classify_regime(&row), "compression");
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
        let equity = vec![
            EquityPoint { trade_index: 0, cumulative_return_pct: 0.0 },
            EquityPoint { trade_index: 1, cumulative_return_pct: 2.0 },
            EquityPoint { trade_index: 2, cumulative_return_pct: 1.0 },
            EquityPoint { trade_index: 3, cumulative_return_pct: 4.0 },
            EquityPoint { trade_index: 4, cumulative_return_pct: 3.5 },
            EquityPoint { trade_index: 5, cumulative_return_pct: 4.5 },
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

    #[test]
    fn test_evaluate_rsi_oversold() {
        let indicator = IndicatorConfig {
            name: "rsi".to_string(),
            weight: 10.0,
            trigger_rule: TriggerRule::OverboughtOversold,
            enabled: true,
        };
        let row = BacktestRow {
            timestamp: 1,
            close: 100.0,
            rsi_14: Some(25.0),
            macd_line: None, macd_signal: None,
            macd_crossover_detected: None, macd_crossover_direction: None,
            rsi_divergence_status: None, macd_divergence_status: None,
            adx_14: None, adx_plus: None, adx_minus: None,
            adx_regime: None, adx_slope: None,
            bbwp: None, squeeze_on: None, squeeze_momentum: None, squeeze_release_trigger: None,
            atr_14: None,
            ema_fast: None, ema_medium: None, ema_slow: None, ema_long: None,
            ema_stack_state: None, vwap: None,
            rvol: None, average_volume: None, volume: None,
        };
        assert_eq!(evaluate_indicator_trigger(&row, &indicator), 10.0);
    }

    #[test]
    fn test_evaluate_macd_crossover_bullish() {
        let indicator = IndicatorConfig {
            name: "macd".to_string(),
            weight: 20.0,
            trigger_rule: TriggerRule::Crossover,
            enabled: true,
        };
        let row = BacktestRow {
            timestamp: 1,
            close: 100.0,
            rsi_14: None, macd_line: None, macd_signal: None,
            macd_crossover_detected: Some(true),
            macd_crossover_direction: Some("BULLISH".to_string()),
            rsi_divergence_status: None, macd_divergence_status: None,
            adx_14: None, adx_plus: None, adx_minus: None,
            adx_regime: None, adx_slope: None,
            bbwp: None, squeeze_on: None, squeeze_momentum: None, squeeze_release_trigger: None,
            atr_14: None,
            ema_fast: None, ema_medium: None, ema_slow: None, ema_long: None,
            ema_stack_state: None, vwap: None,
            rvol: None, average_volume: None, volume: None,
        };
        assert_eq!(evaluate_indicator_trigger(&row, &indicator), 20.0);
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
