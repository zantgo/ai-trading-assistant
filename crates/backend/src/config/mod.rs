pub mod models;
pub use models::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub symbols: Vec<String>,
    pub candles: CandlesConfig,
    pub indicators: IndicatorsConfig,
    #[serde(default)]
    pub hyperliquid: HyperliquidConfig,
    #[serde(default)]
    pub bitget: BitgetConfig,
    #[serde(default)]
    pub fibonacci: FibonacciConfig,
    #[serde(default)]
    pub pivots: PivotsConfig,
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
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub safety: SafetyConfig,
    #[serde(default)]
    pub intervals: IntervalsConfig,
    #[serde(default, skip_serializing)]
    pub workspaces: HashMap<String, WorkspaceSpecificConfig>,
}

pub fn load_config() -> AppConfig {
    let config_raw = std::fs::read_to_string("config.toml")
        .expect("\u{274c} Configuration Error: Failed to find \"config.toml\" in workspace root directory");

    toml::from_str(&config_raw)
        .expect("\u{274c} Configuration Error: Failed to parse fields inside config.toml")
}

pub fn load_workspaces() -> HashMap<String, WorkspaceSpecificConfig> {
    if let Ok(raw) = std::fs::read_to_string("workspaces.json") {
        return serde_json::from_str(&raw).unwrap_or_default();
    }

    if let Ok(raw) = std::fs::read_to_string("pairs.json") {
        let old: HashMap<String, WorkspaceSpecificConfig> = serde_json::from_str(&raw).unwrap_or_default();
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
            let _ = std::fs::write("workspaces.json", json_str);
        }
        println!("\u{1f4e6} Migrated pairs.json -> workspaces.json with new key format");
        return migrated;
    }

    HashMap::new()
}

pub async fn save_workspaces(instances: &HashMap<String, WorkspaceSpecificConfig>) {
    match serde_json::to_string_pretty(instances) {
        Ok(json_str) => {
            if let Err(e) = tokio::fs::write("workspaces.json", json_str).await {
                eprintln!("\u{274c} Config Error: Failed to write workspaces.json: {}", e);
            }
        }
        Err(e) => {
            eprintln!("\u{274c} JSON Serialization Error for instances: {}", e);
        }
    }
}
