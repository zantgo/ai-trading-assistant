use crate::profile_evaluation::{
    calculate_registry_confluence, calculate_registry_opposite_score, evaluate_mtf_alignment,
    indicator_to_snapshot_values, SnapshotValues, REGISTRY_OPPOSITE_EXIT_THRESHOLD,
};
use crate::server::helpers::{default_pair_key, get_active_pair};
use crate::server::types::{
    ActiveTradeDto, ActiveTradesResponse, BreakEvenTrailDto, ContributionDto, ExitSignalsDto,
    HistoryQuery, MonitorResponse, MonitorTimeframe, MtfConfirmation, MtfIndicatorRow,
    SafetyStateDto,
};
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use shared::indicators::registry::INDICATORS;
use shared::models::MarketSnapshot;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn snap_values(s: &Option<MarketSnapshot>) -> Option<SnapshotValues> {
    s.as_ref().map(|m| {
        let price = m.mid_price.to_string().parse::<f64>().unwrap_or(0.0);
        indicator_to_snapshot_values(&m.indicators, price)
    })
}

fn dir_bucket(sv: &SnapshotValues, key: &str) -> i8 {
    if !sv.indicators.contains_key(key) {
        return 0;
    }
    let n = sv.norm(key);
    if n > 0.10 {
        1
    } else if n < -0.10 {
        -1
    } else {
        0
    }
}

fn tf_summary(label: &str, secs: u64, snap: &Option<MarketSnapshot>, sv: &Option<SnapshotValues>, regime_multipliers: Option<&HashMap<String, HashMap<String, f64>>>) -> MonitorTimeframe {
    let (regime, overall_score, overall_label) = snap
        .as_ref()
        .and_then(|m| m.context.as_ref())
        .map(|c| (c.regime.clone(), c.overall_score, c.overall_label.clone()))
        .unwrap_or_else(|| ("RANGE".to_string(), 0, "NEUTRAL".to_string()));

    // Bull-bias confluence for display; sign shows net directional pressure.
    // The full RegistryConfluence carries the per-indicator contribution
    // breakdown that drives the score and the opposite-signal exit trigger.
    let empty_w = std::collections::HashMap::new();
    let empty_e = std::collections::HashMap::new();
    let (confluence_score, confluence_normalized, active_weight, regime_gate, contributions) = sv
        .as_ref()
        .map(|s| {
            let c = calculate_registry_confluence("BULLISH", s, &empty_w, &empty_e, regime_multipliers);
            let contribs: Vec<ContributionDto> = c
                .contributions
                .iter()
                .map(|(key, contribution)| ContributionDto {
                    key: key.clone(),
                    display_name: INDICATORS
                        .iter()
                        .find(|m| m.key == key)
                        .map(|m| m.display_name.to_string())
                        .unwrap_or_else(|| key.clone()),
                    contribution: *contribution,
                })
                .collect();
            (c.score, c.normalized, c.active_weight, c.regime_gate, contribs)
        })
        .unwrap_or((0, 0.0, 0.0, 1.0, Vec::new()));

    let (opposite_score_long, opposite_score_short) = sv
        .as_ref()
        .map(|s| {
            (
                calculate_registry_opposite_score("LONG", s, &empty_w, &empty_e, regime_multipliers),
                calculate_registry_opposite_score("SHORT", s, &empty_w, &empty_e, regime_multipliers),
            )
        })
        .unwrap_or((0, 0));

    MonitorTimeframe {
        label: label.to_string(),
        timeframe_secs: secs,
        regime,
        overall_score,
        overall_label,
        confluence_score,
        confluence_normalized,
        active_weight,
        regime_gate,
        contributions,
        opposite_score_long,
        opposite_score_short,
        opposite_exit_threshold: REGISTRY_OPPOSITE_EXIT_THRESHOLD,
    }
}

/// GET /api/monitor?symbol= — cross-timeframe meta-intelligence synthesis for
/// the Terminal Monitor: per-timeframe context + confluence, an MTF confirmation
/// matrix, and the macro market-context summary.
pub async fn serve_monitor(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        default_pair_key(&cfg.symbols.first().cloned().unwrap_or_default())
    } else {
        query.symbol.clone()
    };

    let regime_mult = {
        let cfg = state.config.read().await;
        (!cfg.scoring.regime_weight_multipliers.is_empty())
            .then(|| cfg.scoring.regime_weight_multipliers.clone())
    };
    let regime_mult_ref: Option<&HashMap<String, HashMap<String, f64>>> = regime_mult.as_ref();

    let Some(pair) = get_active_pair(&state.workspace, &pair_key).await else {
        return Json(MonitorResponse {
            symbol: pair_key,
            timeframes: vec![],
            mtf: MtfConfirmation { trend_agreement_pct: 0.0, structural_trend: "NEUTRAL".into(), rows: vec![] },
            market_context: None,
        })
        .into_response();
    };

    let (micro, fast, slow, macro_snap) = pair.latest_snapshots_all_tf().await;
    let (svm, svf, svs, svmac) = (
        snap_values(&micro),
        snap_values(&fast),
        snap_values(&slow),
        snap_values(&macro_snap),
    );

    let timeframes = vec![
        tf_summary("MICRO", pair.micro.timeframe_secs, &micro, &svm, regime_mult_ref),
        tf_summary("FAST", pair.fast.timeframe_secs, &fast, &svf, regime_mult_ref),
        tf_summary("SLOW", pair.slow.timeframe_secs, &slow, &svs, regime_mult_ref),
        tf_summary("MACRO", pair.r#macro.timeframe_secs, &macro_snap, &svmac, regime_mult_ref),
    ];

    // MTF per-indicator agreement matrix (directional registry indicators).
    let empty = SnapshotValues::from_map(Default::default(), 0.0);
    let svs_arr = [
        svm.as_ref().unwrap_or(&empty),
        svf.as_ref().unwrap_or(&empty),
        svs.as_ref().unwrap_or(&empty),
        svmac.as_ref().unwrap_or(&empty),
    ];
    let mut rows: Vec<MtfIndicatorRow> = Vec::new();
    let mut agree_accum = 0.0;
    let mut agree_n = 0.0;
    for meta in INDICATORS {
        if !meta.directional || meta.render == shared::indicators::RenderKind::Marker {
            continue;
        }
        let per_tf: Vec<i8> = svs_arr.iter().map(|sv| dir_bucket(sv, meta.key)).collect();
        let present: Vec<i8> = per_tf.iter().copied().filter(|&d| d != 0).collect();
        if present.is_empty() {
            continue;
        }
        let bulls = present.iter().filter(|&&d| d > 0).count() as f64;
        let bears = present.iter().filter(|&&d| d < 0).count() as f64;
        let dominant = bulls.max(bears);
        let agreement = dominant / present.len() as f64;
        agree_accum += agreement;
        agree_n += 1.0;
        rows.push(MtfIndicatorRow {
            key: meta.key.to_string(),
            display_name: meta.display_name.to_string(),
            per_tf,
            agreement,
        });
    }
    let trend_agreement_pct = if agree_n > 0.0 { (agree_accum / agree_n) * 100.0 } else { 0.0 };

    let mtf_align = evaluate_mtf_alignment(
        svm.as_ref().unwrap_or(&empty),
        svf.as_ref().unwrap_or(&empty),
        svs.as_ref().unwrap_or(&empty),
        svmac.as_ref().unwrap_or(&empty),
    );

    // Macro context preferred; fall back to micro.
    let market_context = macro_snap
        .as_ref()
        .and_then(|m| m.context.clone())
        .or_else(|| micro.as_ref().and_then(|m| m.context.clone()));

    Json(MonitorResponse {
        symbol: pair_key,
        timeframes,
        mtf: MtfConfirmation {
            trend_agreement_pct,
            structural_trend: mtf_align.structural_trend,
            rows,
        },
        market_context,
    })
    .into_response()
}

/// GET /api/monitor/active-trades?symbol= — active position surveillance for
/// the Monitoring panel (IMOL). Consumes paper trading DB data, live snapshot
/// exit signals, and safety manager state.
pub async fn serve_active_trades(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pair_key = if query.symbol.is_empty() {
        let cfg = state.config.read().await;
        default_pair_key(&cfg.symbols.first().cloned().unwrap_or_default())
    } else {
        query.symbol.clone()
    };

    let pair = get_active_pair(&state.workspace, &pair_key).await;
    let current_price = match &pair {
        Some(p) => p.latest_price().await.unwrap_or(0.0),
        None => 0.0,
    };

    let metrics =
        crate::db::paper_get_account_metrics(&state.pool, &pair_key, current_price).await;

    let has_active = metrics.active_position.is_some();
    let direction = metrics.active_position.as_ref().map(|p| p.direction.clone());
    let avg_entry = metrics
        .active_position
        .as_ref()
        .and_then(|p| p.average_entry_price.or(Some(p.entry_price)));
    let total_size = metrics
        .active_position
        .as_ref()
        .map(|p| p.size)
        .unwrap_or(0.0);

    let invalidation: Option<f64> = metrics
        .active_position
        .as_ref()
        .and_then(|p| p.final_invalidation_level);

    let take_profit_targets = &metrics.take_profit_targets;
    let pos_id = metrics.active_position.as_ref().map(|p| p.id);

    let slots: Vec<ActiveTradeDto> = metrics
        .position_slots
        .iter()
        .filter(|s| s.is_active)
        .map(|s| {
            let pnl = if s.direction == "LONG" {
                (current_price - s.entry_price) * s.size
            } else {
                (s.entry_price - current_price) * s.size
            };
            let roi = if s.allocated_usd > 0.0 {
                (pnl / s.allocated_usd) * 100.0
            } else {
                0.0
            };
            let tps: Vec<f64> = take_profit_targets
                .iter()
                .filter(|o| o.associated_position_id == pos_id)
                .filter_map(|o| o.price)
                .collect();
            ActiveTradeDto {
                slot_id: s.id,
                direction: s.direction.clone(),
                entry_price: s.entry_price,
                size: s.size,
                allocated_usd: s.allocated_usd,
                unrealized_pnl: pnl,
                unrealized_pnl_pct: roi,
                stop_loss_price: None,
                take_profit_prices: tps,
            }
        })
        .collect();

    let break_even_trail = BreakEvenTrailDto {
        enabled: metrics.break_even_trail_enabled,
        trail_price: if metrics.break_even_trail_enabled {
            metrics.active_position.as_ref().and_then(|p| p.average_entry_price)
        } else {
            None
        },
    };

    let exit_signals = match &pair {
        Some(p) => {
            let (micro, _, _, _) = p.latest_snapshots_all_tf().await;
            let sv = snap_values(&micro);
            let empty_w = HashMap::new();
            let empty_e = HashMap::new();
            let (ol, os) = sv.as_ref().map(|s| {
                (
                    calculate_registry_opposite_score("LONG", s, &empty_w, &empty_e, None),
                    calculate_registry_opposite_score("SHORT", s, &empty_w, &empty_e, None),
                )
            }).unwrap_or((0, 0));
            ExitSignalsDto {
                opposite_score_long: ol,
                opposite_score_short: os,
                opposite_exit_threshold: REGISTRY_OPPOSITE_EXIT_THRESHOLD,
                invalidation_level: invalidation,
            }
        }
        None => ExitSignalsDto {
            opposite_score_long: 0,
            opposite_score_short: 0,
            opposite_exit_threshold: REGISTRY_OPPOSITE_EXIT_THRESHOLD,
            invalidation_level: invalidation,
        },
    };

    let safety_state = {
        let inst = state.workspace.get_instance_by_pair_key(&pair_key).await;
        let consecutive = inst
            .as_ref()
            .map(|i| i.safety.consecutive_losses.load(Ordering::Relaxed))
            .unwrap_or(0);
        let caution = inst
            .as_ref()
            .map(|i| i.safety.caution_threshold)
            .unwrap_or(3);
        let suspend = inst
            .as_ref()
            .map(|i| i.safety.dropout_threshold)
            .unwrap_or(7);
        SafetyStateDto {
            consecutive_losses: consecutive,
            caution_threshold: caution,
            suspend_threshold: suspend,
        }
    };

    Json(ActiveTradesResponse {
        symbol: pair_key.clone(),
        has_active_position: has_active,
        direction,
        average_entry_price: avg_entry,
        total_size,
        unrealized_pnl: metrics.unrealized_pnl,
        unrealized_roi_pct: metrics.unrealized_roi_pct,
        margin_used: metrics.margin_used,
        account_value: metrics.total_account_value,
        slots,
        break_even_trail,
        exit_signals,
        safety_state,
    })
    .into_response()
}
