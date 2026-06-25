use super::{Exchange, NormalizedEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct SymbolMapper {
    to_normalized: RwLock<HashMap<(Exchange, String), String>>,
    to_raw: RwLock<HashMap<(Exchange, String), String>>,
}

impl SymbolMapper {
    pub fn new() -> Self {
        Self {
            to_normalized: RwLock::new(HashMap::new()),
            to_raw: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, exchange: Exchange, raw: &str, normalized: &str) {
        let mut to_norm = self.to_normalized.write().await;
        let mut to_r = self.to_raw.write().await;
        to_norm.insert((exchange, raw.to_string()), normalized.to_string());
        to_r.insert((exchange, normalized.to_string()), raw.to_string());
    }

    pub async fn normalize(&self, exchange: Exchange, raw: &str) -> Option<String> {
        let to_norm = self.to_normalized.read().await;
        to_norm.get(&(exchange, raw.to_string())).cloned()
    }

    pub async fn get_raw(&self, exchange: Exchange, normalized: &str) -> Option<String> {
        let to_r = self.to_raw.read().await;
        to_r.get(&(exchange, normalized.to_string())).cloned()
    }

    pub async fn get_normalized_for_exchange(&self, exchange: Exchange) -> Vec<String> {
        let to_norm = self.to_normalized.read().await;
        let mut result = Vec::new();
        for ((ex, _raw), normalized) in to_norm.iter() {
            if *ex == exchange {
                result.push(normalized.clone());
            }
        }
        result
    }

    pub async fn load_default_mappings(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_normalize() {
        let mapper = SymbolMapper::new();
        mapper
            .register(Exchange::Hyperliquid, "BTCUSDT", "BTC-USD")
            .await;
        assert_eq!(
            mapper.normalize(Exchange::Hyperliquid, "BTCUSDT").await,
            Some("BTC-USD".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_raw_reverse_mapping() {
        let mapper = SymbolMapper::new();
        mapper
            .register(Exchange::Hyperliquid, "BTC-USD", "BTC-USD")
            .await;
        assert_eq!(
            mapper.get_raw(Exchange::Hyperliquid, "BTC-USD").await,
            Some("BTC-USD".to_string())
        );
    }

    #[tokio::test]
    async fn test_unknown_mapping_returns_none() {
        let mapper = SymbolMapper::new();
        assert_eq!(
            mapper.normalize(Exchange::Hyperliquid, "UNKNOWN").await,
            None
        );
    }

    #[tokio::test]
    async fn test_case_sensitive_keys() {
        let mapper = SymbolMapper::new();
        mapper
            .register(Exchange::Hyperliquid, "BTCUSDT", "BTC-USD")
            .await;
        assert_eq!(
            mapper.normalize(Exchange::Hyperliquid, "btcusdt").await,
            None,
            "SymbolMapper keys are case-sensitive; lowercase should not match"
        );
    }

    #[tokio::test]
    async fn test_dynamic_mappings_registration() {
        let mapper = SymbolMapper::new();

        mapper
            .register(Exchange::Hyperliquid, "BTC", "BTC-USD")
            .await;

        assert_eq!(
            mapper.normalize(Exchange::Hyperliquid, "BTC").await,
            Some("BTC-USD".to_string())
        );
    }
}
