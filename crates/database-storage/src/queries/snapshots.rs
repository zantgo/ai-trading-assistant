use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use core_domain::models::MarketSnapshot;
use core_domain::normalized::{Exchange, NormalizedCandle};
use sqlx::SqlitePool;

pub async fn insert_snapshot_internal(pool: &SqlitePool, snapshot: &MarketSnapshot) {
    let sqz_on_db_val = snapshot.squeeze_on().map(|s| if s { 1 } else { 0 });
    let exchange_label = snapshot
        .exchange
        .as_ref()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "Hyperliquid".to_string());

    // Normalized [-1.0,1.0] + state label for the 8 primary scored indicators.
    let norm = |k: &str| snapshot.ind_norm(k);
    let label = |k: &str| snapshot.ind_label(k).map(|s| s.to_string());

    // Full indicator map serialized as the auxiliary catch-all JSON blob.
    let auxiliary_json = serde_json::to_string(&snapshot.indicators).ok();

    if let Err(e) = sqlx::query(
        "INSERT INTO market_snapshots (
            exchange, timeframe_secs, timestamp, symbol, mid_price, bid_price, ask_price,
            open, high, low, close, volume, average_volume,
            bb_upper, bb_middle, bb_lower, atr_14, vwap,
            ema_fast, ema_medium, ema_slow, ema_long, rsi_14,
            macd_line, macd_signal, macd_hist, adx_14, adx_plus, adx_minus,
            squeeze_on, squeeze_momentum, bbwp, support_levels, resistance_levels,
            rsi_normalized, rsi_state_label, macd_normalized, macd_state_label,
            squeeze_normalized, squeeze_state_label, adx_normalized, adx_state_label,
            bbwp_normalized, bbwp_state_label, rvol_normalized, rvol_state_label,
            ema_stack_normalized, ema_stack_state_label, vwap_normalized, vwap_state_label,
            fib_GP_top, fib_GP_bottom, fib_ext_1618, fib_ext_2618,
            stoch_k_normalized, stoch_k_state_label, stoch_d_normalized, stoch_d_state_label,
            chandemo_normalized, chandemo_state_label,
            supertrend_normalized, supertrend_state_label, keltner_normalized, keltner_state_label,
            donchian_normalized, donchian_state_label, obv_normalized, obv_state_label,
            cmf_normalized, cmf_state_label, mfi_normalized, mfi_state_label,
            hv_normalized, hv_state_label,
            aroon_normalized, aroon_state_label, choppiness_normalized, choppiness_state_label,
            linreg_slope_normalized, linreg_slope_state_label, zscore_normalized, zscore_state_label,
            auxiliary_normalized_data
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, ?81, ?82, ?83)"
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
    .bind(snapshot.bb_upper().map(|d| d.to_string()))
    .bind(snapshot.bb_middle().map(|d| d.to_string()))
    .bind(snapshot.bb_lower().map(|d| d.to_string()))
    .bind(snapshot.atr_14().map(|d| d.to_string()))
    .bind(snapshot.vwap().map(|d| d.to_string()))
    .bind(snapshot.ema_fast().map(|d| d.to_string()))
    .bind(snapshot.ema_medium().map(|d| d.to_string()))
    .bind(snapshot.ema_slow().map(|d| d.to_string()))
    .bind(snapshot.ema_long().map(|d| d.to_string()))
    .bind(snapshot.rsi_14().map(|d| d.to_string()))
    .bind(snapshot.macd_line().map(|d| d.to_string()))
    .bind(snapshot.macd_signal().map(|d| d.to_string()))
    .bind(snapshot.macd_hist().map(|d| d.to_string()))
    .bind(snapshot.adx_14().map(|d| d.to_string()))
    .bind(snapshot.adx_plus().map(|d| d.to_string()))
    .bind(snapshot.adx_minus().map(|d| d.to_string()))
    .bind(sqz_on_db_val)
    .bind(snapshot.squeeze_momentum().map(|d| d.to_string()))
    .bind(snapshot.bbwp().map(|d| d.to_string()))
    .bind(Option::<String>::None)
    .bind(Option::<String>::None)
    .bind(norm("rsi"))
    .bind(label("rsi"))
    .bind(norm("macd"))
    .bind(label("macd"))
    .bind(norm("squeeze"))
    .bind(label("squeeze"))
    .bind(norm("adx"))
    .bind(label("adx"))
    .bind(norm("bbwp"))
    .bind(label("bbwp"))
    .bind(norm("rvol"))
    .bind(label("rvol"))
    .bind(norm("ema_stack"))
    .bind(label("ema_stack"))
    .bind(norm("vwap"))
    .bind(label("vwap"))
    .bind(snapshot.fib_gp_top())
    .bind(snapshot.fib_gp_bottom())
    .bind(snapshot.fib_ext_1618())
    .bind(snapshot.fib_ext_2618())
    .bind(norm("stochastic"))
    .bind(label("stochastic"))
    .bind(norm("stochastic"))
    .bind(label("stochastic"))
    .bind(norm("chandemo"))
    .bind(label("chandemo"))
    .bind(norm("supertrend"))
    .bind(label("supertrend"))
    .bind(norm("keltner"))
    .bind(label("keltner"))
    .bind(norm("donchian"))
    .bind(label("donchian"))
    .bind(norm("obv"))
    .bind(label("obv"))
    .bind(norm("cmf"))
    .bind(label("cmf"))
    .bind(norm("mfi"))
    .bind(label("mfi"))
    .bind(norm("hv"))
    .bind(label("hv"))
    .bind(norm("aroon"))
    .bind(label("aroon"))
    .bind(norm("choppiness"))
    .bind(label("choppiness"))
    .bind(norm("linreg_slope"))
    .bind(label("linreg_slope"))
    .bind(norm("zscore"))
    .bind(label("zscore"))
    .bind(auxiliary_json)
    .execute(pool)
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
    pub adx_plus_di: Option<String>,
    pub adx_minus_di: Option<String>,
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
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!("Database Error: Failed to query indicator snapshots: {}", e);
        vec![]
    })
}

/// Reconstruct recent completed OHLCV candles for a pair + timeframe from the
/// persisted `market_snapshots` table. Returns candles in ascending timestamp
/// order (oldest first). Used to pre-warm indicator pipelines from local data
/// before falling back to REST for the remaining "gap" up to the present.
pub async fn query_recent_candles(
    pool: &SqlitePool,
    symbol: &str,
    timeframe_secs: u64,
    limit: u32,
) -> Vec<NormalizedCandle> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT exchange, timestamp, open, high, low, close, volume
         FROM market_snapshots
         WHERE symbol = ?1
           AND timeframe_secs = ?2
           AND close IS NOT NULL
         ORDER BY timestamp DESC
         LIMIT ?3",
    )
    .bind(symbol)
    .bind(timeframe_secs as i64)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!("Database Error: Failed to query recent candles: {}", e);
        vec![]
    });

    let parse = |s: Option<String>| {
        s.and_then(|v| Decimal::from_str_exact(&v).ok())
            .unwrap_or(Decimal::ZERO)
    };

    let mut candles: Vec<NormalizedCandle> = rows
        .into_iter()
        .map(|(exchange_str, ts, open, high, low, close, volume)| {
            let exchange = match exchange_str.as_str() {
                "Bitget" => Exchange::Bitget,
                _ => Exchange::Hyperliquid,
            };
            let close_dec = parse(close);
            let non_zero = |d: Decimal| if d.is_zero() { close_dec } else { d };
            NormalizedCandle {
                exchange,
                symbol: symbol.to_string(),
                start_time_ms: (ts.max(0) as u64) * 1000,
                duration_ms: timeframe_secs * 1000,
                open: non_zero(parse(open)),
                high: non_zero(parse(high)),
                low: non_zero(parse(low)),
                close: close_dec,
                volume: parse(volume),
                trades_count: 0,
                reconstructed: None,
            }
        })
        .collect();

    // Query returned newest-first; reverse to ascending (oldest-first).
    candles.reverse();
    candles
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
    .fetch_all(pool)
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
                squeeze_on, squeeze_momentum, bbwp, support_levels, resistance_levels,
                auxiliary_normalized_data
         FROM market_snapshots
         WHERE symbol = ?1 AND timeframe_secs = ?2 AND close IS NOT NULL
         ORDER BY id DESC
         LIMIT 1",
    )
    .bind(symbol)
    .bind(timeframe_secs as i64)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    row.map(|r| {
        let parse_dec =
            |val: Option<String>| val.and_then(|s| rust_decimal::Decimal::from_str_exact(&s).ok());
        let f = |i: usize| -> Option<f64> {
            r.get::<Option<String>, _>(i)
                .and_then(|s| s.parse::<f64>().ok())
        };
        let close = parse_dec(r.get::<Option<String>, _>(9));

        // Prefer the authoritative auxiliary JSON map; fall back to scalar
        // reconstruction for legacy rows predating this migration.
        let aux_json = r.get::<Option<String>, _>(33);
        let indicators = aux_json
            .as_deref()
            .and_then(|s| {
                serde_json::from_str::<
                    std::collections::HashMap<String, core_domain::indicator_dtos::NormalizedIndicatorValue>,
                >(s)
                .ok()
            })
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| {
                crate::analyzer_normalize_fallback::build_indicator_map_from_scalars(
                    crate::analyzer_normalize_fallback::RawScalarInputs {
                        close: close.and_then(|d| d.to_f64()).unwrap_or(0.0),
                        rsi: f(21).unwrap_or(0.0),
                        macd_line: f(22).unwrap_or(0.0),
                        macd_signal: f(23).unwrap_or(0.0),
                        macd_hist: f(24).unwrap_or(0.0),
                        adx: f(25).unwrap_or(0.0),
                        adx_plus_di: f(26).unwrap_or(0.0),
                        adx_minus_di: f(27).unwrap_or(0.0),
                        bbwp: f(30).unwrap_or(0.0),
                        squeeze: 0.0,
                        atr: f(15).unwrap_or(0.0),
                        vwap: f(16).unwrap_or(0.0),
                        ema_fast: f(17).unwrap_or(0.0),
                        ema_medium: f(18).unwrap_or(0.0),
                        ema_slow: f(19).unwrap_or(0.0),
                        ema_long: f(20).unwrap_or(0.0),
                        rvol: 1.0,
                        stoch_k: 50.0,
                        stoch_d: 50.0,
                        chandemo: 0.0,
                        obv: 0.0,
                        cmf: 0.0,
                        mfi: 50.0,
                        hv: 0.0,
                        aroon_up: 50.0,
                        aroon_down: 50.0,
                        choppiness: 50.0,
                    },
                )
            });

        MarketSnapshot {
            timeframe_slot: Some(core_domain::models::TimeframeSlot::parse_from_secs(timeframe_secs)),
            exchange: Some(core_domain::normalized::Exchange::Hyperliquid),
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
            open_interest: None,
            oi_delta_1h: None,
            mark_price: None,
            index_price: None,
            mark_index_spread_pct: None,
            prev_day_px: None,
            open: parse_dec(r.get::<Option<String>, _>(6)),
            high: parse_dec(r.get::<Option<String>, _>(7)),
            low: parse_dec(r.get::<Option<String>, _>(8)),
            close,
            volume: parse_dec(r.get::<Option<String>, _>(10)),
            average_volume: parse_dec(r.get::<Option<String>, _>(11)),
            context: None,
            alignment: None,
            risk: None,
            analysis: None,
            advisory: None,
            decision_context: None,
            statistical_context: None,
            indicators,
            risk_profile: None,
            liquidity: None,
            cluster: None,
            liquidity_signals: vec![],
            metrics_config: None,
            opportunity: None,
            quality_envelope: None,
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
    .fetch_optional(pool)
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
            .fetch_optional(pool)
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
