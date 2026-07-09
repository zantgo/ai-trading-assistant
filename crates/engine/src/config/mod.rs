pub mod models;
pub use models::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use shared::statistics::StatisticsConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub symbols: Vec<String>,
    pub candles: CandlesConfig,
    pub indicators: IndicatorsConfig,
    #[serde(default)]
    pub hyperliquid: HyperliquidConfig,
    #[serde(default)]
    pub fibonacci: FibonacciConfig,
    #[serde(default)]
    pub pivots: PivotsConfig,
    #[serde(default)]
    pub pivot_points: PivotPointsConfig,
    #[serde(default)]
    pub candlestick: CandlestickConfig,
    #[serde(default)]
    pub slow_timeframe: SlowTimeframeConfig,
    #[serde(default)]
    pub macro_timeframe: SlowTimeframeConfig,
    #[serde(default)]
    pub leverage: LeverageConfig,
    #[serde(default)]
    pub scoring: ScoringConfig,
    #[serde(default)]
    pub fees: FeesConfig,
    #[serde(default)]
    pub costs: CostsConfig,
    #[serde(default)]
    pub orderbook: OrderBookConfig,
    #[serde(default)]
    pub execution: ExecutionConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub safety: SafetyConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub portfolio: PortfolioConfig,
    #[serde(default)]
    pub intervals: IntervalsConfig,
    #[serde(default)]
    pub api_failover: ApiFailoverConfig,
    #[serde(default)]
    pub backtest: crate::backtest::engine::BacktestConfig,
    #[serde(default)]
    pub profile: ProfileConfig,
    #[serde(default)]
    pub statistics: StatisticsConfig,
    #[serde(default, skip_serializing)]
    pub instances: HashMap<String, InstanceSpecificConfig>,
}

pub fn load_config() -> AppConfig {
    let config_raw = std::fs::read_to_string("config.toml")
        .expect("\u{274c} Configuration Error: Failed to find \"config.toml\" in workspace root directory");

    toml::from_str(&config_raw)
        .expect("\u{274c} Configuration Error: Failed to parse fields inside config.toml")
}

pub fn load_instances() -> HashMap<String, InstanceSpecificConfig> {
    if let Ok(raw) = std::fs::read_to_string("instances.json") {
        return serde_json::from_str(&raw).unwrap_or_default();
    }

    if let Ok(raw) = std::fs::read_to_string("pairs.json") {
        let old: HashMap<String, InstanceSpecificConfig> = serde_json::from_str(&raw).unwrap_or_default();
        let mut migrated = HashMap::new();
        for (key, value) in old {
            let new_key = if let Some((_exchange, base)) = key.split_once('-') {
                format!("{}-USDT", base)
            } else {
                key
            };
            migrated.insert(new_key, value);
        }
        if let Ok(json_str) = serde_json::to_string_pretty(&migrated) {
            let _ = std::fs::write("instances.json", json_str);
        }
        println!("\u{1f4e6} Migrated pairs.json -> instances.json with new key format");
        return migrated;
    }

    HashMap::new()
}

pub async fn save_instances(instances: &HashMap<String, InstanceSpecificConfig>) {
    match serde_json::to_string_pretty(instances) {
        Ok(json_str) => {
            if let Err(e) = tokio::fs::write("instances.json", json_str).await {
                eprintln!("\u{274c} Config Error: Failed to write instances.json: {}", e);
            }
        }
        Err(e) => {
            eprintln!("\u{274c} JSON Serialization Error for instances: {}", e);
        }
    }
}
