use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandlesConfig {
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorsConfig {
    pub ema_fast: usize,
    pub ema_medium: usize,
    pub ema_slow: usize,
    pub ema_long: usize,
    pub rsi_period: usize,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub adx_period: usize,
    pub atr_period: usize,
    pub squeeze_period: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairSpecificConfig {
    pub candles: CandlesConfig,
    pub indicators: IndicatorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub symbols: Vec<String>,
    pub candles: CandlesConfig,
    pub indicators: IndicatorsConfig,
    #[serde(default, skip_serializing)]
    pub pairs: HashMap<String, PairSpecificConfig>,
}

pub fn load_config() -> AppConfig {
    let config_raw = std::fs::read_to_string("config.toml")
        .expect("❌ Configuration Error: Failed to find \"config.toml\" in workspace root directory");

    toml::from_str(&config_raw)
        .expect("❌ Configuration Error: Failed to parse fields inside config.toml")
}

pub fn load_pairs() -> HashMap<String, PairSpecificConfig> {
    match std::fs::read_to_string("pairs.json") {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

pub fn save_pairs(pairs: &HashMap<String, PairSpecificConfig>) {
    match serde_json::to_string_pretty(pairs) {
        Ok(json_str) => {
            if let Err(e) = std::fs::write("pairs.json", json_str) {
                eprintln!("❌ Config Error: Failed to write pairs.json: {}", e);
            }
        }
        Err(e) => {
            eprintln!("❌ JSON Serialization Error for pairs: {}", e);
        }
    }
}
