use shared::models::MarketSnapshot;
use sqlx::SqlitePool;

pub async fn insert_snapshot_internal(pool: &SqlitePool, snapshot: &MarketSnapshot) {
    let sqz_on_db_val = snapshot.squeeze_on.map(|s| if s { 1 } else { 0 });
    let exchange_label = snapshot
        .exchange
        .as_ref()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "Hyperliquid".to_string());

    if let Err(e) = sqlx::query(
        "INSERT INTO market_snapshots (
            exchange, timeframe_secs, timestamp, symbol, mid_price, bid_price, ask_price,
            open, high, low, close, volume, average_volume,
            bb_upper, bb_middle, bb_lower, atr_14, vwap,
            ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
            macd_line, macd_signal, macd_hist, adx_14, adx_plus, adx_minus,
            squeeze_on, squeeze_momentum, bbwp, support_levels, resistance_levels
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34)"
    )
    .bind(exchange_label)
    .bind(snapshot.timeframe_secs as i64)
    .bind(snapshot.timestamp as i64)
    .bind(&snapshot.symbol)
    .bind(snapshot.mid_price.to_string())
    .bind(snapshot.bid_price.to_string())
    .bind(snapshot.ask_price.to_string())
    .bind(snapshot.open.map(|d| d.to_string()))
    .bind(snapshot.high.map(|d| d.to_string()))
    .bind(snapshot.low.map(|d| d.to_string()))
    .bind(snapshot.close.map(|d| d.to_string()))
    .bind(snapshot.volume.map(|d| d.to_string()))
    .bind(snapshot.average_volume.map(|d| d.to_string()))
    .bind(snapshot.bb_upper.map(|d| d.to_string()))
    .bind(snapshot.bb_middle.map(|d| d.to_string()))
    .bind(snapshot.bb_lower.map(|d| d.to_string()))
    .bind(snapshot.atr_14.map(|d| d.to_string()))
    .bind(snapshot.vwap.map(|d| d.to_string()))
    .bind(snapshot.ema_fast.map(|d| d.to_string()))
    .bind(snapshot.ema_medium.map(|d| d.to_string()))
    .bind(snapshot.ema_slow.map(|d| d.to_string()))
    .bind(snapshot.ema_long.map(|d| d.to_string()))
    .bind(snapshot.rsi_14.map(|d| d.to_string()))
    .bind(snapshot.macd_line.map(|d| d.to_string()))
    .bind(snapshot.macd_signal.map(|d| d.to_string()))
    .bind(snapshot.macd_hist.map(|d| d.to_string()))
    .bind(snapshot.adx_14.map(|d| d.to_string()))
    .bind(snapshot.adx_plus.map(|d| d.to_string()))
    .bind(snapshot.adx_minus.map(|d| d.to_string()))
    .bind(sqz_on_db_val)
    .bind(snapshot.squeeze_momentum.map(|d| d.to_string()))
    .bind(snapshot.bbwp.map(|d| d.to_string()))
    .bind(snapshot.support_levels.clone())
    .bind(snapshot.resistance_levels.clone())
    .execute(&*pool)
    .await
    {
        eprintln!("Database Error: Failed to save completed snapshot: {}", e);
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IndicatorSnapshotRow {
    pub timestamp: i64,
    pub rsi_14: Option<String>,
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<String>,
    pub macd_line: Option<String>,
    pub macd_signal: Option<String>,
    pub macd_hist: Option<String>,
    pub adx_14: Option<String>,
    pub adx_plus: Option<String>,
    pub adx_minus: Option<String>,
    pub atr_14: Option<String>,
    pub bb_upper: Option<String>,
    pub bb_middle: Option<String>,
    pub bb_lower: Option<String>,
    pub ema_fast: Option<String>,
    pub ema_medium: Option<String>,
    pub ema_slow: Option<String>,
    pub ema_long: Option<String>,
    pub average_volume: Option<String>,
}

pub async fn query_indicator_snapshots(
    pool: &SqlitePool,
    symbol: &str,
    timeframe_secs: u64,
    limit: u32,
) -> Vec<IndicatorSnapshotRow> {
    sqlx::query_as::<_, IndicatorSnapshotRow>(
        "SELECT timestamp, rsi_14, squeeze_on, squeeze_momentum,
                macd_line, macd_signal, macd_hist,
                adx_14, adx_plus, adx_minus,
                atr_14, bb_upper, bb_middle, bb_lower,
                ema_fast, ema_medium, ema_slow, ema_long,
                average_volume
         FROM market_snapshots
         WHERE symbol = ?1
           AND timeframe_secs = ?2
           AND close IS NOT NULL
         ORDER BY timestamp ASC
         LIMIT ?3",
    )
    .bind(symbol)
    .bind(timeframe_secs as i64)
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!("Database Error: Failed to query indicator snapshots: {}", e);
        vec![]
    })
}

pub async fn query_atr_snapshots(
    pool: &SqlitePool,
    timeframe_secs: u64,
    limit: u32,
) -> Vec<Option<String>> {
    let rows = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT atr_14 FROM market_snapshots
         WHERE atr_14 IS NOT NULL AND timeframe_secs = ?1
         ORDER BY id DESC
         LIMIT ?2",
    )
    .bind(timeframe_secs as i64)
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await;

    match rows {
        Ok(rows) => rows.into_iter().map(|(atr,)| atr).collect(),
        Err(e) => {
            eprintln!("Database Error: Failed to query ATR snapshots: {}", e);
            vec![]
        }
    }
}

pub async fn query_latest_snapshot(
    pool: &SqlitePool,
    symbol: &str,
    timeframe_secs: u64,
) -> Option<MarketSnapshot> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT exchange, timestamp, symbol, mid_price, bid_price, ask_price,
                open, high, low, close, volume, average_volume,
                bb_upper, bb_middle, bb_lower, atr_14, vwap,
                ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
                macd_line, macd_signal, macd_hist, adx_14, adx_plus, adx_minus,
                squeeze_on, squeeze_momentum
         FROM market_snapshots
         WHERE symbol = ?1 AND timeframe_secs = ?2 AND close IS NOT NULL
         ORDER BY id DESC
         LIMIT 1",
    )
    .bind(symbol)
    .bind(timeframe_secs as i64)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    row.map(|r| {
        let parse_dec =
            |val: Option<String>| val.and_then(|s| rust_decimal::Decimal::from_str_exact(&s).ok());
        MarketSnapshot {
            exchange: Some(shared::normalized::Exchange::Hyperliquid),
            timeframe_secs,
            timestamp: r.get::<i64, _>(1) as u64,
            symbol: r.get(2),
            is_completed: Some(true),
            mid_price: parse_dec(Some(r.get::<String, _>(3)))
                .unwrap_or(rust_decimal::Decimal::ZERO),
            bid_price: parse_dec(Some(r.get::<String, _>(4)))
                .unwrap_or(rust_decimal::Decimal::ZERO),
            ask_price: parse_dec(Some(r.get::<String, _>(5)))
                .unwrap_or(rust_decimal::Decimal::ZERO),
            bid_size: None,
            ask_size: None,
            funding_rate: None,
            open: parse_dec(r.get::<Option<String>, _>(6)),
            high: parse_dec(r.get::<Option<String>, _>(7)),
            low: parse_dec(r.get::<Option<String>, _>(8)),
            close: parse_dec(r.get::<Option<String>, _>(9)),
            volume: parse_dec(r.get::<Option<String>, _>(10)),
            average_volume: parse_dec(r.get::<Option<String>, _>(11)),
            rvol: None,
            bb_upper: parse_dec(r.get::<Option<String>, _>(12)),
            bb_middle: parse_dec(r.get::<Option<String>, _>(13)),
            bb_lower: parse_dec(r.get::<Option<String>, _>(14)),
            atr_14: parse_dec(r.get::<Option<String>, _>(15)),
            atr_slope: None,
            atr_volatility_regime: None,
            atr_stop_loss_level: None,
            atr_take_profit_level: None,
            vwap: parse_dec(r.get::<Option<String>, _>(16)),
            vwap_bias: None,
            ema_fast: parse_dec(r.get::<Option<String>, _>(17)),
            ema_medium: parse_dec(r.get::<Option<String>, _>(18)),
            ema_slow: parse_dec(r.get::<Option<String>, _>(19)),
            ema_long: parse_dec(r.get::<Option<String>, _>(20)),
            ema_stack_state: None,
            rsi_14: parse_dec(r.get::<Option<String>, _>(21)),
            macd_line: parse_dec(r.get::<Option<String>, _>(22)),
            macd_signal: parse_dec(r.get::<Option<String>, _>(23)),
            macd_hist: parse_dec(r.get::<Option<String>, _>(24)),
            adx_14: parse_dec(r.get::<Option<String>, _>(25)),
            adx_plus: parse_dec(r.get::<Option<String>, _>(26)),
            adx_minus: parse_dec(r.get::<Option<String>, _>(27)),
            squeeze_on: r.get::<Option<i32>, _>(28).map(|v| v != 0),
            squeeze_momentum: parse_dec(r.get::<Option<String>, _>(29)),
            squeeze_duration: None,
            squeeze_release_trigger: None,
            squeeze_momentum_direction: None,
            bbwp: parse_dec(r.get::<Option<String>, _>(30)),
            support_levels: r.get::<Option<String>, _>(31),
            resistance_levels: r.get::<Option<String>, _>(32),
            sr_flip_events: None,
            chart_pattern: None,
            chart_pattern_confidence: None,
            fib_golden_pocket_low: None,
            fib_golden_pocket_high: None,
            fib_extension_1618: None,
            fib_extension_2618: None,
            swing_high: None,
            swing_low: None,
            rsi_divergence_status: None,
            rsi_divergence_coords: None,
            macd_divergence_status: None,
            macd_divergence_coords: None,
            macd_histogram_peak: None,
            macd_trend_state: None,
            macd_crossover_detected: None,
            macd_crossover_direction: None,
            adx_slope: None,
            adx_peak: None,
            adx_regime: None,
            adx_di_crossover_detected: None,
            adx_di_crossover_direction: None,
        }
    })
}

pub async fn query_closest_close_price(
    pool: &SqlitePool,
    symbol: &str,
    timeframe_secs: u64,
    target_timestamp_secs: u64,
) -> Option<f64> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT close FROM market_snapshots
         WHERE symbol = ?1 AND timeframe_secs = ?2 AND close IS NOT NULL AND timestamp >= ?3
         ORDER BY timestamp ASC LIMIT 1",
    )
    .bind(symbol)
    .bind(timeframe_secs as i64)
    .bind(target_timestamp_secs as i64)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => {
            let s: String = r.get(0);
            s.parse::<f64>().ok()
        }
        None => {
            let fallback = sqlx::query(
                "SELECT close FROM market_snapshots
                 WHERE symbol = ?1 AND timeframe_secs = ?2 AND close IS NOT NULL AND timestamp <= ?3
                 ORDER BY timestamp DESC LIMIT 1",
            )
            .bind(symbol)
            .bind(timeframe_secs as i64)
            .bind(target_timestamp_secs as i64)
            .fetch_optional(&*pool)
            .await
            .ok()
            .flatten();
            fallback.and_then(|r: sqlx::sqlite::SqliteRow| {
                let s: String = r.get(0);
                s.parse::<f64>().ok()
            })
        }
    }
}
