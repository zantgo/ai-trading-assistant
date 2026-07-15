use crate::server::types::IndicatorSnapshot;

/// Detects squeeze release: squeeze was active (coiling) in prev and is now
/// released (volatility release label). Returns true only on the transition.
pub fn check_squeeze_release(prev: Option<&IndicatorSnapshot>, curr: &IndicatorSnapshot) -> bool {
    let was_squeezing = prev.and_then(|p| p.squeeze_on()).unwrap_or(false);
    let is_released = curr.squeeze_release_trigger().unwrap_or(false);
    was_squeezing && is_released
}

/// Detects a Support/Resistance role flip via the state label transition.
/// An S/R flip is indicated when the support_resistance label transitions
/// to a FLIP state.
pub fn check_sr_flip(prev: Option<&IndicatorSnapshot>, curr: &IndicatorSnapshot) -> bool {
    let prev_has_flip = prev
        .and_then(|p| p.indicators.get("support_resistance"))
        .map(|v| v.state_label.contains("FLIP"))
        .unwrap_or(false);
    let curr_has_flip = curr
        .indicators
        .get("support_resistance")
        .map(|v| v.state_label.contains("FLIP") && !prev_has_flip)
        .unwrap_or(false);
    curr_has_flip
}

/// Detects an EMA 200 cross: price crosses from one side of the 200 EMA to
/// the other between prev and curr.
pub fn check_ema200_cross(prev: Option<&IndicatorSnapshot>, curr: &IndicatorSnapshot) -> bool {
    let prev_price = prev.and_then(|p| p.current_price);
    let prev_ema = prev.and_then(|p| p.ema_long());
    let curr_price = curr.current_price;
    let curr_ema = curr.ema_long();

    match (prev_price, prev_ema, curr_price, curr_ema) {
        (Some(pp), Some(pe), Some(cp), Some(ce)) => (pp <= pe && cp > ce) || (pp >= pe && cp < ce),
        _ => false,
    }
}

/// Detects confirmed divergence: an RSI or MACD divergence that transitioned
/// from potential to confirmed between prev and curr.
pub fn check_confirmed_divergence(
    prev: Option<&IndicatorSnapshot>,
    curr: &IndicatorSnapshot,
) -> bool {
    let prev_rsi_potential = prev
        .and_then(|p| p.rsi_divergence_status())
        .map(|s| s.starts_with("potential"))
        .unwrap_or(false);
    let curr_rsi_confirmed = curr
        .rsi_divergence_status()
        .map(|s| s.starts_with("confirmed"))
        .unwrap_or(false);

    let prev_macd_potential = prev
        .and_then(|p| p.macd_divergence_status())
        .map(|s| s.starts_with("potential"))
        .unwrap_or(false);
    let curr_macd_confirmed = curr
        .macd_divergence_status()
        .map(|s| s.starts_with("confirmed"))
        .unwrap_or(false);

    (prev_rsi_potential && curr_rsi_confirmed) || (prev_macd_potential && curr_macd_confirmed)
}

/// Evaluate all enabled events against prev and curr snapshots.
/// Returns a Vec of event names that triggered.
pub fn evaluate_trigger_events(
    prev: Option<&IndicatorSnapshot>,
    curr: &IndicatorSnapshot,
    enabled_events: &[String],
) -> Vec<String> {
    let mut triggered = Vec::new();
    for event in enabled_events {
        let fired = match event.as_str() {
            "squeeze_release" => check_squeeze_release(prev, curr),
            "sr_flip" => check_sr_flip(prev, curr),
            "ema200_cross" => check_ema200_cross(prev, curr),
            "confirmed_divergence" => check_confirmed_divergence(prev, curr),
            _ => false,
        };
        if fired {
            triggered.push(event.clone());
        }
    }
    triggered
}
