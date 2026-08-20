use core_domain::portfolio::{CorrelationMap, ExposureMatrix, PositionMatrix};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

pub const MAX_SINGLE_PAIR_EXPOSURE_PCT: f64 = 20.0;
pub const MAX_PORTFOLIO_EXPOSURE_PCT: f64 = 50.0;
pub const MAX_CORRELATION: f64 = 0.8;

pub struct ConcentrationLimits {
    pub max_single_pair_pct: f64,
    pub max_portfolio_pct: f64,
    pub max_correlation: f64,
}

impl Default for ConcentrationLimits {
    fn default() -> Self {
        Self {
            max_single_pair_pct: MAX_SINGLE_PAIR_EXPOSURE_PCT,
            max_portfolio_pct: MAX_PORTFOLIO_EXPOSURE_PCT,
            max_correlation: MAX_CORRELATION,
        }
    }
}

impl ConcentrationLimits {
    /// v7.3: limits from `[workspace.risk_limits]` config — the displayed
    /// cap and the enforced cap are the same number.
    pub fn from_config(cfg: &config_models::RiskLimitsConfig) -> Self {
        Self {
            max_single_pair_pct: cfg.max_single_pair_exposure_pct,
            max_portfolio_pct: cfg.max_portfolio_exposure_pct,
            max_correlation: cfg.max_correlation,
        }
    }
}

fn assign_sector(symbol: &str) -> &'static str {
    let base = symbol.split('-').next().unwrap_or(symbol).to_uppercase();
    match base.as_str() {
        "BTC" | "SOL" | "AVAX" | "ADA" | "DOT" | "ATOM" => "Base Chain",
        "ARB" | "OP" | "MATIC" | "POL" | "IMX" | "STRK" => "L2 Protocols",
        "UNI" | "AAVE" | "MKR" | "COMP" | "CRV" | "LDO" | "PENDLE" => "DeFi",
        "DOGE" | "SHIB" | "PEPE" | "WIF" | "BONK" | "FLOKI" => "Meme",
        "LINK" | "RNDR" | "FET" | "OCEAN" | "AGIX" | "WLD" | "TAO" => "AI / Oracle",
        "ETH" | "BNB" | "SUI" | "APT" | "SEI" | "NEAR" | "FTM" => "L1 Smart Contract",
        _ => "Other",
    }
}

pub fn compute_correlation_matrix(price_histories: &HashMap<String, Vec<f64>>) -> CorrelationMap {
    let mut pairs = HashMap::new();
    let symbols: Vec<&String> = price_histories.keys().collect();
    for i in 0..symbols.len() {
        for j in (i + 1)..symbols.len() {
            let x = &price_histories[symbols[i]];
            let y = &price_histories[symbols[j]];
            if let Some(corr) = pearson_correlation(x, y) {
                let key = format!("{}-{}", symbols[i], symbols[j]);
                pairs.insert(key, corr);
            }
        }
    }
    CorrelationMap { pairs }
}

fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len().min(y.len());
    if n < 10 {
        return None;
    }
    let x_slice = &x[x.len() - n..];
    let y_slice = &y[y.len() - n..];
    let mean_x = x_slice.iter().sum::<f64>() / n as f64;
    let mean_y = y_slice.iter().sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..n {
        let dx = x_slice[i] - mean_x;
        let dy = y_slice[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    if var_x == 0.0 || var_y == 0.0 {
        return None;
    }
    Some(cov / (var_x.sqrt() * var_y.sqrt()))
}

pub fn compute_exposure_matrix(positions: &[PositionMatrix], equity: Decimal) -> ExposureMatrix {
    let mut long_exposure = dec!(0);
    let mut short_exposure = dec!(0);
    let mut symbol_concentration = HashMap::new();
    let mut sector_concentration = HashMap::new();

    for pos in positions {
        let notional = pos.allocated_usd;
        match pos.direction.as_str() {
            "LONG" | "Long" => long_exposure += notional,
            "SHORT" | "Short" => short_exposure += notional,
            _ => {}
        }

        let pct = if equity > dec!(0) {
            (notional / equity) * dec!(100)
        } else {
            dec!(0)
        };
        symbol_concentration.insert(pos.symbol.clone(), pct);

        let sector = assign_sector(&pos.symbol);
        let entry = sector_concentration
            .entry(sector.to_string())
            .or_insert(dec!(0));
        *entry += pct;
    }

    let gross_exposure = long_exposure + short_exposure;
    let net_exposure = long_exposure - short_exposure;

    let net_exposure_pct = if equity > dec!(0) {
        (net_exposure / equity) * dec!(100)
    } else {
        dec!(0)
    };

    let max_single_pair = symbol_concentration
        .values()
        .max()
        .copied()
        .unwrap_or(dec!(0));

    ExposureMatrix {
        gross_exposure,
        net_exposure,
        net_exposure_pct,
        long_exposure,
        short_exposure,
        symbol_concentration,
        sector_concentration,
        max_single_pair_pct: max_single_pair,
        correlation_matrix: CorrelationMap {
            pairs: HashMap::new(),
        },
    }
}

pub fn validate_concentration(
    symbol: &str,
    proposed_size: Decimal,
    equity: Decimal,
    matrix: &ExposureMatrix,
    limits: &ConcentrationLimits,
) -> Result<(), String> {
    let proposed_notional = proposed_size;
    let current_symbol_notional = matrix
        .symbol_concentration
        .get(symbol)
        .map(|pct| {
            if equity > dec!(0) {
                (*pct / dec!(100)) * equity
            } else {
                dec!(0)
            }
        })
        .unwrap_or(dec!(0));

    let new_total = current_symbol_notional + proposed_notional;
    let single_pair_pct = if equity > dec!(0) {
        (new_total / equity * dec!(100)).to_f64().unwrap_or(100.0)
    } else {
        100.0
    };

    if single_pair_pct > limits.max_single_pair_pct {
        return Err(format!(
            "Single-pair concentration {:.1}% exceeds limit {:.1}%",
            single_pair_pct, limits.max_single_pair_pct
        ));
    }

    let new_total_exposure = (matrix.gross_exposure + proposed_notional)
        .to_f64()
        .unwrap_or(0.0);
    let equity_f64 = equity.to_f64().unwrap_or(1.0);
    let portfolio_pct = if equity_f64 > 0.0 {
        (new_total_exposure / equity_f64) * 100.0
    } else {
        100.0
    };

    if portfolio_pct > limits.max_portfolio_pct {
        return Err(format!(
            "Portfolio exposure {:.1}% exceeds limit {:.1}%",
            portfolio_pct, limits.max_portfolio_pct
        ));
    }

    Ok(())
}
