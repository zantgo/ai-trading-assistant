use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::llm::{IndividualIndicatorResult, LlmClient, MultiAgentResults};
use crate::server::telemetry::DeterministicTelemetry;
use crate::server::types::IndicatorSnapshot;

async fn run_indicator_agent_with_timeout(
    client: LlmClient,
    name: String,
    system: String,
    context: String,
    timeout_secs: u64,
    name_prefix: Option<String>,
    pair_key: Option<String>,
) -> IndividualIndicatorResult {
    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        client.run_indicator_agent(&name, &system, &context, pair_key.as_deref()),
    )
    .await
    {
        Ok(Ok(result)) => {
            let indicator_name = name_prefix
                .as_ref()
                .map(|p| format!("{}-{}", p, result.indicator_name))
                .unwrap_or(result.indicator_name);
            IndividualIndicatorResult {
                indicator_name,
                signal: result.signal,
                reason: result.reason,
                confidence_score: result.confidence_score,
                divergence_status: result.divergence_status,
                divergence_type: result.divergence_type,
                is_confirmed: result.is_confirmed,
            }
        }
        Ok(Err(e)) => {
            let indicator_name = name_prefix
                .as_ref()
                .map(|p| format!("{}-{}", p, name))
                .unwrap_or(name);
            IndividualIndicatorResult {
                indicator_name,
                signal: "UNAVAILABLE".to_string(),
                reason: format!("Agent error: {}", e),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            }
        }
        Err(_) => {
            let indicator_name = name_prefix
                .as_ref()
                .map(|p| format!("{}-{}", p, name))
                .unwrap_or(name);
            IndividualIndicatorResult {
                indicator_name,
                signal: "UNAVAILABLE".to_string(),
                reason: format!("Agent timed out after {} seconds", timeout_secs),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            }
        }
    }
}

async fn join_agent_results(
    handles: Vec<tokio::task::JoinHandle<IndividualIndicatorResult>>,
) -> Vec<IndividualIndicatorResult> {
    futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| {
            r.unwrap_or_else(|e| IndividualIndicatorResult {
                indicator_name: "UNKNOWN".to_string(),
                signal: "UNAVAILABLE".to_string(),
                reason: format!("Task panic: {}", e),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            })
        })
        .collect()
}

pub async fn run_phase_one_agents_mtf(
    client: &LlmClient,
    symbol: &str,
    micro: &IndicatorSnapshot,
    fast: &IndicatorSnapshot,
    slow: &IndicatorSnapshot,
    macro_snap: &IndicatorSnapshot,
    _prices: &[f64],
    master_id: i64,
    telemetry_tx: &mpsc::Sender<crate::db::TelemetryMsg>,
) -> Vec<IndividualIndicatorResult> {
    let rsi_section = client.get_guide_section("RSI").await;
    let macd_section = client.get_guide_section("MACD").await;
    let squeeze_section = client.get_guide_section("SQUEEZE").await;
    let adx_section = client.get_guide_section("ADX").await;
    let bb_atr_section = client.get_guide_section("BOLLINGER_ATR").await;
    let vol_ema_section = client.get_guide_section("VOLUME_EMA").await;
    let vwap_section = client.get_guide_section("VWAP").await;

    let indicator_names = [
        "RSI",
        "MACD",
        "SQUEEZE",
        "ADX",
        "BOLLINGER_ATR",
        "VOLUME_EMA",
        "VWAP",
    ];
    let sections = [
        &rsi_section,
        &macd_section,
        &squeeze_section,
        &adx_section,
        &bb_atr_section,
        &vol_ema_section,
        &vwap_section,
    ];
    let slow_tf_secs = 300u64;
    let macro_tf_secs = 900u64;
    let timeframes: [(&str, &IndicatorSnapshot, u64); 4] = [
        ("micro", micro, 60),
        ("fast", fast, 180),
        ("slow", slow, slow_tf_secs),
        ("macro", macro_snap, macro_tf_secs),
    ];

    let pair_key = symbol.to_string();
    let handles: Vec<_> = timeframes
        .iter()
        .flat_map(|(tf_label, indicator_snap, _tf_secs)| {
            let pk = pair_key.clone();
            (0..7).map(move |i| {
                tokio::spawn(run_indicator_agent_with_timeout(
                    client.clone(),
                    indicator_names[i].to_string(),
                    sections[i].to_string(),
                    build_indicator_context(indicator_names[i], indicator_snap),
                    10,
                    Some((*tf_label).to_string()),
                    Some(pk.clone()),
                ))
            })
        })
        .collect();

    let results = join_agent_results(handles).await;

    for (tf_label, _, tf_secs) in &timeframes {
        for result in &results {
            if result.indicator_name.starts_with(&format!("{}-", tf_label)) {
                let _ = telemetry_tx
                    .send(crate::db::TelemetryMsg::InsertIndividualLog {
                        master_record_id: master_id,
                        indicator_name: result.indicator_name.clone(),
                        signal: result.signal.clone(),
                        reason: result.reason.clone(),
                        timeframe_secs: *tf_secs,
                    })
                    .await;
            }
        }
    }

    results
}

fn build_indicator_context(indicator_name: &str, snap: &IndicatorSnapshot) -> String {
    match indicator_name {
        "RSI" => format!(
            r#"{{ "rsi_value": {}, "current_price": {}, "rsi_divergence_status": "{}" }}"#,
            snap.rsi.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.current_price
                .map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.rsi_divergence_status.as_deref().unwrap_or("none"),
        ),
        "MACD" => format!(
            r#"{{ "macd_line": {}, "signal_line": {}, "histogram_value": {}, "histogram_trend": "{}", "histogram_peak": {}, "crossover_detected": {}, "crossover_direction": "{}", "macd_divergence_status": "{}" }}"#,
            snap.macd_line
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_signal
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_histogram
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_trend_state.as_deref().unwrap_or("unknown"),
            snap.macd_histogram_peak
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.macd_crossover_detected.unwrap_or(false),
            snap.macd_crossover_direction.as_deref().unwrap_or("NONE"),
            snap.macd_divergence_status.as_deref().unwrap_or("none"),
        ),
        "SQUEEZE" => format!(
            r#"{{ "squeeze_on": {}, "momentum_value": {}, "squeeze_duration": {}, "squeeze_release_trigger": {}, "momentum_direction": "{}" }}"#,
            snap.squeeze_on
                .map_or("null".to_string(), |v| v.to_string()),
            snap.squeeze_momentum
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.squeeze_duration.unwrap_or(0),
            snap.squeeze_release_trigger.unwrap_or(false),
            snap.squeeze_momentum_direction.as_deref().unwrap_or("Flat"),
        ),
        "ADX" => format!(
            r#"{{ "adx_line": {}, "di_plus": {}, "di_minus": {}, "adx_slope": {}, "adx_regime": "{}", "di_crossover_detected": {}, "di_crossover_direction": "{}" }}"#,
            snap.adx.map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_plus
                .map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_minus
                .map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.adx_slope
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.adx_regime.as_deref().unwrap_or("unknown"),
            snap.adx_di_crossover_detected.unwrap_or(false),
            snap.adx_di_crossover_direction.as_deref().unwrap_or("NONE"),
        ),
        "BOLLINGER_ATR" => format!(
            r#"{{ "mid_price": {}, "bb_upper": {}, "bb_middle": {}, "bb_lower": {}, "atr_value": {} }}"#,
            snap.current_price
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.bb_upper
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.bb_middle
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.bb_lower
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.atr.map_or("null".to_string(), |v| format!("{:.4}", v)),
        ),
        "VOLUME_EMA" => format!(
            r#"{{ "close": {}, "ema_fast": {}, "ema_slow": {}, "volume": {}, "average_volume": {}, "rvol": {}, "ema_stack_state": "{}" }}"#,
            snap.current_price
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.ema_fast
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.ema_slow
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.volume
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.average_volume
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.rvol
                .map_or("null".to_string(), |v| format!("{:.2}", v)),
            snap.ema_stack_state.as_deref().unwrap_or("tangled"),
        ),
        "VWAP" => format!(
            r#"{{ "close": {}, "vwap": {}, "vwap_bias": "{}" }}"#,
            snap.current_price
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.vwap
                .map_or("null".to_string(), |v| format!("{:.4}", v)),
            snap.vwap_bias.as_deref().unwrap_or("equilibrium"),
        ),
        _ => "{}".to_string(),
    }
}

pub async fn run_phase_one_agents(
    client: &LlmClient,
    symbol: &str,
    indicators: &IndicatorSnapshot,
    prices: &[f64],
    atr_trend: &str,
    master_id: i64,
    telemetry_tx: &mpsc::Sender<crate::db::TelemetryMsg>,
) -> Vec<IndividualIndicatorResult> {
    let rsi_section = client.get_guide_section("RSI").await;
    let macd_section = client.get_guide_section("MACD").await;
    let squeeze_section = client.get_guide_section("SQUEEZE").await;
    let adx_section = client.get_guide_section("ADX").await;
    let bb_atr_section = client.get_guide_section("BOLLINGER_ATR").await;
    let vol_ema_section = client.get_guide_section("VOLUME_EMA").await;
    let vwap_section = client.get_guide_section("VWAP").await;

    let recent_closes_json =
        serde_json::to_string(&prices.iter().rev().take(10).rev().collect::<Vec<_>>())
            .unwrap_or_else(|_| "[]".into());

    let rsi_context = format!(
        r#"{{ "rsi_value": {}, "recent_closes": {} }}"#,
        indicators
            .rsi
            .map_or("null".to_string(), |v| format!("{:.2}", v)),
        recent_closes_json,
    );

    let macd_hist_trend = compute_histogram_trend(prices, indicators.macd_histogram);
    let macd_context = format!(
        r#"{{ "macd_line": {}, "signal_line": {}, "histogram_value": {}, "histogram_trend": "{}" }}"#,
        indicators
            .macd_line
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .macd_signal
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .macd_histogram
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        macd_hist_trend,
    );

    let mom_trend = compute_squeeze_momentum_trend(indicators.squeeze_momentum);
    let squeeze_context = format!(
        r#"{{ "squeeze_on": {}, "momentum_value": {}, "momentum_trend": "{}" }}"#,
        indicators
            .squeeze_on
            .map_or("null".to_string(), |v| v.to_string()),
        indicators
            .squeeze_momentum
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        mom_trend,
    );

    let adx_context = format!(
        r#"{{ "adx_line": {}, "di_plus": {}, "di_minus": {} }}"#,
        indicators
            .adx
            .map_or("null".to_string(), |v| format!("{:.2}", v)),
        indicators
            .adx_plus
            .map_or("null".to_string(), |v| format!("{:.2}", v)),
        indicators
            .adx_minus
            .map_or("null".to_string(), |v| format!("{:.2}", v)),
    );

    let bb_atr_context = format!(
        r#"{{ "mid_price": {}, "bb_upper": {}, "bb_middle": {}, "bb_lower": {}, "atr_value": {}, "atr_trend": "{}" }}"#,
        indicators
            .current_price
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .bb_upper
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .bb_middle
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .bb_lower
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .atr
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        atr_trend,
    );

    let vol_ema_context = format!(
        r#"{{ "close": {}, "ema_fast": {}, "ema_medium": {}, "ema_slow": {}, "ema_long": {}, "volume": {}, "average_volume": {}, "rvol": {}, "ema_stack_state": "{}" }}"#,
        indicators
            .current_price
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .ema_fast
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .ema_medium
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .ema_slow
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .ema_long
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .volume
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .average_volume
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .rvol
            .map_or("null".to_string(), |v| format!("{:.2}", v)),
        indicators.ema_stack_state.as_deref().unwrap_or("tangled"),
    );

    let vwap_context = format!(
        r#"{{ "close": {}, "vwap": {}, "vwap_bias": "{}" }}"#,
        indicators
            .current_price
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators
            .vwap
            .map_or("null".to_string(), |v| format!("{:.4}", v)),
        indicators.vwap_bias.as_deref().unwrap_or("equilibrium"),
    );

    let agents = vec![
        ("RSI", rsi_section, rsi_context),
        ("MACD", macd_section, macd_context),
        ("SQUEEZE", squeeze_section, squeeze_context),
        ("ADX", adx_section, adx_context),
        ("BOLLINGER_ATR", bb_atr_section, bb_atr_context),
        ("VOLUME_EMA", vol_ema_section, vol_ema_context),
        ("VWAP", vwap_section, vwap_context),
    ];

    let pair_key = symbol.to_string();
    let handles: Vec<_> = agents
        .into_iter()
        .map(|(name, section, context)| {
            tokio::spawn(run_indicator_agent_with_timeout(
                client.clone(),
                name.to_string(),
                section,
                context,
                10,
                None,
                Some(pair_key.clone()),
            ))
        })
        .collect();

    let results = join_agent_results(handles).await;

    for result in &results {
        let _ = telemetry_tx
            .send(crate::db::TelemetryMsg::InsertIndividualLog {
                master_record_id: master_id,
                indicator_name: result.indicator_name.clone(),
                signal: result.signal.clone(),
                reason: result.reason.clone(),
                timeframe_secs: 60,
            })
            .await;
    }

    results
}

fn compute_histogram_trend(_prices: &[f64], current_hist: Option<f64>) -> String {
    match current_hist {
        Some(v) if v > 0.0 => "increasing".to_string(),
        Some(v) if v < 0.0 => "decreasing".to_string(),
        _ => "flat".to_string(),
    }
}

fn compute_squeeze_momentum_trend(momentum: Option<f64>) -> String {
    match momentum {
        Some(v) if v > 0.0 => "rising".to_string(),
        Some(v) if v < 0.0 => "falling".to_string(),
        _ => "flat".to_string(),
    }
}

pub async fn run_multi_agent_pipeline(
    client: Arc<LlmClient>,
    pool: SqlitePool,
    symbol: &str,
    micro: &IndicatorSnapshot,
    _fast: &IndicatorSnapshot,
    _slow: &IndicatorSnapshot,
    _macro: &IndicatorSnapshot,
    prices: &[f64],
    master_id: i64,
    telemetry: &DeterministicTelemetry,
) -> Result<MultiAgentResults, String> {
    let prices_json = serde_json::to_string(&prices).unwrap_or_default();
    let pair_key = symbol.to_string();

    let context_trend = format!(
        r#"{{ "close": {}, "ema_stack_state": "{}", "deterministic_eight_factor_score": {}, "slow_trend_regime": "{}" }}"#,
        micro.current_price.unwrap_or(0.0),
        micro.ema_stack_state.as_deref().unwrap_or("tangled"),
        telemetry.total_confluence_score,
        telemetry.market_regime
    );

    let context_volatility = format!(
        r#"{{ "market_regime": "{}", "bbwp": {}, "atr": {}, "squeeze_on": {}, "rvol": {} }}"#,
        telemetry.market_regime,
        telemetry.bbwp_percentile,
        micro.atr.unwrap_or(0.0),
        telemetry.squeeze_on,
        telemetry.rvol
    );

    let context_structure = format!(
        r#"{{ "current_price": {}, "prices": {}, "squeeze_momentum_direction": "{}" }}"#,
        micro.current_price.unwrap_or(0.0),
        prices_json,
        micro
            .squeeze_momentum_direction
            .as_deref()
            .unwrap_or("Flat")
    );

    let context_risk = format!(r#"{{ "leverage": 20, "max_risk_pct": 2.0 }}"#);

    let context_position = format!(
        r#"{{ "current_price": {} }}"#,
        micro.current_price.unwrap_or(0.0)
    );

    let p_key_trend = pair_key.clone();
    let trend_ctx = context_trend.clone();
    let trend_client = client.clone();
    let h_trend = tokio::spawn(async move {
        trend_client
            .run_domain_agent::<crate::llm::TrendAgentData>(
                "Trend",
                crate::llm::TREND_AGENT_PROMPT,
                &trend_ctx,
                Some(&p_key_trend),
            )
            .await
    });

    let p_key_vol = pair_key.clone();
    let vol_ctx = context_volatility.clone();
    let vol_client = client.clone();
    let h_vol = tokio::spawn(async move {
        vol_client
            .run_domain_agent::<crate::llm::VolatilityAgentData>(
                "Volatility",
                crate::llm::VOLATILITY_AGENT_PROMPT,
                &vol_ctx,
                Some(&p_key_vol),
            )
            .await
    });

    let p_key_struct = pair_key.clone();
    let struct_ctx = context_structure.clone();
    let struct_client = client.clone();
    let h_struct = tokio::spawn(async move {
        struct_client
            .run_domain_agent::<crate::llm::StructureAgentData>(
                "Structure",
                crate::llm::STRUCTURE_AGENT_PROMPT,
                &struct_ctx,
                Some(&p_key_struct),
            )
            .await
    });

    let p_key_risk = pair_key.clone();
    let risk_ctx = context_risk.clone();
    let risk_client = client.clone();
    let h_risk = tokio::spawn(async move {
        risk_client
            .run_domain_agent::<crate::llm::RiskAgentData>(
                "Risk",
                crate::llm::RISK_AGENT_PROMPT,
                &risk_ctx,
                Some(&p_key_risk),
            )
            .await
    });

    let p_key_pos = pair_key.clone();
    let pos_ctx = context_position.clone();
    let pos_client = client.clone();
    let h_pos = tokio::spawn(async move {
        pos_client
            .run_domain_agent::<crate::llm::PositionAgentData>(
                "Position",
                crate::llm::POSITION_AGENT_PROMPT,
                &pos_ctx,
                Some(&p_key_pos),
            )
            .await
    });

    let r_trend = h_trend
        .await
        .map_err(|e| format!("Task join error: {}", e))??;
    let r_vol = h_vol
        .await
        .map_err(|e| format!("Task join error: {}", e))??;
    let r_struct = h_struct
        .await
        .map_err(|e| format!("Task join error: {}", e))??;
    let r_risk = h_risk
        .await
        .map_err(|e| format!("Task join error: {}", e))??;
    let r_pos = h_pos
        .await
        .map_err(|e| format!("Task join error: {}", e))??;

    crate::db::insert_agent_thought_log(
        &pool,
        master_id,
        "Trend",
        &r_trend.thought,
        &serde_json::to_string(&r_trend.data).unwrap_or_default(),
        r_trend.data.confidence_score,
    )
    .await;
    crate::db::insert_agent_thought_log(
        &pool,
        master_id,
        "Volatility",
        &r_vol.thought,
        &serde_json::to_string(&r_vol.data).unwrap_or_default(),
        r_vol.data.volatility_score,
    )
    .await;
    crate::db::insert_agent_thought_log(
        &pool,
        master_id,
        "Structure",
        &r_struct.thought,
        &serde_json::to_string(&r_struct.data).unwrap_or_default(),
        r_struct.data.structural_score,
    )
    .await;
    crate::db::insert_agent_thought_log(
        &pool,
        master_id,
        "Risk",
        &r_risk.thought,
        &serde_json::to_string(&r_risk.data).unwrap_or_default(),
        r_risk.data.exposure_score,
    )
    .await;
    crate::db::insert_agent_thought_log(
        &pool,
        master_id,
        "Position",
        &r_pos.thought,
        &serde_json::to_string(&r_pos.data).unwrap_or_default(),
        100,
    )
    .await;

    Ok(MultiAgentResults {
        trend: r_trend,
        volatility: r_vol,
        structure: r_struct,
        risk: r_risk,
        position: r_pos,
    })
}
