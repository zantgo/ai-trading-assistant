use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::backtest::metrics::{compute_cagr, compute_max_drawdown, compute_sharpe, compute_sortino};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    #[serde(default = "default_initial_capital")]
    pub initial_capital: f64,
    #[serde(default = "default_slippage_pct")]
    pub slippage_pct: f64,
    #[serde(default = "default_commission_pct")]
    pub commission_pct: f64,
    #[serde(default = "default_risk_free_rate")]
    pub risk_free_rate: f64,
    #[serde(default = "default_quorum_threshold")]
    pub quorum_threshold: f64,
}

fn default_initial_capital() -> f64 { 10000.0 }
fn default_slippage_pct() -> f64 { 0.05 }
fn default_commission_pct() -> f64 { 0.04 }
fn default_risk_free_rate() -> f64 { 0.02 }
fn default_quorum_threshold() -> f64 { 60.0 }

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: default_initial_capital(),
            slippage_pct: default_slippage_pct(),
            commission_pct: default_commission_pct(),
            risk_free_rate: default_risk_free_rate(),
            quorum_threshold: default_quorum_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub total_return_pct: f64,
    pub cagr: f64,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub total_trades: usize,
    pub avg_r_multiple: f64,
    pub equity_curve: Vec<(i64, f64)>,
    pub drawdown_curve: Vec<(i64, f64)>,
    pub trades: Vec<BacktestTrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTrade {
    pub entry_time: i64,
    pub exit_time: i64,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl_pct: f64,
    pub regime: String,
    pub confluence_score: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
struct HistoricalRow {
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
    squeeze_state_label: Option<String>,
    bbwp_state_label: Option<String>,
    ema_stack_state_label: Option<String>,
    rvol_state_label: Option<String>,
    adx_state_label: Option<String>,
}

fn normalized_for(row: &HistoricalRow, name: &str) -> f64 {
    match name {
        "rsi" => row.rsi_normalized,
        "macd" => row.macd_normalized,
        "squeeze" => row.squeeze_normalized,
        "adx" => row.adx_normalized,
        "bbwp" => row.bbwp_normalized,
        "rvol" => row.rvol_normalized,
        "ema" => row.ema_stack_normalized,
        _ => None,
    }
    .unwrap_or(0.0)
}

fn adx_congestion(row: &HistoricalRow) -> bool {
    row.adx_state_label.as_deref() == Some("TRENDLESS_CONGESTION")
        || row.adx_normalized.is_none_or(|v| v.abs() < 0.05)
}

fn squeeze_release(row: &HistoricalRow) -> bool {
    row.squeeze_state_label
        .as_deref()
        .is_some_and(|l| l.ends_with("VOLATILITY_RELEASE"))
}

fn rvol_institutional(row: &HistoricalRow) -> bool {
    matches!(
        row.rvol_state_label.as_deref(),
        Some("INSTITUTIONAL_BREAKOUT_VOLUME")
    )
}

fn bbwp_climax(row: &HistoricalRow) -> bool {
    row.bbwp_state_label.as_deref() == Some("VOLATILITY_EXHAUSTION_REVERSION_WARNING")
}

fn classify_regime(row: &HistoricalRow) -> String {
    let squeeze = row.squeeze_state_label.as_deref().unwrap_or("");
    let bbwp = row.bbwp_state_label.as_deref().unwrap_or("");
    let ema = row.ema_stack_state_label.as_deref().unwrap_or("");
    let adx_norm = row.adx_normalized.unwrap_or(0.0);
    let stacked = ema.contains("BULLISH") || ema.contains("BEARISH");

    if squeeze == "COMPRESSION_COILING" || bbwp == "MAX_VOLATILITY_COMPRESSION" {
        "compression".to_string()
    } else if squeeze.ends_with("VOLATILITY_RELEASE")
        || bbwp == "VOLATILITY_EXHAUSTION_REVERSION_WARNING"
    {
        "expansion".to_string()
    } else if adx_norm.abs() >= 0.5 && stacked {
        "trending".to_string()
    } else {
        "range".to_string()
    }
}

fn compute_confluence_score(row: &HistoricalRow) -> f64 {
    let congested = adx_congestion(row);
    let breakout_active = squeeze_release(row);
    let rvol_confirmed = rvol_institutional(row);

    let weights: &[(&str, f64)] = &[
        ("rsi", 10.0),
        ("macd", 10.0),
        ("squeeze", 15.0),
        ("adx", 15.0),
        ("bbwp", 15.0),
        ("rvol", 15.0),
        ("ema", 20.0),
    ];

    let mut score = 0.0_f64;
    let mut ema_weight = 0.0_f64;
    let mut ema_norm = 0.0_f64;

    for (name, weight) in weights {
        let norm = normalized_for(row, name);
        let mut contrib = norm * weight;

        match *name {
            "ema" => {
                ema_weight = *weight;
                ema_norm = norm;
                if congested {
                    contrib = 0.0;
                }
            }
            "squeeze" | "macd" if breakout_active && !rvol_confirmed => {
                contrib *= 0.3;
            }
            _ => {}
        }
        score += contrib;
    }

    if bbwp_climax(row) && ema_weight > 0.0 {
        score += -0.1 * ema_norm.signum() * ema_weight;
    }

    score.clamp(-90.0, 90.0)
}

pub struct BacktestEngine {
    pool: SqlitePool,
    symbol: String,
    start_ts: i64,
    end_ts: i64,
    config: BacktestConfig,
}

impl BacktestEngine {
    pub fn new(
        pool: SqlitePool,
        symbol: String,
        start_ts: i64,
        end_ts: i64,
        config: BacktestConfig,
    ) -> Self {
        Self {
            pool,
            symbol,
            start_ts,
            end_ts,
            config,
        }
    }

    pub async fn run(&mut self) -> Result<BacktestResult, String> {
        let rows: Vec<HistoricalRow> = sqlx::query_as(
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
                    squeeze_state_label,
                    bbwp_state_label,
                    ema_stack_state_label,
                    rvol_state_label,
                    adx_state_label
             FROM market_snapshots
             WHERE symbol = ?1
               AND timestamp >= ?2
               AND timestamp <= ?3
               AND close IS NOT NULL
             ORDER BY timestamp ASC",
        )
        .bind(&self.symbol)
        .bind(self.start_ts)
        .bind(self.end_ts)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query snapshots: {}", e))?;

        if rows.len() < 2 {
            return Err("Insufficient historical data for the selected symbol and time range"
                .to_string());
        }

        let initial_capital = self.config.initial_capital;
        let commission_pct = self.config.commission_pct / 100.0;
        let slippage_pct = self.config.slippage_pct / 100.0;
        let quorum_threshold = self.config.quorum_threshold;

        let mut trades: Vec<BacktestTrade> = Vec::new();
        let mut equity_curve: Vec<(i64, f64)> = vec![(rows[0].timestamp, initial_capital)];
        let mut trade_returns: Vec<f64> = Vec::new();

        let mut in_position = false;
        let mut current_direction = String::new();
        let mut entry_price = 0.0_f64;
        let mut entry_regime = String::new();
        let mut entry_confluence = 0.0_f64;
        let mut peak_equity = initial_capital;
        let mut current_equity = initial_capital;

        for (i, row) in rows.iter().enumerate() {
            if !in_position {
                let regime = classify_regime(row);
                if regime != "trending" && regime != "expansion" {
                    continue;
                }

                let score = compute_confluence_score(row);
                let abs_score = score.abs();

                if abs_score < quorum_threshold {
                    continue;
                }

                let direction = if score > 0.0 { "LONG" } else { "SHORT" };

                entry_price = row.close;
                entry_regime = regime;
                entry_confluence = score;
                current_direction = direction.to_string();
                in_position = true;
            } else {
                let reverse_score = compute_confluence_score(row);
                let exit_signal = if current_direction == "LONG" {
                    reverse_score < -quorum_threshold
                } else {
                    reverse_score > quorum_threshold
                };

                let force_exit = i >= rows.len() - 1;

                if exit_signal || force_exit {
                    let exit_price = if force_exit {
                        row.close
                    } else {
                        let slip = if current_direction == "LONG" {
                            row.close * (1.0 - slippage_pct)
                        } else {
                            row.close * (1.0 + slippage_pct)
                        };
                        slip
                    };

                    let raw_pnl = if current_direction == "LONG" {
                        (exit_price - entry_price) / entry_price
                    } else {
                        (entry_price - exit_price) / entry_price
                    };

                    let commission_cost = commission_pct * 2.0;
                    let net_pnl = raw_pnl - commission_cost;

                    let pnl_amount = current_equity * net_pnl;
                    current_equity += pnl_amount;

                    equity_curve.push((row.timestamp, current_equity));
                    if current_equity > peak_equity {
                        peak_equity = current_equity;
                    }

                    trade_returns.push(net_pnl * 100.0);

                    trades.push(BacktestTrade {
                        entry_time: rows.iter().find(|r| r.close == entry_price).map(|r| r.timestamp).unwrap_or(0),
                        exit_time: row.timestamp,
                        direction: current_direction.clone(),
                        entry_price,
                        exit_price: row.close,
                        pnl_pct: net_pnl * 100.0,
                        regime: entry_regime.clone(),
                        confluence_score: entry_confluence,
                    });

                    in_position = false;
                }
            }
        }

        if in_position {
            let last = rows.last().unwrap();
            let pnl = if current_direction == "LONG" {
                ((last.close - entry_price) / entry_price) - commission_pct
            } else {
                ((entry_price - last.close) / entry_price) - commission_pct
            };
            current_equity += current_equity * pnl;
            equity_curve.push((last.timestamp, current_equity));

            trade_returns.push(pnl * 100.0);
            trades.push(BacktestTrade {
                entry_time: rows.iter().find(|r| r.close == entry_price).map(|r| r.timestamp).unwrap_or(0),
                exit_time: last.timestamp,
                direction: current_direction.clone(),
                entry_price,
                exit_price: last.close,
                pnl_pct: pnl * 100.0,
                regime: entry_regime,
                confluence_score: entry_confluence,
            });
        }

        let total_return_pct = if initial_capital > 0.0 {
            ((current_equity - initial_capital) / initial_capital) * 100.0
        } else {
            0.0
        };

        let drawdown_curve: Vec<(i64, f64)> = {
            let mut peak = f64::NEG_INFINITY;
            equity_curve
                .iter()
                .map(|&(ts, val)| {
                    if val > peak {
                        peak = val;
                    }
                    let dd = if peak > 0.0 { (peak - val) / peak * 100.0 } else { 0.0 };
                    (ts, dd)
                })
                .collect()
        };

        let max_drawdown_pct = compute_max_drawdown(&equity_curve);

        let days = if equity_curve.len() >= 2 {
            let first_ts = equity_curve.first().map(|e| e.0).unwrap_or(0);
            let last_ts = equity_curve.last().map(|e| e.0).unwrap_or(0);
            ((last_ts - first_ts) as f64) / 86_400.0
        } else {
            0.0
        };

        let cagr = compute_cagr(initial_capital, current_equity, days);

        let daily_returns: Vec<f64> = trade_returns
            .iter()
            .map(|r| r / 100.0)
            .collect();

        let sharpe_ratio = compute_sharpe(&daily_returns, self.config.risk_free_rate);
        let sortino_ratio = compute_sortino(&daily_returns, self.config.risk_free_rate);

        let total_trades = trades.len();
        let wins: Vec<f64> = trade_returns.iter().filter(|&&r| r > 0.0).copied().collect();
        let losses: Vec<f64> = trade_returns.iter().filter(|&&r| r <= 0.0).copied().collect();
        let win_rate = if total_trades > 0 {
            wins.len() as f64 / total_trades as f64
        } else {
            0.0
        };

        let gross_profit: f64 = wins.iter().sum();
        let gross_loss: f64 = losses.iter().map(|l| l.abs()).sum();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        let avg_r_multiple = if total_trades > 0 {
            let avg_win = if !wins.is_empty() {
                wins.iter().sum::<f64>() / wins.len() as f64
            } else {
                0.0
            };
            let avg_loss = if !losses.is_empty() {
                losses.iter().map(|l| l.abs()).sum::<f64>() / losses.len() as f64
            } else {
                0.0
            };
            if avg_loss > 0.0 {
                avg_win / avg_loss
            } else {
                avg_win
            }
        } else {
            0.0
        };

        Ok(BacktestResult {
            total_return_pct,
            cagr,
            max_drawdown_pct,
            sharpe_ratio,
            sortino_ratio,
            win_rate,
            profit_factor,
            total_trades,
            avg_r_multiple,
            equity_curve,
            drawdown_curve,
            trades,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_regime_trending() {
        let r = HistoricalRow {
            timestamp: 1,
            close: 100.0,
            atr_14: Some(1.0),
            rsi_normalized: None,
            macd_normalized: None,
            squeeze_normalized: None,
            adx_normalized: Some(0.7),
            bbwp_normalized: None,
            rvol_normalized: None,
            ema_stack_normalized: None,
            squeeze_state_label: None,
            bbwp_state_label: None,
            ema_stack_state_label: Some("ESTABLISHED_BULLISH_STACK".to_string()),
            rvol_state_label: None,
            adx_state_label: None,
        };
        assert_eq!(classify_regime(&r), "trending");
    }

    #[test]
    fn test_classify_regime_compression() {
        let r = HistoricalRow {
            timestamp: 1,
            close: 100.0,
            atr_14: None,
            rsi_normalized: None,
            macd_normalized: None,
            squeeze_normalized: None,
            adx_normalized: None,
            bbwp_normalized: None,
            rvol_normalized: None,
            ema_stack_normalized: None,
            squeeze_state_label: Some("COMPRESSION_COILING".to_string()),
            bbwp_state_label: None,
            ema_stack_state_label: None,
            rvol_state_label: None,
            adx_state_label: None,
        };
        assert_eq!(classify_regime(&r), "compression");
    }

    #[test]
    fn test_classify_regime_expansion() {
        let r = HistoricalRow {
            timestamp: 1,
            close: 100.0,
            atr_14: None,
            rsi_normalized: None,
            macd_normalized: None,
            squeeze_normalized: None,
            adx_normalized: None,
            bbwp_normalized: None,
            rvol_normalized: None,
            ema_stack_normalized: None,
            squeeze_state_label: Some("BULLISH_VOLATILITY_RELEASE".to_string()),
            bbwp_state_label: None,
            ema_stack_state_label: None,
            rvol_state_label: None,
            adx_state_label: None,
        };
        assert_eq!(classify_regime(&r), "expansion");
    }

    #[test]
    fn test_classify_regime_range() {
        let r = HistoricalRow {
            timestamp: 1,
            close: 100.0,
            atr_14: None,
            rsi_normalized: None,
            macd_normalized: None,
            squeeze_normalized: None,
            adx_normalized: Some(0.1),
            bbwp_normalized: None,
            rvol_normalized: None,
            ema_stack_normalized: None,
            squeeze_state_label: None,
            bbwp_state_label: None,
            ema_stack_state_label: None,
            rvol_state_label: None,
            adx_state_label: None,
        };
        assert_eq!(classify_regime(&r), "range");
    }

    #[test]
    fn test_confluence_score_mixed() {
        let r = HistoricalRow {
            timestamp: 1,
            close: 100.0,
            atr_14: Some(1.0),
            rsi_normalized: Some(0.8),
            macd_normalized: Some(0.9),
            squeeze_normalized: Some(0.5),
            adx_normalized: Some(0.7),
            bbwp_normalized: Some(0.3),
            rvol_normalized: Some(0.5),
            ema_stack_normalized: Some(1.0),
            squeeze_state_label: None,
            bbwp_state_label: None,
            ema_stack_state_label: Some("ESTABLISHED_BULLISH_STACK".to_string()),
            rvol_state_label: None,
            adx_state_label: Some("STRONG_BULL_TREND".to_string()),
        };
        let score = compute_confluence_score(&r);
        assert!(score > 0.0);
        assert!(score <= 90.0);
    }

    #[test]
    fn test_confluence_score_clamps() {
        let r = HistoricalRow {
            timestamp: 1,
            close: 100.0,
            atr_14: None,
            rsi_normalized: Some(1.0),
            macd_normalized: Some(1.0),
            squeeze_normalized: Some(1.0),
            adx_normalized: Some(1.0),
            bbwp_normalized: Some(1.0),
            rvol_normalized: Some(1.0),
            ema_stack_normalized: Some(1.0),
            squeeze_state_label: None,
            bbwp_state_label: None,
            ema_stack_state_label: Some("ESTABLISHED_BULLISH_STACK".to_string()),
            rvol_state_label: Some("INSTITUTIONAL_BREAKOUT_VOLUME".to_string()),
            adx_state_label: Some("STRONG_BULL_TREND".to_string()),
        };
        let score = compute_confluence_score(&r);
        assert_eq!(score, 90.0);
    }
}
