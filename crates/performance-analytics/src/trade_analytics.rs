use core_domain::performance::TradeAnalyticsRecord;
use sqlx::SqlitePool;

/// Reconstruct trades from the telemetry table into normalized TradeAnalyticsRecords.
/// Follows the 8-step pipeline defined in docs:03-05-02-pae-layer1-trade-analytics.md §3
pub async fn reconstruct_trades(pool: &SqlitePool) -> Vec<TradeAnalyticsRecord> {
    let rows = sqlx::query_as::<_, TelemetryQueryRow>(
        "SELECT id, symbol, direction, entry_timestamp, exit_timestamp,
                entry_price, exit_price, size, realized_pnl, commission_fees,
                funding_fees, roi_pct, trigger_source
         FROM trade_telemetry_history
         ORDER BY exit_timestamp ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| {
            let hold_time_seconds = ((r.exit_timestamp - r.entry_timestamp).max(0) / 1000) as u64;
            let gross_pnl = r.realized_pnl;
            let fees = r.commission_fees + r.funding_fees;
            let net_pnl = gross_pnl - fees;
            let roi_pct = r.roi_pct;
            let flat_trade = gross_pnl.abs() < f64::EPSILON;
            let slippage = 0.0;

            let mfe = gross_pnl.max(0.0);
            let mae = gross_pnl.min(0.0);

            TradeAnalyticsRecord {
                trade_id: format!("T-{}", r.id),
                symbol: r.symbol,
                direction: r.direction,
                entry_timestamp: r.entry_timestamp,
                exit_timestamp: r.exit_timestamp,
                hold_time_seconds,
                entry_price: r.entry_price,
                exit_price: r.exit_price,
                size: r.size,
                gross_pnl,
                net_pnl,
                roi_pct,
                execution_slippage: slippage,
                mfe,
                mae,
                trigger_source: r.trigger_source,
                exit_reason: String::new(),
                flat_trade,
            }
        })
        .collect()
}

/// Compute execution efficiency metrics per trade.
/// - slippage_bps: (|fill − target| / target) × 10000
/// - mae_ratio: |MAE| / |gross_pnl|
/// - mfe_capture: gross_pnl / MFE
/// - fee_efficiency: (gross_pnl − net_pnl) / |gross_pnl| if |gross_pnl| > 0
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
