use core_domain::performance::TradeAnalyticsRecord;
use sqlx::SqlitePool;

pub async fn reconstruct_trades(pool: &SqlitePool) -> Vec<TradeAnalyticsRecord> {
    let mut records: Vec<TradeAnalyticsRecord> = Vec::new();

    let telemetry = fetch_telemetry_trades(pool).await;
    let paper = fetch_paper_trades(pool).await;

    let mut seen_ids = std::collections::HashSet::new();

    for row in telemetry {
        if seen_ids.insert(format!("T-{}", row.id)) {
            let record = build_record(pool, &row).await;
            records.push(record);
        }
    }

    for row in paper {
        if seen_ids.insert(format!("P-{}", row.id)) {
            let record = build_paper_record(pool, &row).await;
            records.push(record);
        }
    }

    records.sort_by_key(|a| a.exit_timestamp);
    records
}

async fn build_record(pool: &SqlitePool, row: &TelemetryQueryRow) -> TradeAnalyticsRecord {
    let hold_time_seconds = ((row.exit_timestamp - row.entry_timestamp).max(0) / 1000) as u64;
    let gross_pnl = row.realized_pnl;
    let fees = row.commission_fees + row.funding_fees;
    let net_pnl = gross_pnl - fees;
    let roi_pct = row.roi_pct;
    let flat_trade = gross_pnl.abs() < f64::EPSILON;

    let slippage = estimate_slippage(pool, row).await;
    let (mfe, mae) = compute_mfe_mae(pool, row).await;
    let exit_reason = derive_exit_reason(&row.trigger_source, gross_pnl);

    TradeAnalyticsRecord {
        trade_id: format!("T-{}", row.id),
        symbol: row.symbol.clone(),
        direction: row.direction.clone(),
        entry_timestamp: row.entry_timestamp,
        exit_timestamp: row.exit_timestamp,
        hold_time_seconds,
        entry_price: row.entry_price,
        exit_price: row.exit_price,
        size: row.size,
        gross_pnl,
        net_pnl,
        roi_pct,
        execution_slippage: slippage,
        mfe,
        mae,
        trigger_source: row.trigger_source.clone(),
        exit_reason,
        flat_trade,
    }
}

async fn build_paper_record(pool: &SqlitePool, row: &PaperTradeQueryRow) -> TradeAnalyticsRecord {
    let hold_time_seconds = ((row.exit_timestamp - row.entry_timestamp).max(0) / 1000) as u64;
    let gross_pnl = row.realized_pnl;
    let net_pnl = row.realized_pnl;
    let roi_pct = row.roi_pct;
    let flat_trade = gross_pnl.abs() < f64::EPSILON;

    let slippage = 0.0;
    let (mfe, mae) = compute_mfe_mae_for_paper(pool, row).await;
    let exit_reason = derive_exit_reason(&row.trigger, gross_pnl);

    TradeAnalyticsRecord {
        trade_id: format!("P-{}", row.id),
        symbol: row.symbol.clone(),
        direction: row.direction.clone(),
        entry_timestamp: row.entry_timestamp,
        exit_timestamp: row.exit_timestamp,
        hold_time_seconds,
        entry_price: row.entry_price,
        exit_price: row.exit_price,
        size: row.size,
        gross_pnl,
        net_pnl,
        roi_pct,
        execution_slippage: slippage,
        mfe,
        mae,
        trigger_source: row.trigger.clone(),
        exit_reason,
        flat_trade,
    }
}

async fn estimate_slippage(_pool: &SqlitePool, row: &TelemetryQueryRow) -> f64 {
    let direction_mult = if row.direction.to_uppercase() == "SHORT" {
        -1.0
    } else {
        1.0
    };
    let expected_move = (row.exit_price - row.entry_price) * direction_mult;
    if expected_move.abs() < f64::EPSILON {
        return 0.0;
    }
    let realized_per_unit = row.realized_pnl / row.size;
    let expected_per_unit = expected_move;
    let diff = (expected_per_unit - realized_per_unit).abs();
    diff.min(expected_per_unit.abs())
}

async fn compute_mfe_mae(pool: &SqlitePool, row: &TelemetryQueryRow) -> (f64, f64) {
    let mfe_val = query_market_extreme(
        pool,
        &row.symbol,
        row.entry_timestamp,
        row.exit_timestamp,
        true,
    )
    .await;
    let mae_val = query_market_extreme(
        pool,
        &row.symbol,
        row.entry_timestamp,
        row.exit_timestamp,
        false,
    )
    .await;

    let is_long = row.direction.to_uppercase() == "LONG";
    let (mfe, mae) = if is_long {
        let favorable = if let Some(high) = mfe_val {
            (high - row.entry_price) * row.size
        } else {
            row.realized_pnl.max(0.0)
        };
        let adverse = if let Some(low) = mae_val {
            (low - row.entry_price) * row.size
        } else {
            row.realized_pnl.min(0.0)
        };
        (favorable, adverse)
    } else {
        let favorable = if let Some(low) = mae_val {
            (row.entry_price - low) * row.size
        } else {
            row.realized_pnl.max(0.0)
        };
        let adverse = if let Some(high) = mfe_val {
            (row.entry_price - high) * row.size
        } else {
            row.realized_pnl.min(0.0)
        };
        (favorable, adverse)
    };

    (mfe.max(0.0), mae.min(0.0))
}

async fn compute_mfe_mae_for_paper(pool: &SqlitePool, row: &PaperTradeQueryRow) -> (f64, f64) {
    let mfe_val = query_market_extreme(
        pool,
        &row.symbol,
        row.entry_timestamp,
        row.exit_timestamp,
        true,
    )
    .await;
    let mae_val = query_market_extreme(
        pool,
        &row.symbol,
        row.entry_timestamp,
        row.exit_timestamp,
        false,
    )
    .await;

    let is_long = row.direction.to_uppercase() == "LONG";
    let (mfe, mae) = if is_long {
        let favorable = if let Some(high) = mfe_val {
            (high - row.entry_price) * row.size
        } else {
            row.realized_pnl.max(0.0)
        };
        let adverse = if let Some(low) = mae_val {
            (low - row.entry_price) * row.size
        } else {
            row.realized_pnl.min(0.0)
        };
        (favorable, adverse)
    } else {
        let favorable = if let Some(low) = mae_val {
            (row.entry_price - low) * row.size
        } else {
            row.realized_pnl.max(0.0)
        };
        let adverse = if let Some(high) = mfe_val {
            (row.entry_price - high) * row.size
        } else {
            row.realized_pnl.min(0.0)
        };
        (favorable, adverse)
    };

    (mfe.max(0.0), mae.min(0.0))
}

async fn query_market_extreme(
    pool: &SqlitePool,
    symbol: &str,
    entry_ts: i64,
    exit_ts: i64,
    is_high: bool,
) -> Option<f64> {
    let column = if is_high { "high" } else { "low" };
    let query = format!(
        "SELECT {} FROM market_snapshots
         WHERE symbol = ?1 AND timestamp >= ?2 AND timestamp <= ?3
         AND {} IS NOT NULL AND {} != ''
         ORDER BY CAST({} AS REAL) {} LIMIT 1",
        column,
        column,
        column,
        column,
        if is_high { "DESC" } else { "ASC" }
    );

    let result: Result<Option<String>, _> = sqlx::query_scalar(&query)
        .bind(symbol)
        .bind(entry_ts)
        .bind(exit_ts)
        .fetch_optional(pool)
        .await;

    match result {
        Ok(Some(raw)) => raw.parse::<f64>().ok(),
        _ => None,
    }
}

async fn fetch_telemetry_trades(pool: &SqlitePool) -> Vec<TelemetryQueryRow> {
    sqlx::query_as::<_, TelemetryQueryRow>(
        "SELECT id, symbol, direction, entry_timestamp, exit_timestamp,
                entry_price, exit_price, size, realized_pnl, commission_fees,
                funding_fees, roi_pct, trigger_source
         FROM trade_telemetry_history
         ORDER BY exit_timestamp ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

async fn fetch_paper_trades(pool: &SqlitePool) -> Vec<PaperTradeQueryRow> {
    sqlx::query_as::<_, PaperTradeQueryRow>(
        "SELECT id, symbol, direction, entry_price, exit_price,
                size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp,
                trigger
         FROM paper_trades
         ORDER BY exit_timestamp ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

fn derive_exit_reason(trigger_source: &str, gross_pnl: f64) -> String {
    let upper = trigger_source.to_uppercase();
    if upper.contains("STOP_LOSS") || upper.contains("SL") {
        "STOP_LOSS".into()
    } else if upper.contains("TAKE_PROFIT") || upper.contains("TP") {
        "TAKE_PROFIT".into()
    } else if upper.contains("SIGNAL") || upper.contains("EXIT") {
        "SIGNAL_EXIT".into()
    } else if upper.contains("LIQUIDATION") || upper.contains("LIQ") {
        "EMERGENCY_LIQUIDATION".into()
    } else if upper.contains("MANUAL") {
        "MANUAL".into()
    } else if gross_pnl > 0.0 {
        "TAKE_PROFIT".into()
    } else if gross_pnl < 0.0 {
        "STOP_LOSS".into()
    } else {
        "MANUAL".into()
    }
}

pub fn compute_efficiency_metrics(trades: &[TradeAnalyticsRecord]) -> Vec<f64> {
    trades
        .iter()
        .map(|t| {
            if t.gross_pnl.abs() < f64::EPSILON {
                0.0
            } else {
                (t.gross_pnl - t.net_pnl) / t.gross_pnl.abs()
            }
        })
        .collect()
}

#[derive(Debug, sqlx::FromRow)]
struct TelemetryQueryRow {
    id: i64,
    symbol: String,
    direction: String,
    entry_timestamp: i64,
    exit_timestamp: i64,
    entry_price: f64,
    exit_price: f64,
    size: f64,
    realized_pnl: f64,
    commission_fees: f64,
    funding_fees: f64,
    roi_pct: f64,
    trigger_source: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PaperTradeQueryRow {
    id: i64,
    symbol: String,
    direction: String,
    entry_price: f64,
    exit_price: f64,
    size: f64,
    realized_pnl: f64,
    roi_pct: f64,
    entry_timestamp: i64,
    exit_timestamp: i64,
    trigger: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_exit_reason_stop_loss() {
        assert_eq!(derive_exit_reason("STOP_LOSS_TRIGGER", -100.0), "STOP_LOSS");
        assert_eq!(derive_exit_reason("SL_HIT", -50.0), "STOP_LOSS");
    }

    #[test]
    fn test_derive_exit_reason_take_profit() {
        assert_eq!(
            derive_exit_reason("TAKE_PROFIT_TARGET", 100.0),
            "TAKE_PROFIT"
        );
        assert_eq!(derive_exit_reason("TP_LEVEL", 75.0), "TAKE_PROFIT");
    }

    #[test]
    fn test_derive_exit_reason_signal_exit() {
        assert_eq!(derive_exit_reason("SIGNAL_CROSSOVER", 10.0), "SIGNAL_EXIT");
        assert_eq!(derive_exit_reason("EXIT_POSITION", -5.0), "SIGNAL_EXIT");
    }

    #[test]
    fn test_derive_exit_reason_liquidation() {
        assert_eq!(
            derive_exit_reason("LIQUIDATION", -200.0),
            "EMERGENCY_LIQUIDATION"
        );
        assert_eq!(derive_exit_reason("LIQ", -200.0), "EMERGENCY_LIQUIDATION");
    }

    #[test]
    fn test_derive_exit_reason_manual() {
        assert_eq!(derive_exit_reason("MANUAL", 50.0), "MANUAL");
    }

    #[test]
    fn test_derive_exit_reason_fallback_profit() {
        assert_eq!(derive_exit_reason("UNKNOWN_ALGO", 100.0), "TAKE_PROFIT");
    }

    #[test]
    fn test_derive_exit_reason_fallback_loss() {
        assert_eq!(derive_exit_reason("UNKNOWN_ALGO", -50.0), "STOP_LOSS");
    }

    #[test]
    fn test_derive_exit_reason_fallback_flat() {
        assert_eq!(derive_exit_reason("UNKNOWN_ALGO", 0.0), "MANUAL");
    }

    #[test]
    fn test_efficiency_metrics_positive_gross() {
        let trades = vec![TradeAnalyticsRecord {
            trade_id: "T-1".into(),
            symbol: "BTC-USDT".into(),
            direction: "LONG".into(),
            entry_timestamp: 1000,
            exit_timestamp: 2000,
            hold_time_seconds: 1,
            entry_price: 100.0,
            exit_price: 110.0,
            size: 1.0,
            gross_pnl: 10.0,
            net_pnl: 8.0,
            roi_pct: 10.0,
            execution_slippage: 0.0,
            mfe: 15.0,
            mae: -2.0,
            trigger_source: "MANUAL".into(),
            exit_reason: "TAKE_PROFIT".into(),
            flat_trade: false,
        }];
        let metrics = compute_efficiency_metrics(&trades);
        assert!((metrics[0] - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_efficiency_metrics_flat_trade() {
        let trades = vec![TradeAnalyticsRecord {
            trade_id: "T-1".into(),
            symbol: "BTC-USDT".into(),
            direction: "LONG".into(),
            entry_timestamp: 1000,
            exit_timestamp: 2000,
            hold_time_seconds: 1,
            entry_price: 100.0,
            exit_price: 100.0,
            size: 1.0,
            gross_pnl: 0.0,
            net_pnl: 0.0,
            roi_pct: 0.0,
            execution_slippage: 0.0,
            mfe: 0.0,
            mae: 0.0,
            trigger_source: "MANUAL".into(),
            exit_reason: "MANUAL".into(),
            flat_trade: true,
        }];
        let metrics = compute_efficiency_metrics(&trades);
        assert_eq!(metrics[0], 0.0);
    }

    #[test]
    fn test_efficiency_metrics_empty() {
        let metrics = compute_efficiency_metrics(&[]);
        assert!(metrics.is_empty());
    }
}
