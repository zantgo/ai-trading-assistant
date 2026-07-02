pub mod backtest;
pub mod bootstrap;
pub mod monte_carlo;
pub mod types;

use sqlx::SqlitePool;
use types::{
    BacktestCurveData, BootstrapResult, EdgeAnalysisResponse, EdgeConfig, EdgeSaveRequest,
    HistoricalMetrics, MonteCarloResult,
};

use crate::db;

pub async fn run_analysis(
    pool: &SqlitePool,
    edge_id: i64,
    symbol: &str,
    timeframe_secs: u64,
) -> Result<EdgeAnalysisResponse, String> {
    let edge = db::edges_get(pool, edge_id).await?;
    let config: EdgeConfig = serde_json::from_str(&edge.config_payload)
        .map_err(|e| format!("Failed to parse edge config: {}", e))?;

    if let Some(cached) = db::edge_analytics_cache_get(pool, edge_id).await {
        let metrics: HistoricalMetrics =
            serde_json::from_str(&cached.historical_metrics).unwrap_or({
                HistoricalMetrics {
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
                }
            });
        let mc: MonteCarloResult =
            serde_json::from_str(&cached.monte_carlo_paths).unwrap_or_else(|_| {
                MonteCarloResult {
                    paths: Vec::new(),
                    avg_final_return_pct: 0.0,
                    median_max_drawdown_pct: 0.0,
                    worst_case_drawdown_pct: 0.0,
                    drawdown_distribution: Vec::new(),
                    probability_of_ruin_pct: 0.0,
                    confidence_95_drawdown_pct: 0.0,
                }
            });
        let bs: BootstrapResult =
            serde_json::from_str(&cached.bootstrap_results).unwrap_or({
                BootstrapResult {
                    p_value: 1.0,
                    is_significant: false,
                    mean_return: 0.0,
                    confidence_95_lower: 0.0,
                    confidence_95_upper: 0.0,
                    iterations: 0,
                }
            });

        return Ok(EdgeAnalysisResponse {
            edge_id,
            edge_name: edge.name,
            symbol: symbol.to_string(),
            timeframe_secs,
            backtest_depth: config.backtest_depth,
            historical_metrics: metrics,
            backtest_curve: BacktestCurveData {
                in_sample: Vec::new(),
                out_of_sample: Vec::new(),
                combined: Vec::new(),
            },
            bootstrap_p_value: bs.p_value,
            bootstrap_significant: bs.is_significant,
            monte_carlo_paths: mc.paths,
            drawdown_distribution: mc.drawdown_distribution,
            probability_of_ruin_pct: mc.probability_of_ruin_pct,
            confidence_95_drawdown_pct: mc.confidence_95_drawdown_pct,
            skewness: 0.0,
            cached: true,
        });
    }

    let backtest_output =
        backtest::run_backtest(pool, &config, symbol, timeframe_secs).await?;

    let bootstrap_cfg = bootstrap::BootstrapConfig::default();
    let bootstrap_result =
        bootstrap::run_bootstrap(&backtest_output.trade_returns, &bootstrap_cfg);

    let mc_cfg = monte_carlo::MonteCarloConfig::default();
    let ruin_threshold = config.sizing.max_leverage.clamp(1.0, 20.0);
    let monte_carlo_result = monte_carlo::run_monte_carlo(
        &backtest_output.trade_returns,
        ruin_threshold * 5.0,
        &mc_cfg,
    );

    let skewness = bootstrap::compute_return_skewness(&backtest_output.trade_returns);

    let metrics_json = serde_json::to_string(&backtest_output.metrics).unwrap_or_default();
    let mc_json = serde_json::to_string(&monte_carlo_result).unwrap_or_default();
    let bs_json = serde_json::to_string(&bootstrap_result).unwrap_or_default();

    let _ = db::edge_analytics_cache_upsert(pool, edge_id, &metrics_json, &mc_json, &bs_json)
        .await;

    Ok(EdgeAnalysisResponse {
        edge_id,
        edge_name: edge.name,
        symbol: symbol.to_string(),
        timeframe_secs,
        backtest_depth: config.backtest_depth,
        historical_metrics: backtest_output.metrics,
        backtest_curve: BacktestCurveData {
            in_sample: backtest_output.in_sample_equity,
            out_of_sample: backtest_output.out_of_sample_equity,
            combined: backtest_output.equity_curve,
        },
        bootstrap_p_value: bootstrap_result.p_value,
        bootstrap_significant: bootstrap_result.is_significant,
        monte_carlo_paths: monte_carlo_result.paths,
        drawdown_distribution: monte_carlo_result.drawdown_distribution,
        probability_of_ruin_pct: monte_carlo_result.probability_of_ruin_pct,
        confidence_95_drawdown_pct: monte_carlo_result.confidence_95_drawdown_pct,
        skewness,
        cached: false,
    })
}

pub async fn save_edge(pool: &SqlitePool, req: EdgeSaveRequest) -> Result<i64, String> {
    if req.name.trim().is_empty() {
        return Err("Edge name is required".to_string());
    }

    let config_json =
        serde_json::to_string(&req.config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    let id = db::edges_insert(pool, req.name.trim(), &req.pair_key, &req.description, &config_json)
        .await;

    if id > 0 {
        Ok(id)
    } else {
        Err("Failed to save edge (name may already exist)".to_string())
    }
}
